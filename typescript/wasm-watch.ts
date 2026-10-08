// Dev server only: rebuilds the WASM package when the Rust it is made from changes, then reloads the
// page, so the worker loads the new module. Off under Vitest (which reuses the Vite config) and with
// REKOLOR_WASM_WATCH=0.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin, ViteDevServer } from 'vite';

const rust = fileURLToPath(new URL('../rust', import.meta.url));
const wasmCrate = path.join(rust, 'crates/wasm');
const pkg = path.join(wasmCrate, 'pkg');
/** The crates the package is built from (`rekolor-wasm` and its workspace dependencies). */
const crates = ['core', 'config', 'wasm'].map((name) => path.join(rust, 'crates', name));
const sourceDirs = crates.map((crate) => path.join(crate, 'src'));
const manifests = [
  ...crates.map((crate) => path.join(crate, 'Cargo.toml')),
  path.join(rust, 'Cargo.toml'),
  path.join(rust, 'Cargo.lock'),
];

/** Changes closer together than this make one build. */
const DEBOUNCE_MS = 200;

// Builds take a lock, so two dev servers on the same checkout build one after the other instead of
// writing `pkg/` at the same time (which made one of them fail). A lock older than STALE_LOCK_MS is
// left over from a killed build.
const lockFile = path.join(rust, 'target', '.rekolor-wasm-watch.lock');
const LOCK_RETRY_MS = 250;
const STALE_LOCK_MS = 120_000;

function tryLock(): boolean {
  try {
    fs.mkdirSync(path.dirname(lockFile), { recursive: true });
    fs.writeFileSync(lockFile, String(process.pid), { flag: 'wx' });
    return true;
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code !== 'EEXIST') throw e;
    try {
      if (Date.now() - fs.statSync(lockFile).mtimeMs > STALE_LOCK_MS) fs.rmSync(lockFile);
    } catch {
      // Removed meanwhile by its owner: the next try takes it.
    }
    return false;
  }
}

const unlock = () => fs.rmSync(lockFile, { force: true });

const isRustInput = (file: string) =>
  manifests.includes(file) ||
  (file.endsWith('.rs') && sourceDirs.some((dir) => file.startsWith(dir + path.sep)));

export function wasmWatch(): Plugin | false {
  if (process.env.VITEST || process.env.REKOLOR_WASM_WATCH === '0') return false;
  return {
    name: 'rekolor-wasm-watch',
    apply: 'serve',
    config: () => ({
      // wasm-pack replaces the package file by file; reloading on the first one would load a
      // half-built package. The page is reloaded once the build is done (below).
      server: { watch: { ignored: (file: string) => file.startsWith(pkg) } },
    }),
    configureServer(server) {
      const log = server.config.logger;
      let timer: ReturnType<typeof setTimeout> | undefined;
      let running = false;
      let again = false;

      const build = () => {
        if (running) {
          again = true; // one more build when this one is done
          return;
        }
        if (!tryLock()) {
          // Another dev server is building: try again shortly (a new change resets the wait).
          clearTimeout(timer);
          timer = setTimeout(build, LOCK_RETRY_MS);
          return;
        }
        running = true;
        const started = performance.now();
        log.info('[rekolor] Rust changed: rebuilding the WASM package…', { timestamp: true });
        const child = spawn('wasm-pack', ['build', wasmCrate, '--release', '--target', 'web'], {
          stdio: ['ignore', 'pipe', 'pipe'],
        });
        let output = '';
        const collect = (chunk: Buffer) => (output += chunk.toString());
        child.stdout.on('data', collect);
        child.stderr.on('data', collect);
        let finished = false;
        const finish = (ok: boolean, why: string) => {
          if (finished) return;
          finished = true;
          running = false;
          unlock();
          // Even a failed build may have rewritten part of the package (e.g. wasm-bindgen ran, then
          // wasm-opt failed): drop the cached modules either way, so a manual reload never pairs
          // old JavaScript with a new `.wasm`.
          invalidate(server);
          if (ok) {
            const seconds = ((performance.now() - started) / 1000).toFixed(1);
            log.info(`[rekolor] WASM package rebuilt in ${seconds} s; reloading the page`, {
              timestamp: true,
            });
            server.ws.send({ type: 'full-reload' });
          } else {
            log.error(`[rekolor] WASM build failed (${why}):\n${output}`, { timestamp: true });
            server.ws.send({
              type: 'error',
              err: {
                message: `The WASM package failed to build (${why}); the page wasn't reloaded.`,
                stack: output,
                plugin: 'rekolor-wasm-watch',
              },
            });
          }
          if (again) {
            again = false;
            build();
          }
        };
        child.on('error', (e) => finish(false, `couldn't run wasm-pack: ${e.message}`));
        child.on('close', (code) => finish(code === 0, `wasm-pack exited with ${code}`));
      };

      server.watcher.add([...sourceDirs, ...manifests]);
      server.watcher.on('all', (_event, file) => {
        if (!isRustInput(file)) return;
        clearTimeout(timer);
        timer = setTimeout(build, DEBOUNCE_MS);
      });
    },
  };
}

/** The package's modules are cached but not watched: drop every cached module. */
function invalidate(server: ViteDevServer) {
  for (const environment of Object.values(server.environments)) {
    environment.moduleGraph.invalidateAll();
  }
}
