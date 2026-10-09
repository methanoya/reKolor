// Types for the build-time values Vite gives the app as `import.meta.env` (the `VITE_` environment
// variables). A `.d.ts` file only declares types; it contains no code.

interface ImportMetaEnv {
  /** The commit being built, set by CI (see `src/lib/logrocket.ts`); unset in local builds. */
  readonly VITE_RELEASE?: string;
}
