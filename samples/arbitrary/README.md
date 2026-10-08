# reKolor test images

Curated for the current reKolor workflow: load a JPEG or PNG, click source
colors, match those colors to Pantone, then remap every pixel to the nearest
selected Pantone color with CIEDE2000.

All images are public domain or CC0. The local photo copies are resized to keep
interactive browser tests reasonably fast.

| File | Size | Best used for | Source and license |
| --- | ---: | --- | --- |
| `01-color-chart.png` | 960 x 646 | Primary correctness fixture: flat color patches, skin-like patches, and neutrals | [ColorChart.svg](https://commons.wikimedia.org/wiki/File:ColorChart.svg), CC0 |
| `02-rgb-6level-chart.png` | 258 x 200 | Broad RGB coverage and many exact repeated pixel values | [RGB 6levels palette color test chart.png](https://commons.wikimedia.org/wiki/File:RGB_6levels_palette_color_test_chart.png), public domain |
| `03-gradient.png` | 300 x 200 | Quantization boundaries, banding, and behavior as the chosen palette grows | [SVG Gradient Test librsvg.png](https://commons.wikimedia.org/wiki/File:SVG_Gradient_Test_librsvg.png), CC0 |
| `04-hot-peppers.jpg` | 1280 x 842 | Saturated natural colors, texture, highlights, shadows, and JPEG noise | [Multi colored hot peppers.jpg](https://commons.wikimedia.org/wiki/File:Multi_colored_hot_peppers.jpg), public domain |
| `05-mae-jemison.jpg` | 960 x 1200 | Skin tones, deep shadows, white fabric, blue background, and fine detail | [Mae Carol Jemison.jpg](https://commons.wikimedia.org/wiki/File:Mae_Carol_Jemison.jpg), NASA/public domain |
| `06-neon-street.jpg` | 1000 x 750 | Near-black regions and highly saturated light that is difficult to reproduce in print | [Neon lights on street.jpg](https://commons.wikimedia.org/wiki/File:Neon_lights_on_street.jpg), public domain |
| `07-alpha-hue.png` | 960 x 960 | Alpha-channel compositing and hidden RGB values in transparent pixels | [Hue-alpha.png](https://commons.wikimedia.org/wiki/File:Hue-alpha.png), public domain |
| `08-antialiased-red-type.png` | 572 x 409 | Anti-aliased typography and thin curved edges; despite its source title, this copy is RGB without an alpha channel | [1967 (transparent red, Mojo).png](https://commons.wikimedia.org/wiki/File:1967_(transparent_red,_Mojo).png), public domain |

## Suggested test order

1. Start with `01-color-chart.png`. Pick 3, then 6, then 12 well-separated
   swatches. Confirm that the result uses no more colors than the number picked,
   and that repeated runs with the same picks are identical.
2. Use `03-gradient.png` with 2, 4, and 8 picks. Check whether the band
   boundaries are perceptually sensible and whether adding colors improves the
   result monotonically.
3. Use both photographs with a small print-like palette: black/dark, white/light,
   one warm color, one cool color, and one accent. Look for hue reversals,
   crushed facial detail, and unstable handling of near-black pixels.
4. Use `02-rgb-6level-chart.png` for broad coverage and performance. It is also
   useful for checking that clicking the same source RGB value twice does not add
   duplicate choices.
5. Use `07-alpha-hue.png` as an edge-case test. The current implementation
   composites RGBA pixels against the material color (white by default) before
   matching and emits an opaque PNG, so transparency loss should be recorded
   explicitly as current behavior or as a bug, depending on product intent.
6. Use the red type image at 100% zoom. Check thin strokes and curved edges for
   halos or broken contours after palette reduction.

## Useful pass/fail checks

- JPEG and PNG inputs load without errors and report the correct dimensions.
- Processing the same image, in the same click order, produces byte-identical
  PNG output.
- The output contains only selected nominal Pantone colors.
- Exact source-color hits map to the Pantone color shown in the picker.
- No unselected color appears in the processed image.
- Empty selection preserves the original preview.
- Processing time grows acceptably from the 258 x 200 chart to the 1280 x 842
  photograph.
