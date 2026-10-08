//! The reKolor command-line tool (R1, R6).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use rekolor_cli::config::{self, ConfigPick, PaletteConfig, SizedPalette};
use rekolor_cli::{discover, generate, palette_file};
use rekolor_core::{Mapping, Palette, Rgb8, analyze, recolor};
use rekolor_io::DecodedImage;

/// Palette sizes of the golden set (X1).
const GOLDEN_SIZES: [u32; 3] = [3, 7, 16];

#[derive(Parser)]
#[command(name = "rekolor", version, about = "Limited-palette print previews")]
struct Cli {
    /// Palette file (JSON). Default: palettes/pantone.json in the current or a parent directory.
    #[arg(long, global = true, value_name = "PATH")]
    palette: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Recolor an image with picks from a palette config or given on the command line.
    Recolor(RecolorArgs),
    /// Print an image's size, color counts and decoder warnings.
    Analyze { input: PathBuf },
    /// Palette configs (<name>.palettes.toml) for a samples tree.
    #[command(subcommand)]
    Palettes(PalettesCommand),
    /// Golden outputs (<name>-out-<size>.png) for a samples tree.
    #[command(subcommand)]
    Golden(GoldenCommand),
}

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
}

#[derive(Subcommand)]
enum PalettesCommand {
    /// Write a first-version config next to every image (existing configs are kept).
    Generate {
        dir: PathBuf,
        /// Overwrite existing configs (they may contain hand edits).
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum GoldenCommand {
    /// Write <name>-out-<size>.png next to every image that has a config.
    Update { dir: PathBuf },
}

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

fn run(cli: Cli) -> Result<()> {
    let palette_path = || -> Result<PathBuf> {
        match &cli.palette {
            Some(path) => Ok(path.clone()),
            None => palette_file::find_default(&std::env::current_dir()?),
        }
    };
    match cli.command {
        Command::Analyze { input } => {
            let image = decode(&input)?;
            let s = analyze(image.view());
            println!(
                "{}: {}×{}, {} colors (composited over white), {} RGBA colors",
                input.display(),
                s.width,
                s.height,
                s.colors,
                s.rgba_colors
            );
            Ok(())
        }
        Command::Recolor(args) => {
            let palette = palette_file::load(&palette_path()?)?;
            let mappings = match (&args.config, args.size) {
                (Some(path), Some(size)) => config::load(path)?
                    .size(size)?
                    .mappings(&palette)
                    .with_context(|| format!("{}", path.display()))?,
                _ => args
                    .pick
                    .iter()
                    .map(|p| parse_pick(p, &palette))
                    .collect::<Result<_>>()?,
            };
            let image = decode(&args.input)?;
            let stats = recolor_to_file(&image, &mappings, &args.output)?;
            println!(
                "{} → {} ({} picks; {} exact, {} nearest pixels)",
                args.input.display(),
                args.output.display(),
                mappings.len(),
                stats.exact,
                stats.nearest
            );
            Ok(())
        }
        Command::Palettes(PalettesCommand::Generate { dir, force }) => {
            let palette = palette_file::load(&palette_path()?)?;
            for input in discover::images(&dir)? {
                let path = config::config_path(&input);
                if path.exists() && !force {
                    println!("{}: kept existing config", path.display());
                    continue;
                }
                let image = decode(&input)?;
                let max = *GOLDEN_SIZES.iter().max().expect("sizes") as usize;
                let picks = generate::picks(image.view(), &palette, max);
                let config = PaletteConfig {
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
                let checked = config::load(&config_path).and_then(|config| {
                    config
                        .palette
                        .iter()
                        .map(|sized| Ok((sized.size, sized.mappings(&palette)?)))
                        .collect::<Result<Vec<_>>>()
                        .with_context(|| format!("{}", config_path.display()))
                });
                let decodable = rekolor_io::decode_file(&input)
                    .map(|_| ())
                    .with_context(|| format!("{}", input.display()));
                match (checked, decodable) {
                    (Ok(sizes), Ok(())) => jobs.push(GoldenJob { input, sizes }),
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
            for GoldenJob { input, sizes } in jobs {
                let image = decode(&input)?;
                for (size, mappings) in sizes {
                    let output = config::output_path(&input, size);
                    recolor_to_file(&image, &mappings, &output)?;
                    println!("{}", output.display());
                }
            }
            Ok(())
        }
    }
}

/// One image of a `golden update`, checked in pass 1: its outputs per palette size.
struct GoldenJob {
    input: PathBuf,
    sizes: Vec<(u32, Vec<Mapping>)>,
}

/// Decodes an image and prints its decoder warnings (M1) to stderr.
fn decode(path: &Path) -> Result<DecodedImage> {
    let image = rekolor_io::decode_file(path)?;
    for warning in image.warnings() {
        eprintln!("warning: {}: {warning}", path.display());
    }
    Ok(image)
}

fn recolor_to_file(
    image: &DecodedImage,
    mappings: &[Mapping],
    output: &Path,
) -> Result<rekolor_core::RecolorStats> {
    let mut out = vec![0; image.rgba().len()];
    let stats = recolor(image.view(), mappings, &mut out)?;
    rekolor_io::write_png(output, &out, image.width(), image.height())?;
    Ok(stats)
}

/// Parses "R,G,B=INK NAME".
fn parse_pick(text: &str, palette: &Palette) -> Result<Mapping> {
    let Some((color, ink)) = text.split_once('=') else {
        bail!("pick {text:?}: expected R,G,B=INK NAME");
    };
    let channels: Vec<u8> = color
        .split(',')
        .map(|c| c.trim().parse::<u8>())
        .collect::<Result<_, _>>()
        .with_context(|| format!("pick {text:?}: R,G,B must be numbers 0–255"))?;
    let [r, g, b] = channels[..] else {
        bail!("pick {text:?}: expected three channels R,G,B");
    };
    let ink = ink.trim();
    let entry = palette
        .entries()
        .iter()
        .find(|e| e.name == ink)
        .with_context(|| format!("pick {text:?}: ink {ink:?} is not in the palette"))?;
    Ok(Mapping {
        source: Rgb8::new(r, g, b),
        ink: entry.rgb,
    })
}
