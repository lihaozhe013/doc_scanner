use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Rgb, RgbImage};

use crate::{CoreError, model::EnhancementSettings};

pub fn validate_settings(
    settings: &EnhancementSettings,
) -> Result<(), CoreError> {
    if settings.adaptive_block_size < 3
        || settings.adaptive_block_size.is_multiple_of(2)
    {
        return Err(CoreError::InvalidParameter {
            field: "adaptive_block_size",
            reason: "must be an odd value of at least 3".to_owned(),
        });
    }
    if !settings.adaptive_threshold_offset.is_finite()
        || !(-128.0..=128.0).contains(&settings.adaptive_threshold_offset)
    {
        return Err(CoreError::InvalidParameter {
            field: "adaptive_threshold_offset",
            reason: "must be finite and between -128 and 128".to_owned(),
        });
    }
    if !settings.color_brightness.is_finite()
        || !(-255.0..=255.0).contains(&settings.color_brightness)
    {
        return Err(CoreError::InvalidParameter {
            field: "color_brightness",
            reason: "must be finite and between -255 and 255".to_owned(),
        });
    }
    if !settings.color_contrast.is_finite()
        || !(0.0..=4.0).contains(&settings.color_contrast)
    {
        return Err(CoreError::InvalidParameter {
            field: "color_contrast",
            reason: "must be finite and between 0 and 4".to_owned(),
        });
    }
    if !settings.denoise_strength.is_finite()
        || !(0.0..=1.0).contains(&settings.denoise_strength)
    {
        return Err(CoreError::InvalidParameter {
            field: "denoise_strength",
            reason: "must be finite and between 0 and 1".to_owned(),
        });
    }
    if !settings.sharpening_strength.is_finite()
        || !(0.0..=2.0).contains(&settings.sharpening_strength)
    {
        return Err(CoreError::InvalidParameter {
            field: "sharpening_strength",
            reason: "must be finite and between 0 and 2".to_owned(),
        });
    }
    if !settings.magic_local_contrast.is_finite()
        || !(0.0..=4.0).contains(&settings.magic_local_contrast)
    {
        return Err(CoreError::InvalidParameter {
            field: "magic_local_contrast",
            reason: "must be finite and between 0 and 4".to_owned(),
        });
    }
    if !settings.magic_saturation.is_finite()
        || !(0.0..=4.0).contains(&settings.magic_saturation)
    {
        return Err(CoreError::InvalidParameter {
            field: "magic_saturation",
            reason: "must be finite and between 0 and 4".to_owned(),
        });
    }
    Ok(())
}

pub fn apply(
    image: DynamicImage,
    settings: &EnhancementSettings,
) -> Result<DynamicImage, CoreError> {
    validate_settings(settings)?;
    if image.width() == 0 || image.height() == 0 {
        return Err(CoreError::InvalidDimensions {
            width: image.width(),
            height: image.height(),
        });
    }
    match settings.preset {
        crate::EnhancementPreset::Original => Ok(image),
        crate::EnhancementPreset::AdaptiveBlackAndWhite => {
            Ok(DynamicImage::ImageLuma8(adaptive_black_and_white(
                &image,
                settings.adaptive_block_size,
                settings.adaptive_threshold_offset,
            )))
        }
        crate::EnhancementPreset::EnhancedColor => {
            Ok(DynamicImage::ImageRgb8(enhanced_color(&image, settings)))
        }
        crate::EnhancementPreset::MagicColor => {
            Ok(DynamicImage::ImageRgb8(magic_color(&image, settings)))
        }
    }
}

fn adaptive_black_and_white(
    image: &DynamicImage,
    block_size: u32,
    offset: f32,
) -> GrayImage {
    let gray = image.to_luma8();
    let (width, height) = gray.dimensions();
    let stride = width as usize + 1;
    let mut integral = vec![0_u64; (height as usize + 1) * stride];

    for y in 0..height as usize {
        let mut row_sum = 0_u64;
        for x in 0..width as usize {
            row_sum += u64::from(gray.get_pixel(x as u32, y as u32)[0]);
            let index = (y + 1) * stride + x + 1;
            integral[index] = integral[y * stride + x + 1] + row_sum;
        }
    }

    let radius = block_size / 2;
    ImageBuffer::from_fn(width, height, |x, y| {
        let left = x.saturating_sub(radius) as usize;
        let top = y.saturating_sub(radius) as usize;
        let right = (x + radius + 1).min(width) as usize;
        let bottom = (y + radius + 1).min(height) as usize;
        let area = u64::from((right - left) as u32 * (bottom - top) as u32);
        let sum = integral[bottom * stride + right]
            .saturating_sub(integral[top * stride + right])
            .saturating_sub(integral[bottom * stride + left])
            .saturating_add(integral[top * stride + left]);
        let threshold = sum as f32 / area as f32 - offset;
        if f32::from(gray.get_pixel(x, y)[0]) > threshold {
            Luma([255])
        } else {
            Luma([0])
        }
    })
}

fn enhanced_color(
    image: &DynamicImage,
    settings: &EnhancementSettings,
) -> RgbImage {
    let mut rgb = image.to_rgb8();
    apply_brightness_and_contrast(
        &mut rgb,
        settings.color_brightness,
        settings.color_contrast,
    );
    if settings.denoise_strength > 0.0 {
        rgb = blend_with_box_blur(&rgb, settings.denoise_strength);
    }
    if settings.sharpening_strength > 0.0 {
        rgb = sharpen(&rgb, settings.sharpening_strength);
    }
    rgb
}

fn magic_color(
    image: &DynamicImage,
    settings: &EnhancementSettings,
) -> RgbImage {
    let source = image.to_rgb8();
    let (width, height) = source.dimensions();
    let mut output = RgbImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let pixel = source.get_pixel(x, y);
            let luminance = 0.299 * f32::from(pixel[0])
                + 0.587 * f32::from(pixel[1])
                + 0.114 * f32::from(pixel[2]);
            let contrasted =
                ((luminance - 128.0) * settings.magic_local_contrast + 128.0)
                    .clamp(0.0, 255.0);
            let factor = if luminance > 0.0 {
                contrasted / luminance
            } else {
                1.0
            };
            let average = (f32::from(pixel[0])
                + f32::from(pixel[1])
                + f32::from(pixel[2]))
                / 3.0;
            let channels = [pixel[0], pixel[1], pixel[2]].map(|channel| {
                let saturated = average
                    + (f32::from(channel) - average)
                        * settings.magic_saturation;
                (saturated * factor).clamp(0.0, 255.0).round() as u8
            });
            output.put_pixel(x, y, Rgb(channels));
        }
    }
    output
}

fn apply_brightness_and_contrast(
    image: &mut RgbImage,
    brightness: f32,
    contrast: f32,
) {
    for pixel in image.pixels_mut() {
        for channel in &mut pixel.0 {
            let value =
                (f32::from(*channel) - 128.0) * contrast + 128.0 + brightness;
            *channel = value.clamp(0.0, 255.0).round() as u8;
        }
    }
}

fn blend_with_box_blur(image: &RgbImage, strength: f32) -> RgbImage {
    let (width, height) = image.dimensions();
    let mut output = image.clone();
    for y in 0..height {
        for x in 0..width {
            let mut totals = [0_u32; 3];
            let mut count = 0_u32;
            for sample_y in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for sample_x in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let sample = image.get_pixel(sample_x, sample_y);
                    for channel in 0..3 {
                        totals[channel] += u32::from(sample[channel]);
                    }
                    count += 1;
                }
            }
            let original = image.get_pixel(x, y);
            let channels = [0, 1, 2].map(|channel| {
                let blurred = totals[channel] as f32 / count as f32;
                (f32::from(original[channel]) * (1.0 - strength)
                    + blurred * strength)
                    .round()
                    .clamp(0.0, 255.0) as u8
            });
            output.put_pixel(x, y, Rgb(channels));
        }
    }
    output
}

fn sharpen(image: &RgbImage, strength: f32) -> RgbImage {
    let (width, height) = image.dimensions();
    let mut output = image.clone();
    for y in 0..height {
        for x in 0..width {
            let mut totals = [0_u32; 3];
            let mut count = 0_u32;
            for sample_y in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for sample_x in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    if sample_x == x && sample_y == y {
                        continue;
                    }
                    let sample = image.get_pixel(sample_x, sample_y);
                    for channel in 0..3 {
                        totals[channel] += u32::from(sample[channel]);
                    }
                    count += 1;
                }
            }
            let original = image.get_pixel(x, y);
            let channels = [0, 1, 2].map(|channel| {
                let average = if count == 0 {
                    f32::from(original[channel])
                } else {
                    totals[channel] as f32 / count as f32
                };
                (f32::from(original[channel])
                    + (f32::from(original[channel]) - average) * strength)
                    .round()
                    .clamp(0.0, 255.0) as u8
            });
            output.put_pixel(x, y, Rgb(channels));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, Rgb, RgbImage};

    use super::*;
    use crate::{EnhancementPreset, EnhancementSettings};

    #[test]
    fn invalid_block_size_is_rejected() {
        let settings = EnhancementSettings {
            adaptive_block_size: 4,
            ..EnhancementSettings::default()
        };
        assert!(validate_settings(&settings).is_err());
    }

    #[test]
    fn enhancement_modes_return_expected_pixel_types() {
        let mut image = RgbImage::new(8, 8);
        image.put_pixel(3, 3, Rgb([255, 10, 20]));
        let image = DynamicImage::ImageRgb8(image);

        let black_and_white = EnhancementSettings {
            preset: EnhancementPreset::AdaptiveBlackAndWhite,
            ..EnhancementSettings::default()
        };
        assert!(matches!(
            apply(image.clone(), &black_and_white).unwrap(),
            DynamicImage::ImageLuma8(_)
        ));

        let color = EnhancementSettings {
            preset: EnhancementPreset::MagicColor,
            ..EnhancementSettings::default()
        };
        assert!(matches!(
            apply(image, &color).unwrap(),
            DynamicImage::ImageRgb8(_)
        ));
    }
}
