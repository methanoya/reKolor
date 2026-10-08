// Access to the cross-decoder fixtures recorded by
// `cargo run --release -p rekolor-io --example update_decoder_fixtures`.

import referencesTsv from '../../../rust/testdata/decoders/references.tsv?raw';

const urls = import.meta.glob('../../../rust/testdata/decoders/*.{png,jpg,rgba}', {
  query: '?url',
  import: 'default',
  eager: true,
}) as Record<string, string>;

export interface Reference {
  name: string;
  file: string;
  width: number;
  height: number;
  warnings: string;
}

export const references: Reference[] = referencesTsv
  .split('\n')
  .filter((l) => l && !l.startsWith('#'))
  .map((line) => {
    const [name = '', file = '', width = '0', height = '0', warnings = ''] = line.split('\t');
    return { name, file, width: Number(width), height: Number(height), warnings };
  });

export const reference = (name: string): Reference => {
  const found = references.find((r) => r.name === name);
  if (!found) throw new Error(`no fixture ${name}`);
  return found;
};

async function fetchBytes(file: string): Promise<ArrayBuffer> {
  const url = urls[`../../../rust/testdata/decoders/${file}`];
  if (!url) throw new Error(`no fixture file ${file}`);
  return (await fetch(url)).arrayBuffer();
}

/** The encoded fixture, as a File (what the app gets from the file chooser). */
export async function fixtureFile(name: string): Promise<File> {
  const { file } = reference(name);
  return new File([await fetchBytes(file)], file);
}

/** How `rekolor-io` decoded it: straight-alpha RGBA. */
export async function rustPixels(name: string): Promise<Uint8Array> {
  return new Uint8Array(await fetchBytes(`${name}.rgba`));
}
