import type { RgbColor8 } from './typeshare';
export type { RgbColor8 } from './typeshare';
export interface RgbaColor8 {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface PaletteEntry {
  rgb: [number, number, number];
}

export interface ProcessingImageType {
  width: number;
  height: number;
  content: string;
}

export interface PickingStripeType {
  picked: RgbaColor8;
  original: RgbColor8;
  nominal?: RgbColor8;
  palette?: string;
  nominalName?: string;
}
