// Browser commands for a held mouse button (Vitest's `userEvent` only clicks and drags whole
// gestures). They run in Node with Playwright; tests call them through `commands` from
// 'vitest/browser'. Keys held with `userEvent.keyboard('{Shift>}')` apply to these moves too.

import type { BrowserCommand } from 'vitest/node';

// Each command receives a context (`ctx`) with Playwright's page, followed by the arguments the
// test passed. `BrowserCommand<[...]>` lists those arguments' types.
/** Moves the mouse to (x, y) CSS pixels from the top-left of the element `selector` matches. */
const mouseTo: BrowserCommand<[selector: string, x: number, y: number]> = async (
  ctx,
  selector,
  x,
  y,
) => {
  const frame = await ctx.frame();
  await frame.locator(selector).hover({ position: { x, y }, force: true });
};

const mouseDown: BrowserCommand<[]> = async (ctx) => {
  await ctx.page.mouse.down();
};

const mouseUp: BrowserCommand<[]> = async (ctx) => {
  await ctx.page.mouse.up();
};

export const mouseCommands = { mouseTo, mouseDown, mouseUp };

// Adds the commands to Vitest's own `BrowserCommands` type ("declaration merging"), so a test's
// `commands.mouseTo(...)` call is type-checked.
declare module 'vitest/browser' {
  interface BrowserCommands {
    mouseTo: (selector: string, x: number, y: number) => Promise<void>;
    mouseDown: () => Promise<void>;
    mouseUp: () => Promise<void>;
  }
}
