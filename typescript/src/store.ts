import { Writable, writable } from 'svelte/store';
import type { PickingStripeType } from './types';

export const pickingStripes: Writable<Array<PickingStripeType>> = writable([]);
