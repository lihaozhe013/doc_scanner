use image::{DynamicImage, Rgba, RgbaImage};

use crate::{CoreError, OutputDimensions, Point, ValidatedQuadrilateral};

pub(crate) fn perspective_warp(
    source: &DynamicImage,
    quadrilateral: &ValidatedQuadrilateral,
    output: OutputDimensions,
) -> Result<DynamicImage, CoreError> {
    if output.width == 0 || output.height == 0 {
        return Err(CoreError::InvalidOutputDimensions {
            width: output.width,
            height: output.height,
        });
    }

    let source = source.to_rgba8();
    let source_width = source.width();
    let source_height = source.height();
    if source_width == 0 || source_height == 0 {
        return Err(CoreError::InvalidDimensions {
            width: source_width,
            height: source_height,
        });
    }

    let source_points = quadrilateral.points.map(|point| {
        Point::new(
            point.x * f64::from(source_width.saturating_sub(1)),
            point.y * f64::from(source_height.saturating_sub(1)),
        )
    });
    let destination_points = [
        Point::new(0.0, 0.0),
        Point::new(f64::from(output.width.saturating_sub(1)), 0.0),
        Point::new(
            f64::from(output.width.saturating_sub(1)),
            f64::from(output.height.saturating_sub(1)),
        ),
        Point::new(0.0, f64::from(output.height.saturating_sub(1))),
    ];
    let homography = solve_homography(destination_points, source_points)
        .ok_or(CoreError::InvalidOutputDimensions {
            width: output.width,
            height: output.height,
        })?;

    let mut warped = RgbaImage::new(output.width, output.height);
    for y in 0..output.height {
        for x in 0..output.width {
            let source_point = project(homography, f64::from(x), f64::from(y));
            let pixel = source_point
                .and_then(|point| bilinear_sample(&source, point.x, point.y))
                .unwrap_or(Rgba([255, 255, 255, 0]));
            warped.put_pixel(x, y, pixel);
        }
    }

    Ok(DynamicImage::ImageRgba8(warped))
}

fn solve_homography(from: [Point; 4], to: [Point; 4]) -> Option<[f64; 9]> {
    let mut matrix = [[0.0_f64; 9]; 8];
    for (index, (from, to)) in from.into_iter().zip(to).enumerate() {
        let row = index * 2;
        let x = from.x;
        let y = from.y;
        let u = to.x;
        let v = to.y;

        matrix[row] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
        matrix[row + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
    }

    for pivot in 0..8 {
        let pivot_row = (pivot..8).max_by(|left, right| {
            matrix[*left][pivot]
                .abs()
                .partial_cmp(&matrix[*right][pivot].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if matrix[pivot_row][pivot].abs() <= 1.0e-12 {
            return None;
        }
        matrix.swap(pivot, pivot_row);

        let divisor = matrix[pivot][pivot];
        for value in matrix[pivot][pivot..=8].iter_mut() {
            *value /= divisor;
        }

        let pivot_values = matrix[pivot];
        for (row_index, row_values) in matrix.iter_mut().enumerate().take(8) {
            if row_index == pivot {
                continue;
            }
            let factor = row_values[pivot];
            for (value, pivot_value) in row_values[pivot..=8]
                .iter_mut()
                .zip(pivot_values[pivot..=8].iter())
            {
                *value -= factor * pivot_value;
            }
        }
    }

    let mut homography = [0.0_f64; 9];
    homography[..8].copy_from_slice(&matrix.map(|row| row[8]));
    homography[8] = 1.0;
    if homography.iter().all(|value| value.is_finite()) {
        Some(homography)
    } else {
        None
    }
}

fn project(homography: [f64; 9], x: f64, y: f64) -> Option<Point> {
    let denominator =
        homography[6].mul_add(x, homography[7] * y) + homography[8];
    if !denominator.is_finite() || denominator.abs() <= 1.0e-12 {
        return None;
    }
    let projected_x = (homography[0].mul_add(x, homography[1] * y)
        + homography[2])
        / denominator;
    let projected_y = (homography[3].mul_add(x, homography[4] * y)
        + homography[5])
        / denominator;
    if projected_x.is_finite() && projected_y.is_finite() {
        Some(Point::new(projected_x, projected_y))
    } else {
        None
    }
}

fn bilinear_sample(image: &RgbaImage, x: f64, y: f64) -> Option<Rgba<u8>> {
    let width = f64::from(image.width());
    let height = f64::from(image.height());
    if x < 0.0 || y < 0.0 || x > width - 1.0 || y > height - 1.0 {
        return None;
    }

    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(image.width() - 1);
    let y1 = (y0 + 1).min(image.height() - 1);
    let dx = x - f64::from(x0);
    let dy = y - f64::from(y0);
    let top_left = image.get_pixel(x0, y0);
    let top_right = image.get_pixel(x1, y0);
    let bottom_left = image.get_pixel(x0, y1);
    let bottom_right = image.get_pixel(x1, y1);
    let channels = [0, 1, 2, 3].map(|channel| {
        let top = f64::from(top_left[channel])
            + (f64::from(top_right[channel]) - f64::from(top_left[channel]))
                * dx;
        let bottom = f64::from(bottom_left[channel])
            + (f64::from(bottom_right[channel])
                - f64::from(bottom_left[channel]))
                * dx;
        (top + (bottom - top) * dy).round().clamp(0.0, 255.0) as u8
    });
    Some(Rgba(channels))
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, Rgba, RgbaImage};

    use super::*;
    use crate::{Bounds, Quadrilateral};

    #[test]
    fn identity_warp_preserves_dimensions_and_corners() {
        let mut image = RgbaImage::new(8, 6);
        image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        image.put_pixel(7, 5, Rgba([0, 255, 0, 255]));
        let image = DynamicImage::ImageRgba8(image);
        let quad = Quadrilateral::full_image()
            .validate(Bounds::unit())
            .unwrap();

        let result = perspective_warp(
            &image,
            &quad,
            OutputDimensions {
                width: 8,
                height: 6,
            },
        )
        .unwrap()
        .to_rgba8();
        assert_eq!(result.dimensions(), (8, 6));
        assert_eq!(result.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(result.get_pixel(7, 5), &Rgba([0, 255, 0, 255]));
    }
}
