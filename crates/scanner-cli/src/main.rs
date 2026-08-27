use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use scanner_core::{
    CollisionPolicy, EditState, EnhancementPreset, OutputFormat, export_image,
    load_image,
};

#[derive(Debug, Parser)]
#[command(
    name = "scanner-cli",
    version,
    about = "Process raster document pages locally"
)]
struct Arguments {
    #[arg(short, long, value_name = "PATH")]
    input: PathBuf,

    #[arg(short, long, value_name = "DIRECTORY")]
    output: PathBuf,

    #[arg(long, value_enum, default_value_t = PresetArgument::Original)]
    preset: PresetArgument,

    #[arg(long, value_enum, default_value_t = FormatArgument::Png)]
    format: FormatArgument,

    #[arg(long, default_value_t = 92, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,

    #[arg(long, help = "Fail instead of auto-renaming an existing output")]
    fail_on_collision: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PresetArgument {
    #[value(name = "original")]
    Original,
    #[value(name = "adaptive-black-and-white")]
    AdaptiveBlackAndWhite,
    #[value(name = "enhanced-color")]
    EnhancedColor,
    #[value(name = "magic-color")]
    MagicColor,
}

impl From<PresetArgument> for EnhancementPreset {
    fn from(value: PresetArgument) -> Self {
        match value {
            PresetArgument::Original => Self::Original,
            PresetArgument::AdaptiveBlackAndWhite => {
                Self::AdaptiveBlackAndWhite
            }
            PresetArgument::EnhancedColor => Self::EnhancedColor,
            PresetArgument::MagicColor => Self::MagicColor,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArgument {
    Jpeg,
    Png,
    Bmp,
    Tiff,
}

impl From<FormatArgument> for OutputFormat {
    fn from(value: FormatArgument) -> Self {
        match value {
            FormatArgument::Jpeg => Self::Jpeg,
            FormatArgument::Png => Self::Png,
            FormatArgument::Bmp => Self::Bmp,
            FormatArgument::Tiff => Self::Tiff,
        }
    }
}

fn main() -> Result<ExitCode> {
    let arguments = Arguments::parse();
    let paths = input_paths(&arguments.input)?;
    let collision_policy = if arguments.fail_on_collision {
        CollisionPolicy::FailIfExists
    } else {
        CollisionPolicy::AutoRename
    };
    let mut succeeded = 0_usize;
    let mut failed = 0_usize;

    for path in paths {
        let display_name = scanner_core::display_name_for_path(&path);
        let result = process_one(
            &path,
            &arguments.output,
            arguments.preset.into(),
            arguments.format.into(),
            arguments.quality,
            collision_policy,
        );
        match result {
            Ok(output) => {
                println!(
                    "Exported {} ({} × {})",
                    output.path.display(),
                    output.width,
                    output.height
                );
                succeeded += 1;
            }
            Err(error) => {
                eprintln!("Failed {}: {error:#}", display_name);
                failed += 1;
            }
        }
    }

    println!("Completed: {succeeded} succeeded, {failed} failed");
    if failed > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn process_one(
    input: &Path,
    output_directory: &Path,
    preset: EnhancementPreset,
    format: OutputFormat,
    quality: u8,
    collision_policy: CollisionPolicy,
) -> Result<scanner_core::ExportResult> {
    let (width, height) = {
        let loaded =
            load_image(input).with_context(|| "loading input metadata")?;
        (loaded.source.metadata.width, loaded.source.metadata.height)
    };
    let mut edit = EditState::for_dimensions(width, height);
    edit.enhancement.preset = preset;
    edit.output.format = format;
    edit.output.jpeg_quality = quality;
    export_image(input, &edit, output_directory, collision_policy)
        .with_context(|| "processing and exporting image")
}

fn input_paths(input: &Path) -> Result<Vec<PathBuf>> {
    if input.is_file() {
        return Ok(vec![input.to_owned()]);
    }
    if !input.is_dir() {
        bail!("input path is not a file or directory");
    }
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(input).context("reading input directory")? {
        let entry = entry.context("reading directory entry")?;
        let path = entry.path();
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort_by(|left, right| {
        scanner_core::display_name_for_path(left)
            .to_ascii_lowercase()
            .cmp(
                &scanner_core::display_name_for_path(right)
                    .to_ascii_lowercase(),
            )
            .then_with(|| {
                scanner_core::display_name_for_path(left)
                    .cmp(&scanner_core::display_name_for_path(right))
            })
    });
    if paths.is_empty() {
        bail!("input directory contains no files");
    }
    Ok(paths)
}
