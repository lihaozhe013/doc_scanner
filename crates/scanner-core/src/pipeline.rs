use std::path::Path;

use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};

use crate::{
    CoreError,
    backend::perspective_warp,
    effects,
    geometry::{
        Bounds, MAX_IMAGE_DIMENSION, MAX_IMAGE_PIXELS, OutputDimensions,
        Quadrilateral,
    },
    model::{
        EditState, ImageMetadata, LoadedSource, RasterFormat, SourceImage,
    },
};

pub const DEFAULT_PREVIEW_MAX_DIMENSION: u32 = 1_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingMode {
    Preview { max_dimension: u32 },
    FullResolution,
}

#[derive(Debug)]
pub struct ProcessingResult {
    pub image: DynamicImage,
    pub output_dimensions: OutputDimensions,
}

struct DecodedImage {
    image: DynamicImage,
    orientation: crate::CanonicalOrientation,
}

pub fn load_image(path: &Path) -> Result<LoadedSource, CoreError> {
    let format = source_format(path)?;
    let decoded = decode_checked(path)?;
    let metadata = metadata_for(&decoded.image, format, decoded.orientation)?;
    let source =
        SourceImage::new(crate::ImageId::new(), path.to_owned(), metadata);
    Ok(LoadedSource {
        source,
        image: decoded.image,
    })
}

pub fn process_image(
    path: &Path,
    edit: &EditState,
    mode: ProcessingMode,
) -> Result<ProcessingResult, CoreError> {
    let format = source_format(path)?;
    let decoded = decode_checked(path)?;
    metadata_for(&decoded.image, format, decoded.orientation)?;
    let decoded = decoded.image;
    let working = match mode {
        ProcessingMode::Preview { max_dimension } => {
            resize_for_preview(decoded, max_dimension)?
        }
        ProcessingMode::FullResolution => decoded,
    };

    effects::validate_settings(&edit.enhancement)?;
    let validated = edit.quadrilateral.validate(Bounds::unit())?;
    let ordered = Quadrilateral::new(validated.points).ordered()?;
    let validated = ordered.validate(Bounds::unit())?;
    let output_dimensions =
        validated.output_dimensions(working.width(), working.height())?;
    let warped = perspective_warp(&working, &validated, output_dimensions)?;
    let enhanced = effects::apply(warped, &edit.enhancement)?;
    Ok(ProcessingResult {
        image: enhanced,
        output_dimensions,
    })
}

fn decode_checked(path: &Path) -> Result<DecodedImage, CoreError> {
    let reader = ImageReader::open(path)
        .map_err(|error| CoreError::io("opening", error))?;
    let mut decoder =
        reader.into_decoder().map_err(|error| CoreError::Decode {
            message: error.to_string(),
        })?;
    let (width, height) = decoder.dimensions();
    validate_dimensions(width, height)?;

    let orientation =
        decoder.orientation().map_err(|error| CoreError::Decode {
            message: error.to_string(),
        })?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|error| {
        CoreError::Decode {
            message: error.to_string(),
        }
    })?;
    image.apply_orientation(orientation);
    Ok(DecodedImage {
        image,
        orientation: crate::CanonicalOrientation::Normalized,
    })
}

pub(crate) fn validate_dimensions(
    width: u32,
    height: u32,
) -> Result<(), CoreError> {
    if width == 0 || height == 0 {
        return Err(CoreError::InvalidDimensions { width, height });
    }
    if width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || u64::from(width).saturating_mul(u64::from(height)) > MAX_IMAGE_PIXELS
    {
        return Err(CoreError::AllocationTooLarge { width, height });
    }
    Ok(())
}

fn source_format(path: &Path) -> Result<RasterFormat, CoreError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    extension
        .as_deref()
        .and_then(RasterFormat::from_extension)
        .ok_or_else(|| CoreError::UnsupportedFormat {
            extension: extension
                .map(|value| format!(" `{value}`"))
                .unwrap_or_else(|| " without a supported extension".to_owned()),
        })
}

fn metadata_for(
    image: &DynamicImage,
    format: RasterFormat,
    orientation: crate::CanonicalOrientation,
) -> Result<ImageMetadata, CoreError> {
    validate_dimensions(image.width(), image.height())?;
    Ok(ImageMetadata {
        width: image.width(),
        height: image.height(),
        channels: image.color().channel_count(),
        format,
        orientation,
    })
}

fn resize_for_preview(
    image: DynamicImage,
    max_dimension: u32,
) -> Result<DynamicImage, CoreError> {
    if max_dimension == 0 {
        return Err(CoreError::InvalidParameter {
            field: "preview_max_dimension",
            reason: "must be greater than zero".to_owned(),
        });
    }
    if image.width().max(image.height()) <= max_dimension {
        return Ok(image);
    }
    let scale =
        f64::from(max_dimension) / f64::from(image.width().max(image.height()));
    let width = (f64::from(image.width()) * scale).round().max(1.0) as u32;
    let height = (f64::from(image.height()) * scale).round().max(1.0) as u32;
    validate_dimensions(width, height)?;
    Ok(image.resize(width, height, FilterType::Triangle))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use image::{DynamicImage, GenericImageView, ImageFormat, Rgb, RgbImage};

    use super::*;

    #[test]
    fn preview_keeps_the_same_pipeline_and_bounds_output() {
        let directory = tempfile_directory();
        let path = directory.join("page.png");
        let mut image = RgbImage::new(32, 20);
        for pixel in image.pixels_mut() {
            *pixel = Rgb([220, 220, 220]);
        }
        DynamicImage::ImageRgb8(image)
            .save_with_format(&path, ImageFormat::Png)
            .unwrap();

        let result = process_image(
            &path,
            &EditState::default(),
            ProcessingMode::Preview { max_dimension: 16 },
        )
        .unwrap();
        assert_eq!(result.image.dimensions(), (16, 10));
        assert_eq!(
            result.output_dimensions,
            OutputDimensions {
                width: 16,
                height: 10
            }
        );
        let _ = fs::remove_dir_all(directory);
    }

    fn tempfile_directory() -> std::path::PathBuf {
        let directory = std::env::temp_dir()
            .join(format!("scanner-core-{}", crate::ImageId::new().as_uuid()));
        fs::create_dir_all(&directory).unwrap();
        directory
    }
}
