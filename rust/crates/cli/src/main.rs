//! The reKolor command-line tool.

// The program's entry point: argument parsing and the four commands. Run `rekolor --help` (or
// `cargo run -p rekolor-cli -- --help`) for the generated help text.
//
// `use rekolor_cli::...` imports this crate's own library half (`lib.rs`).
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use rekolor_cli::config::{
    self, ConfigPick, DEFAULT_UNPRINTED_DELTA_E, PaletteConfig, SizedPalette, Unprinted,
};
use rekolor_cli::{discover, generate, palette_file};
use rekolor_core::{Mapping, MaterialRange, Palette, Rgb8, analyze, recolor_with_ranges};
use rekolor_io::DecodedImage;

// `[u32; 3]`: a fixed array of three numbers.
/// Palette sizes of the golden set.
const GOLDEN_SIZES: [u32; 3] = [3, 7, 16];

// clap builds the whole command-line parser from these annotated types: struct fields become
// options, `///` doc comments become the help text, and `#[arg(...)]`/`#[command(...)]` tweak
// names and rules. A field of type `Option<T>` is an optional flag; `Vec<T>` can be repeated.
// `global = true` lets `--palette` appear after any subcommand.
#[derive(Parser)]
#[command(name = "rekolor", version, about = "Limited-palette print previews")]
struct Cli {
    /// Palette file (JSON). Default: palettes/pantone.json in the current or a parent directory.
    #[arg(long, global = true, value_name = "PATH")]
    palette: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

// The subcommands: `rekolor recolor`, `rekolor analyze`, `rekolor palettes generate`,
// `rekolor golden update`. An enum variant can carry fields, like a struct.
#[derive(Subcommand)]
enum Command {
    /// Recolor an image with picks from a palette config or given on the command line.
    Recolor(RecolorArgs),
    /// Print an image's size, color counts and decoder warnings.
    Analyze {
        input: PathBuf,
        /// Material color to count the colors on (default: white).
        #[arg(long, value_name = "R,G,B", value_parser = parse_rgb)]
        material: Option<Rgb8>,
    },
    /// Palette configs (<name>.palettes.toml) for a samples tree.
    #[command(subcommand)]
    Palettes(PalettesCommand),
    /// Golden outputs (<name>-out-<size>.png) for a samples tree.
    #[command(subcommand)]
    Golden(GoldenCommand),
}

// `requires`, `conflicts_with` and `required_unless_present` let clap reject invalid combinations
// before the program runs. `value_parser = parse_rgb` converts the text with the function below.
#[derive(Args)]
struct RecolorArgs {
    input: PathBuf,
    /// Output PNG.
    #[arg(short, long)]
    output: PathBuf,
    /// Palette config to take the picks from (needs --size).
    #[arg(long, requires = "size", conflicts_with = "pick")]
    config: Option<PathBuf>,
    /// Which palette of the config to use.
    #[arg(long, requires = "config")]
    size: Option<u32>,
    /// A pick: "R,G,B=INK NAME", e.g. "255,184,0=Pantone 1235". Repeatable.
    #[arg(long, value_name = "R,G,B=INK", required_unless_present = "config")]
    pick: Vec<String>,
    /// Material color the image is composited over (the garment or substrate). Default: the
    /// config's material with --config, else white; given with --config, it overrides the config's.
    #[arg(long, value_name = "R,G,B", value_parser = parse_rgb)]
    material: Option<Rgb8>,
    /// A color left unprinted (transparent in the output): "R,G,B[,A]=ΔE", or "material=ΔE" for
    /// the material's own color. Repeatable. With --config, the config's are used.
    #[arg(long, value_name = "COLOR=ΔE", value_parser = parse_unprinted, conflicts_with = "config")]
    unprinted: Vec<Unprinted>,
}

#[derive(Subcommand)]
enum PalettesCommand {
    /// Write a first-version config next to every image (existing configs are kept).
    Generate {
        dir: PathBuf,
        /// Overwrite existing configs (they may contain hand edits).
        #[arg(long)]
        force: bool,
        /// Material color to match on, written into each config (default: white).
        #[arg(long, value_name = "R,G,B", value_parser = parse_rgb)]
        material: Option<Rgb8>,
    },
}

#[derive(Subcommand)]
enum GoldenCommand {
    /// Write <name>-out-<size>.png next to every image that has a config.
    Update { dir: PathBuf },
}

// Program start. Logging is off except warnings unless `RUST_LOG` says otherwise (e.g.
// `RUST_LOG=debug`). Errors print as `error: …`, with `{e:#}` adding the chain of causes, and the
// exit code tells scripts it failed.
fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

// `palette_path` is a closure called only by the commands that need a palette, so `analyze` works
// without one.
fn run(cli: Cli) -> Result<()> {
    let palette_path = || -> Result<PathBuf> {
        match &cli.palette {
            Some(path) => Ok(path.clone()),
            None => palette_file::find_default(&std::env::current_dir()?),
        }
    };
    match cli.command {
        Command::Analyze { input, material } => {
            let material = material.unwrap_or(Rgb8::WHITE);
            let image = decode(&input)?;
            let s = analyze(image.view(), material);
            println!(
                "{}: {}×{}, {} colors (composited over {}), {} RGBA colors",
                input.display(),
                s.width,
                s.height,
                s.colors,
                describe(material),
                s.rgba_colors
            );
            Ok(())
        }
        Command::Recolor(args) => {
            let palette = palette_file::load(&palette_path()?)?;
            // Either the picks come from a config file (`--config` and `--size`) or from `--pick`
            // options.
            let (mappings, material, ranges) = match (&args.config, args.size) {
                (Some(path), Some(size)) => {
                    let config = config::load(path)?;
                    // An explicit --material overrides the config's; the config's unprinted
                    // colors are used, the material's own one on the material in use.
                    let material = args.material.unwrap_or(config.material.into());
                    let mappings = config
                        .size(size)?
                        .mappings(&palette, material)
                        .with_context(|| format!("{}", path.display()))?;
                    (mappings, material, config.ranges(material))
                }
                _ => {
                    let mappings = args
                        .pick
                        .iter()
                        .map(|p| parse_pick(p, &palette))
                        // Collecting into a `Result` gives the list if every pick parsed, or the
                        // first error. `::<...>` ("turbofish") names the type to collect into.
                        .collect::<Result<_>>()?;
                    let material = args.material.unwrap_or(Rgb8::WHITE);
                    // A config with no sections, built only to reuse its validation and range
                    // conversion for the `--unprinted` options.
                    let unprinted = PaletteConfig {
                        material: [material.r, material.g, material.b],
                        unprinted: args.unprinted.clone(),
                        palette: vec![],
                    };
                    unprinted.validate()?;
                    (mappings, material, unprinted.ranges(material))
                }
            };
            let image = decode(&args.input)?;
            let stats = recolor_to_file(&image, &mappings, material, &ranges, &args.output)?;
            println!(
                "{} → {} ({} picks on {}; {} exact, {} unprinted, {} nearest pixels)",
                args.input.display(),
                args.output.display(),
                mappings.len(),
                describe(material),
                stats.exact,
                stats.unprinted,
                stats.nearest
            );
            Ok(())
        }
        Command::Palettes(PalettesCommand::Generate {
            dir,
            force,
            material,
        }) => {
            // A chosen material leaves its own color unprinted, as in the app. `Option` → list with
            // zero or one entry: `map` builds the entry if there is a material, and
            // `into_iter().collect()` turns `Some(x)` into `[x]` and `None` into `[]`.
            let unprinted = material
                .map(|_| Unprinted::Material {
                    delta_e: DEFAULT_UNPRINTED_DELTA_E,
                })
                .into_iter()
                .collect::<Vec<_>>();
            let material = material.unwrap_or(Rgb8::WHITE);
            let palette = palette_file::load(&palette_path()?)?;
            for input in discover::images(&dir)? {
                let path = config::config_path(&input);
                if path.exists() && !force {
                    println!("{}: kept existing config", path.display());
                    continue;
                }
                let image = decode(&input)?;
                // Pick once for the largest size; each smaller size takes the first N of those
                // picks.
                let max = *GOLDEN_SIZES.iter().max().expect("sizes") as usize;
                let picks = generate::picks(image.view(), &palette, material, max);
                let config = PaletteConfig {
                    material: [material.r, material.g, material.b],
                    unprinted: unprinted.clone(),
                    palette: GOLDEN_SIZES
                        .iter()
                        .map(|&size| SizedPalette {
                            size,
                            picks: picks
                                .iter()
                                .take(size as usize)
                                .map(|p| ConfigPick {
                                    rgba: [p.rgba.r, p.rgba.g, p.rgba.b, p.rgba.a],
                                    ink: palette.entries()[p.ink_index].name.clone(),
                                    delta_e: 0.0,
                                })
                                .collect(),
                        })
                        .collect(),
                };
                let name = input
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                std::fs::write(
                    &path,
                    config::golden_toml(&config, &name, "rekolor palettes generate"),
                )
                .with_context(|| format!("writing {}", path.display()))?;
                let found: Vec<String> = config
                    .palette
                    .iter()
                    .map(|p| format!("{}/{}", p.picks.len(), p.size))
                    .collect();
                println!("{}: {} inks", path.display(), found.join(", "));
            }
            Ok(())
        }
        Command::Golden(GoldenCommand::Update { dir }) => {
            let palette = palette_file::load(&palette_path()?)?;
            // Pass 1: check every config (sizes, ink names) and every image before writing
            // anything, so a bad file anywhere in the tree leaves all outputs untouched.
            let mut jobs: Vec<GoldenJob> = Vec::new();
            let mut problems: Vec<String> = Vec::new();
            for input in discover::images(&dir)? {
                let config_path = config::config_path(&input);
                if !config_path.exists() {
                    eprintln!("{}: no config, skipped", input.display());
                    continue;
                }
                // `and_then` runs the closure only if loading succeeded; any error inside it
                // short-circuits.
                let checked = config::load(&config_path).and_then(|config| {
                    let material = Rgb8::from(config.material);
                    let sizes = config
                        .palette
                        .iter()
                        .map(|sized| Ok((sized.size, sized.mappings(&palette, material)?)))
                        .collect::<Result<Vec<_>>>()
                        .with_context(|| format!("{}", config_path.display()))?;
                    Ok((material, config.ranges(material), sizes))
                });
                let decodable = rekolor_io::decode_file(&input)
                    .map(|_| ())
                    .with_context(|| format!("{}", input.display()));
                match (checked, decodable) {
                    (Ok((material, ranges, sizes)), Ok(())) => jobs.push(GoldenJob {
                        input,
                        material,
                        ranges,
                        sizes,
                    }),
                    // Any failure: collect both errors (if both failed); `.err()` gives
                    // `Some(error)` or `None`, and `flatten` drops the `None`s.
                    (checked, decodable) => problems.extend(
                        [checked.err(), decodable.err()]
                            .into_iter()
                            .flatten()
                            .map(|e| format!("{e:#}")),
                    ),
                }
            }
            if !problems.is_empty() {
                bail!(
                    "nothing written; fix these first:\n  {}",
                    problems.join("\n  ")
                );
            }
            // Pass 2: render and write.
            // The `for` pattern unpacks each job's fields into variables.
            for GoldenJob {
                input,
                material,
                ranges,
                sizes,
            } in jobs
            {
                let image = decode(&input)?;
                for (size, mappings) in sizes {
                    let output = config::output_path(&input, size);
                    recolor_to_file(&image, &mappings, material, &ranges, &output)?;
                    println!("{}", output.display());
                }
            }
            Ok(())
        }
    }
}

// `Vec<(u32, Vec<Mapping>)>`: a list of (palette size, mappings for that size) pairs.
/// One image of a `golden update`, checked in pass 1: its config's material and its outputs per
/// palette size.
struct GoldenJob {
    input: PathBuf,
    material: Rgb8,
    /// The config's unprinted colors on its material.
    ranges: Vec<MaterialRange>,
    sizes: Vec<(u32, Vec<Mapping>)>,
}

/// Decodes an image and prints its decoder warnings to stderr.
fn decode(path: &Path) -> Result<DecodedImage> {
    let image = rekolor_io::decode_file(path)?;
    for warning in image.warnings() {
        eprintln!("warning: {}: {warning}", path.display());
    }
    Ok(image)
}

// Recolors into a new buffer of the same size and writes it as PNG.
fn recolor_to_file(
    image: &DecodedImage,
    mappings: &[Mapping],
    material: Rgb8,
    ranges: &[MaterialRange],
    output: &Path,
) -> Result<rekolor_core::RecolorStats> {
    let mut out = vec![0; image.rgba().len()];
    let stats = recolor_with_ranges(image.view(), mappings, material, ranges, &mut out)?;
    rekolor_io::write_png(output, &out, image.width(), image.height())?;
    Ok(stats)
}

/// "white" or "(r, g, b)", for messages.
fn describe(material: Rgb8) -> String {
    if material == Rgb8::WHITE {
        "white".into()
    } else {
        format!("({}, {}, {})", material.r, material.g, material.b)
    }
}

/// Parses "R,G,B" (a pick's color, `--material`).
fn parse_rgb(text: &str) -> Result<Rgb8, String> {
    // Split at commas and parse each part as a byte; `collect::<Result<_, _>>` fails if any part
    // isn't a number 0–255.
    let channels: Vec<u8> = text
        .split(',')
        .map(|c| c.trim().parse::<u8>())
        .collect::<Result<_, _>>()
        .map_err(|_| "R,G,B must be numbers 0–255".to_string())?;
    // Exactly three channels, or an error. `channels[..]` views the `Vec` as a slice, which can be
    // matched against a fixed pattern like `[r, g, b]`.
    let [r, g, b] = channels[..] else {
        return Err("expected three channels R,G,B".into());
    };
    Ok(Rgb8::new(r, g, b))
}

/// Parses "R,G,B[,A]=ΔE" or "material=ΔE" (`--unprinted`); the ΔE range is checked with the
/// config rules.
fn parse_unprinted(text: &str) -> Result<Unprinted, String> {
    // `split_once('=')` gives the parts before and after the first `=`, or `None` if there is none.
    let (color, delta_e) = text
        .split_once('=')
        .ok_or_else(|| "expected R,G,B[,A]=ΔE or material=ΔE".to_string())?;
    let delta_e: f32 = delta_e
        .trim()
        .parse()
        .map_err(|_| format!("ΔE {:?} is not a number", delta_e.trim()))?;
    if color.trim() == "material" {
        return Ok(Unprinted::Material { delta_e });
    }
    let channels: Vec<u8> = color
        .split(',')
        .map(|c| c.trim().parse::<u8>())
        .collect::<Result<_, _>>()
        .map_err(|_| "R,G,B[,A] must be numbers 0–255".to_string())?;
    // Three channels mean opaque (alpha 255); four give the alpha too.
    let rgba = match channels[..] {
        [r, g, b] => [r, g, b, 255],
        [r, g, b, a] => [r, g, b, a],
        _ => return Err("expected three or four channels R,G,B[,A]".into()),
    };
    Ok(Unprinted::Color { rgba, delta_e })
}

/// Parses "R,G,B=INK NAME".
fn parse_pick(text: &str, palette: &Palette) -> Result<Mapping> {
    let Some((color, ink)) = text.split_once('=') else {
        bail!("pick {text:?}: expected R,G,B=INK NAME");
    };
    // `parse_rgb` returns a `String` error (as clap wants); `anyhow!` makes it an `anyhow::Error`
    // with the pick's text added.
    let source = parse_rgb(color).map_err(|e| anyhow::anyhow!("pick {text:?}: {e}"))?;
    let ink = ink.trim();
    let entry = palette
        .entries()
        .iter()
        .find(|e| e.name == ink)
        .with_context(|| format!("pick {text:?}: ink {ink:?} is not in the palette"))?;
    Ok(Mapping {
        source,
        ink: entry.rgb,
        delta_e: 0.0,
    })
}
