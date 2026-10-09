# The README screenshot

`screenshot.png` at the project root is the picture at the top of `README.md`. It shows the web app
with an image from this folder open and that image's palette config imported.

## Which image

The script takes the first image in this folder that has a palette config of the same name:

- the image: `<name>.jpg` or `<name>.png`;
- its config: `<name>.palettes.toml`, as the app's "Export picks" saves it.

"First" means the first by file name, in character-code order: digits, then capital letters, then
lowercase letters. Images without a config are skipped. If the config has several palettes, the
script imports the first one the app lists. Files in this folder other than the image and its config
are ignored.

## Recreate it

From the project root, once the setup below is done:

```sh
node publish/screenshot.mjs
```

The script builds the web app (`npm run build` in `typescript/`, which rewrites `typescript/dist/`),
serves the build at `http://127.0.0.1:4180/`, and opens it in Chromium. There it opens the image,
imports the config, waits until the preview is drawn, and overwrites `screenshot.png`.

It prints which image and config it used, the app's status line and its file line. It exits non-zero
if any step fails:
- no image in this folder has a config;
- the app reports an error, such as a config it can't read (the script prints the app's message);
- a step times out.

### Setup

Redo steps 1 and 2 whenever the code they build has changed.

1. The WASM package the web app loads, in `rust/`:
   `wasm-pack build crates/wasm --release --target web`.
2. The web app's dependencies, in `typescript/`: `npm ci`.
3. Chromium for Playwright, in `typescript/`: `npx playwright install chromium`.

Port 4180 must be free.

## Check the result

1. The `image:` line names the pair you expected, and the `status:` line starts with "Imported".
2. Look at `screenshot.png`. You should see:
   - the original on the left and the printed preview on the right;
   - the picks below them;
   - the Material panel on the right.

   It is 1280 pixels wide. Its height is whatever the page needs.
3. If the app and the pair haven't changed, the file comes out byte for byte the same on the same
   machine. After app changes, differences are expected; recording them is the point.
4. If the picture no longer matches the image's alt text in `README.md`, update the alt text.

## Keep as is: ask the owner first

- **The image and its config.** What the screenshot shows was chosen by the owner. Don't add,
  rename or replace images or configs in this folder; a new pair whose name sorts earlier would
  become the screenshot. Don't edit the config either. The image's file name appears in the app.
- **The capture settings:** Chromium, a 1280 × 900 CSS-pixel window, device scale 1, light theme,
  the whole page.

## If the script fails after the app changed

The script follows the app's own inputs and texts, defined in `typescript/src/App.svelte`:

| What | Used for |
|---|---|
| `#file` | the hidden file input behind "Open image…" |
| `input[accept^=".toml"]` | the hidden file input behind "Import picks…" |
| "Engine ready" | the status line once the WASM engine has loaded |
| "No picks yet: nothing is printed" | the preview's heading once the image is open (and after importing a palette with no picks) |
| "Imported …" | the status line once the config is imported |
| the "Which palette?" dialog | shown when the config has several palettes; the script clicks its first button |
| `Printed with N inks`, optionally followed by `· N color(s) left to the material` | the preview's heading once the import is drawn; while a recolor runs, it ends in "· updating…" |
| the `alert` element | an error message from the app |

If one of these changed, update the script to match the app. Keep the steps and the settings the
same.

## Where it was made

On macOS (Apple silicon), with the Chromium that Playwright 1.64 installs. Other systems render
fonts differently, so a capture made elsewhere will differ slightly in its text.
