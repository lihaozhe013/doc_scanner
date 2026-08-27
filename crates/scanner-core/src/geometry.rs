use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MIN_EDGE_LENGTH: f64 = 1.0e-5;
pub const MIN_QUADRILATERAL_AREA: f64 = 1.0e-8;
pub const MAX_IMAGE_DIMENSION: u32 = 100_000;
pub const MAX_IMAGE_PIXELS: u64 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub type ImagePoint = Point;

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    pub fn distance_squared(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.mul_add(dx, dy * dy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}

impl Bounds {
    pub const fn unit() -> Self {
        Self {
            min: Point::new(0.0, 0.0),
            max: Point::new(1.0, 1.0),
        }
    }

    pub fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quadrilateral {
    pub points: [Point; 4],
}

impl Quadrilateral {
    pub const fn new(points: [Point; 4]) -> Self {
        Self { points }
    }

    pub const fn full_image() -> Self {
        Self::new([
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        ])
    }

    pub fn validate(
        self,
        bounds: Bounds,
    ) -> Result<ValidatedQuadrilateral, GeometryError> {
        if !bounds.min.is_finite() || !bounds.max.is_finite() {
            return Err(GeometryError::InvalidBounds);
        }
        if bounds.min.x >= bounds.max.x || bounds.min.y >= bounds.max.y {
            return Err(GeometryError::InvalidBounds);
        }
        if self.points.iter().any(|point| !point.is_finite()) {
            return Err(GeometryError::NonFinitePoint);
        }
        if self.points.iter().any(|&point| !bounds.contains(point)) {
            return Err(GeometryError::OutsideBounds);
        }

        for first in 0..self.points.len() {
            for second in (first + 1)..self.points.len() {
                if self.points[first].distance_squared(self.points[second])
                    <= MIN_EDGE_LENGTH * MIN_EDGE_LENGTH
                {
                    return Err(GeometryError::DuplicatePoint);
                }
            }
        }

        if segments_intersect(
            self.points[0],
            self.points[1],
            self.points[2],
            self.points[3],
        ) || segments_intersect(
            self.points[1],
            self.points[2],
            self.points[3],
            self.points[0],
        ) {
            return Err(GeometryError::SelfIntersecting);
        }

        let mut cross_sign = 0.0;
        for index in 0..4 {
            let a = self.points[index];
            let b = self.points[(index + 1) % 4];
            let c = self.points[(index + 2) % 4];
            let cross = cross(a, b, c);
            if cross.abs() <= MIN_QUADRILATERAL_AREA {
                return Err(GeometryError::Collinear);
            }
            if cross_sign == 0.0 {
                cross_sign = cross.signum();
            } else if cross.signum() != cross_sign {
                return Err(GeometryError::NonConvex);
            }
        }

        let area = signed_area(self.points).abs();
        if !area.is_finite() || area <= MIN_QUADRILATERAL_AREA {
            return Err(GeometryError::NearZeroArea);
        }

        for index in 0..4 {
            let next = (index + 1) % 4;
            if self.points[index].distance_squared(self.points[next])
                <= MIN_EDGE_LENGTH * MIN_EDGE_LENGTH
            {
                return Err(GeometryError::EdgeTooShort);
            }
        }

        Ok(ValidatedQuadrilateral {
            points: self.points,
            area,
        })
    }

    pub fn ordered(self) -> Result<Self, GeometryError> {
        if self.points.iter().any(|point| !point.is_finite()) {
            return Err(GeometryError::NonFinitePoint);
        }

        let centroid = Point::new(
            self.points.iter().map(|point| point.x).sum::<f64>() / 4.0,
            self.points.iter().map(|point| point.y).sum::<f64>() / 4.0,
        );
        let mut points = self.points;
        points.sort_by(|left, right| {
            let left_angle = (left.y - centroid.y).atan2(left.x - centroid.x);
            let right_angle =
                (right.y - centroid.y).atan2(right.x - centroid.x);
            left_angle
                .partial_cmp(&right_angle)
                .unwrap_or(Ordering::Equal)
        });

        let start = points
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                let left_key = left.x + left.y;
                let right_key = right.x + right.y;
                left_key
                    .partial_cmp(&right_key)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| {
                        left.x.partial_cmp(&right.x).unwrap_or(Ordering::Equal)
                    })
            })
            .map(|(index, _)| index)
            .ok_or(GeometryError::NonFinitePoint)?;
        points.rotate_left(start);

        let ordered = Self::new(points);
        if signed_area(ordered.points) < 0.0 {
            Ok(Self::new([
                ordered.points[0],
                ordered.points[3],
                ordered.points[2],
                ordered.points[1],
            ]))
        } else {
            Ok(ordered)
        }
    }

    pub fn validate_and_order(
        self,
        bounds: Bounds,
    ) -> Result<ValidatedQuadrilateral, GeometryError> {
        self.ordered()?.validate(bounds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidatedQuadrilateral {
    pub points: [Point; 4],
    pub area: f64,
}

impl ValidatedQuadrilateral {
    pub fn output_dimensions(
        self,
        image_width: u32,
        image_height: u32,
    ) -> Result<OutputDimensions, GeometryError> {
        if image_width == 0 || image_height == 0 {
            return Err(GeometryError::InvalidImageDimensions);
        }

        let horizontal_top =
            self.points[0].distance_squared(self.points[1]).sqrt();
        let horizontal_bottom =
            self.points[3].distance_squared(self.points[2]).sqrt();
        let vertical_left =
            self.points[0].distance_squared(self.points[3]).sqrt();
        let vertical_right =
            self.points[1].distance_squared(self.points[2]).sqrt();
        let width = ((horizontal_top + horizontal_bottom) / 2.0
            * f64::from(image_width))
        .round();
        let height = ((vertical_left + vertical_right) / 2.0
            * f64::from(image_height))
        .round();

        if !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
        {
            return Err(GeometryError::InvalidOutputDimensions);
        }

        let width = width as u64;
        let height = height as u64;
        if width > u64::from(MAX_IMAGE_DIMENSION)
            || height > u64::from(MAX_IMAGE_DIMENSION)
            || width.saturating_mul(height) > MAX_IMAGE_PIXELS
        {
            return Err(GeometryError::OutputTooLarge);
        }

        Ok(OutputDimensions {
            width: width as u32,
            height: height as u32,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: f64,
    pub y: f64,
}

impl ScreenPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateMapper {
    pub origin: ScreenPoint,
    pub size: ScreenPoint,
}

impl CoordinateMapper {
    pub const fn new(origin: ScreenPoint, size: ScreenPoint) -> Self {
        Self { origin, size }
    }

    pub fn image_to_screen(self, point: Point) -> ScreenPoint {
        ScreenPoint::new(
            self.origin.x + point.x * self.size.x,
            self.origin.y + point.y * self.size.y,
        )
    }

    pub fn screen_to_image(self, point: ScreenPoint) -> Point {
        Point::new(
            (point.x - self.origin.x) / self.size.x,
            (point.y - self.origin.y) / self.size.y,
        )
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    #[error("bounds are invalid")]
    InvalidBounds,
    #[error("a point is not finite")]
    NonFinitePoint,
    #[error("a point is outside the image bounds")]
    OutsideBounds,
    #[error("quadrilateral points must be distinct")]
    DuplicatePoint,
    #[error("quadrilateral edges intersect")]
    SelfIntersecting,
    #[error("quadrilateral is not convex")]
    NonConvex,
    #[error("three quadrilateral points are collinear")]
    Collinear,
    #[error("quadrilateral area is too small")]
    NearZeroArea,
    #[error("quadrilateral edge is too short")]
    EdgeTooShort,
    #[error("image dimensions are invalid")]
    InvalidImageDimensions,
    #[error("output dimensions are invalid")]
    InvalidOutputDimensions,
    #[error("output dimensions exceed the safety limit")]
    OutputTooLarge,
}

fn signed_area(points: [Point; 4]) -> f64 {
    let sum = (0..4)
        .map(|index| {
            let current = points[index];
            let next = points[(index + 1) % 4];
            current.x * next.y - next.x * current.y
        })
        .sum::<f64>();
    sum / 2.0
}

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let ab_c = cross(a, b, c);
    let ab_d = cross(a, b, d);
    let cd_a = cross(c, d, a);
    let cd_b = cross(c, d, b);
    let epsilon = MIN_QUADRILATERAL_AREA;

    if ab_c.abs() <= epsilon && on_segment(a, b, c)
        || ab_d.abs() <= epsilon && on_segment(a, b, d)
        || cd_a.abs() <= epsilon && on_segment(c, d, a)
        || cd_b.abs() <= epsilon && on_segment(c, d, b)
    {
        return true;
    }

    ab_c.signum() != ab_d.signum() && cd_a.signum() != cd_b.signum()
}

fn on_segment(a: Point, b: Point, point: Point) -> bool {
    point.x >= a.x.min(b.x)
        && point.x <= a.x.max(b.x)
        && point.y >= a.y.min(b.y)
        && point.y <= a.y.max(b.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_image_quad_is_valid() {
        let validated = Quadrilateral::full_image()
            .validate(Bounds::unit())
            .unwrap();
        assert_eq!(
            validated.output_dimensions(1200, 800).unwrap(),
            OutputDimensions {
                width: 1200,
                height: 800,
            }
        );
    }

    #[test]
    fn ordering_is_deterministic() {
        let quad = Quadrilateral::new([
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 0.0),
        ]);
        assert_eq!(quad.ordered().unwrap(), Quadrilateral::full_image());
    }

    #[test]
    fn invalid_quadrilateral_is_rejected() {
        let crossing = Quadrilateral::new([
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
        ]);
        assert_eq!(
            crossing.validate(Bounds::unit()),
            Err(GeometryError::SelfIntersecting)
        );
    }

    #[test]
    fn mapper_round_trips() {
        let mapper = CoordinateMapper::new(
            ScreenPoint::new(20.0, 30.0),
            ScreenPoint::new(800.0, 600.0),
        );
        let image_point = Point::new(0.25, 0.75);
        let screen_point = mapper.image_to_screen(image_point);
        let round_trip = mapper.screen_to_image(screen_point);
        assert!((round_trip.x - image_point.x).abs() < f64::EPSILON);
        assert!((round_trip.y - image_point.y).abs() < f64::EPSILON);
    }
}
