use std::fs::{self, File, OpenOptions};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageFormat, codecs::jpeg::JpegEncoder};
use uuid::Uuid;

use crate::{
    CoreError,
    model::{EditState, OutputFormat},
    pipeline::{ProcessingMode, process_image},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionPolicy {
    AutoRename,
    FailIfExists,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportResult {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub format: OutputFormat,
}

pub fn export_image(
    source_path: &Path,
    edit: &EditState,
    destination_dir: &Path,
    collision_policy: CollisionPolicy,
) -> Result<ExportResult, CoreError> {
    if edit.output.jpeg_quality == 0 {
        return Err(CoreError::InvalidParameter {
            field: "jpeg_quality",
            reason: "must be between 1 and 100".to_owned(),
        });
    }
    fs::create_dir_all(destination_dir)
        .map_err(|error| CoreError::export_io("creating destination", error))?;
    let processed =
        process_image(source_path, edit, ProcessingMode::FullResolution)?;
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("scan");
    let output_path = next_available_output_path(
        destination_dir,
        stem,
        edit.output.format,
        collision_policy,
    )?;
    atomic_encode(
        &processed.image,
        &output_path,
        edit.output.format,
        edit.output.jpeg_quality,
    )?;

    let reopened = crate::load_image(&output_path)?;
    if reopened.source.metadata.width != processed.output_dimensions.width
        || reopened.source.metadata.height != processed.output_dimensions.height
    {
        let _ = fs::remove_file(&output_path);
        return Err(CoreError::InvalidOutputDimensions {
            width: reopened.source.metadata.width,
            height: reopened.source.metadata.height,
        });
    }

    Ok(ExportResult {
        path: output_path,
        width: reopened.source.metadata.width,
        height: reopened.source.metadata.height,
        format: edit.output.format,
    })
}

pub fn next_available_output_path(
    destination_dir: &Path,
    stem: &str,
    format: OutputFormat,
    collision_policy: CollisionPolicy,
) -> Result<PathBuf, CoreError> {
    let stem = safe_stem(stem);
    let base = destination_dir.join(format!("{}.{}", stem, format.extension()));
    if !base.exists() {
        return Ok(base);
    }
    if collision_policy == CollisionPolicy::FailIfExists {
        return Err(CoreError::OutputExists);
    }

    for index in 1_u32.. {
        let candidate = destination_dir.join(format!(
            "{} ({}).{}",
            stem,
            index,
            format.extension()
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(CoreError::OutputExists)
}

fn atomic_encode(
    image: &DynamicImage,
    destination: &Path,
    format: OutputFormat,
    quality: u8,
) -> Result<(), CoreError> {
    let parent = destination.parent().ok_or_else(|| {
        CoreError::export_io(
            "resolving destination directory",
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "destination has no parent",
            ),
        )
    })?;
    let temporary_path = create_temporary_file(parent)?;
    let encode_result = encode_to_file(image, &temporary_path, format, quality);
    if let Err(error) = encode_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    match fs::hard_link(&temporary_path, destination) {
        Ok(()) => {
            let _ = fs::remove_file(&temporary_path);
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(&temporary_path);
            return Err(CoreError::OutputExists);
        }
        Err(_) if !destination.exists() => {}
        Err(error) => {
            let _ = fs::remove_file(&temporary_path);
            return Err(CoreError::export_io("linking output", error));
        }
    }

    if let Err(error) = fs::rename(&temporary_path, destination) {
        let _ = fs::remove_file(&temporary_path);
        return if error.kind() == std::io::ErrorKind::AlreadyExists {
            Err(CoreError::OutputExists)
        } else {
            Err(CoreError::export_io("replacing destination", error))
        };
    }
    Ok(())
}

fn safe_stem(stem: &str) -> String {
    let sanitized = stem
        .chars()
        .map(|character| {
            if character == '/' || character == '\\' || character.is_control() {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "scan".to_owned()
    } else {
        sanitized
    }
}

fn create_temporary_file(parent: &Path) -> Result<PathBuf, CoreError> {
    for _ in 0..16 {
        let path = parent.join(format!(".scanner-{}.tmp", Uuid::new_v4()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                continue;
            }
            Err(error) => {
                return Err(CoreError::export_io(
                    "creating temporary output",
                    error,
                ));
            }
        }
    }
    Err(CoreError::export_io(
        "creating temporary output",
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique temporary file",
        ),
    ))
}

fn encode_to_file(
    image: &DynamicImage,
    path: &Path,
    format: OutputFormat,
    quality: u8,
) -> Result<(), CoreError> {
    let mut file = File::options().write(true).open(path).map_err(|error| {
        CoreError::export_io("opening temporary output", error)
    })?;
    match format {
        OutputFormat::Jpeg => {
            let mut encoder =
                JpegEncoder::new_with_quality(&mut file, quality.clamp(1, 100));
            encoder
                .encode_image(image)
                .map_err(|error| CoreError::Encode {
                    message: error.to_string(),
                })?;
        }
        OutputFormat::Png => {
            write_with_format(&mut file, image, ImageFormat::Png)?
        }
        OutputFormat::Bmp => {
            write_with_format(&mut file, image, ImageFormat::Bmp)?
        }
        OutputFormat::Tiff => {
            write_with_format(&mut file, image, ImageFormat::Tiff)?
        }
    }
    file.flush()
        .and_then(|_| file.sync_all())
        .map_err(|error| {
            CoreError::export_io("flushing temporary output", error)
        })?;
    Ok(())
}

fn write_with_format(
    file: &mut (impl Write + Seek),
    image: &DynamicImage,
    format: ImageFormat,
) -> Result<(), CoreError> {
    image
        .write_to(file, format)
        .map_err(|error| CoreError::Encode {
            message: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn collision_names_are_deterministic() {
        let directory = temp_directory();
        let first = directory.join("page.png");
        let second = directory.join("page (1).png");
        fs::write(&first, b"existing").unwrap();
        assert_eq!(
            next_available_output_path(
                &directory,
                "page",
                OutputFormat::Png,
                CollisionPolicy::AutoRename,
            )
            .unwrap(),
            second
        );
        assert!(matches!(
            next_available_output_path(
                &directory,
                "page",
                OutputFormat::Png,
                CollisionPolicy::FailIfExists,
            ),
            Err(CoreError::OutputExists)
        ));
        let _ = fs::remove_dir_all(directory);
    }

    fn temp_directory() -> PathBuf {
        let directory = std::env::temp_dir()
            .join(format!("scanner-export-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        directory
    }
}
