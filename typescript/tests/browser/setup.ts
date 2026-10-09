// Runs before every browser test file (`setupFiles` in `vitest.config.ts`): each test starts with
// none of the work the app keeps across reloads (`src/lib/persist.ts`), so no test restores what an
// earlier one left.
import { beforeEach } from 'vitest';

beforeEach(async () => {
  sessionStorage.clear();
  // Waits until the database is really gone. A connection still open from the previous test (the
  // app opens one only for a moment) delays the deletion until it closes.
  await new Promise<void>((resolve) => {
    const request = indexedDB.deleteDatabase('rekolor');
    request.onsuccess = request.onerror = () => resolve();
  });
});
