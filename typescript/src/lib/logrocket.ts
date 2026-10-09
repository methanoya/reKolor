// Session recording with LogRocket (https://logrocket.com), on the published site only. What is
// recorded, and why, is explained to visitors on the privacy page (`public/privacy.html`, linked
// from the header); keep the two in step.
//
// There, LogRocket records each visit for the project `gzqrb0/rekolor`: the page's layout and
// text, clicks and other input events, console output, uncaught errors, performance data, and the
// browser's details (type, system, screen, language, referring page), plus the named events and
// the error kinds the app reports below. Not recorded: IP addresses (`shouldCaptureIP: false`),
// network requests (`network.isEnabled: false`), and any element marked `data-private`: the image
// views (the user's image and its preview) and every element that can show a file name (the file
// details, the status and error lines, the palette dialog, the file inputs). LogRocket never sends
// a private element's content.
//
// Everywhere else (local development, the tests, CI's smoke test) LogRocket isn't even loaded and
// nothing is sent: every function here does nothing.
//
// The rest of the app calls only these functions, never LogRocket itself, so the rule "published
// site only" lives in one place. LogRocket runs in the page, not in the engine's worker; errors
// from the worker reach the page as the error kinds the app reports (`reportError`).

// Only the type here: a normal `import` would load LogRocket on every page, and loading it already
// contacts LogRocket's servers (it fetches its recorder script), before any `init`. The code is
// loaded with `import()` below, on the published site only, as a separate file of the build.
import type LogRocketApi from 'logrocket';

const APP_ID = 'gzqrb0/rekolor';
/** The only host that records: the published site (GitHub Pages). */
const PUBLISHED_HOST = 'methanoya.github.io';

type Call = (logRocket: typeof LogRocketApi) => void;
/** LogRocket, once loaded and started. */
let logRocket: typeof LogRocketApi | undefined;
/** Calls made while LogRocket loads, sent once it has started; `undefined`: not recording. */
let queued: Call[] | undefined;

/**
 * Starts recording if this page is the published site. Called once, before the app mounts
 * (`main.ts`): LogRocket misses what happens before it starts.
 */
export function startRecording(): void {
  if (queued || logRocket || location.hostname !== PUBLISHED_HOST) return;
  queued = [];
  // `import()` loads a module when it is called and returns a promise of it; `default` is what the
  // module exports.
  import('logrocket')
    .then(({ default: loaded }) => {
      loaded.init(APP_ID, {
        // The release names the code that was running in each recording: CI sets `VITE_RELEASE`
        // to the commit it builds (Vite passes `VITE_` variables to the app as
        // `import.meta.env`). A build made elsewhere says "local".
        release: import.meta.env.VITE_RELEASE ?? 'local',
        // As little as the purpose needs: no IP addresses, and no network requests (the app makes
        // none of its own besides loading itself).
        shouldCaptureIP: false,
        network: { isEnabled: false },
      });
      logRocket = loaded;
      for (const call of queued ?? []) call(loaded);
      queued = undefined;
    })
    .catch(() => {
      // The module couldn't be loaded (a lost connection, say): the app works the same without it.
      queued = undefined;
    });
}

// Sends a call now if LogRocket is running, queues it while LogRocket loads, and drops it if this
// page doesn't record.
function send(call: Call): void {
  if (logRocket) call(logRocket);
  else queued?.push(call);
}

/** Values an event may carry: LogRocket accepts strings, numbers and booleans. */
export type EventProperties = Record<string, string | number | boolean>;

// The image formats an event may name, keyed by MIME subtype (`image/png` → `png`) or file
// extension, each mapped to one name. A lookup in this fixed table is the only way a format gets
// into an event: an unknown type or extension becomes `other`, so no part of a file's name (such as
// `patient-id` in `scan.patient-id`) can reach LogRocket.
const FORMATS: Readonly<Record<string, string>> = {
  png: 'png',
  apng: 'png',
  jpeg: 'jpeg',
  jpg: 'jpeg',
  jpe: 'jpeg',
  jfif: 'jpeg',
  pjpeg: 'jpeg',
  gif: 'gif',
  webp: 'webp',
  avif: 'avif',
  bmp: 'bmp',
  'x-ms-bmp': 'bmp',
  ico: 'ico',
  'x-icon': 'ico',
  'vnd.microsoft.icon': 'ico',
  tif: 'tiff',
  tiff: 'tiff',
  svg: 'svg',
  'svg+xml': 'svg',
  heic: 'heic',
  heif: 'heif',
  jxl: 'jxl',
  exr: 'exr',
  hdr: 'hdr',
  qoi: 'qoi',
  tga: 'tga',
  dds: 'dds',
  pbm: 'pnm',
  pgm: 'pnm',
  ppm: 'pnm',
  pnm: 'pnm',
  pam: 'pnm',
  ff: 'farbfeld',
};

/**
 * The image format to report for a file: from its type (`image/…`), or, when the browser gives
 * none, its extension; always one of the names in `FORMATS`, or `other`.
 */
export function imageFormat(file: { name: string; type: string }): string {
  const subtype = /^image\/(.+)$/.exec(file.type)?.[1];
  const extension = /\.([^.]+)$/.exec(file.name)?.[1];
  // `Object.hasOwn` keeps names like `constructor` (inherited by every object) from matching.
  const key = (subtype ?? extension ?? '').toLowerCase();
  return Object.hasOwn(FORMATS, key) ? FORMATS[key]! : 'other';
}

/** Records a named user action (searchable in LogRocket as a custom event). */
export function track(event: string, properties?: EventProperties): void {
  send((lr) => lr.track(event, properties));
}

/**
 * Reports an error the app showed, by its kind only (`decodeFailed`, `invalidConfig`, …): the
 * message itself can name the user's files. Listed as an error in LogRocket's Issues, tagged with
 * the kind.
 */
export function reportError(kind: string): void {
  send((lr) => lr.captureMessage(`reKolor error: ${kind}`, { tags: { kind } }));
}
