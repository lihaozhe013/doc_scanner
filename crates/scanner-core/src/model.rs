use std::path::{Path, PathBuf};

use image::DynamicImage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::geometry::{ImagePoint, Quadrilateral};

pub const CURRENT_SESSION_SCHEMA: u32 = 1;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct ImageId(Uuid);

impl ImageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for ImageId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanonicalOrientation {
    Normalized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RasterFormat {
    Jpeg,
    Png,
    Bmp,
    Tiff,
}

impl RasterFormat {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "bmp" => Some(Self::Bmp),
            "tif" | "tiff" => Some(Self::Tiff),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::Bmp => "BMP",
            Self::Tiff => "TIFF",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub format: RasterFormat,
    pub orientation: CanonicalOrientation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceImage {
    pub id: ImageId,
    pub path: PathBuf,
    pub display_name: String,
    pub metadata: ImageMetadata,
}

impl SourceImage {
    pub fn new(id: ImageId, path: PathBuf, metadata: ImageMetadata) -> Self {
        let display_name = display_name_for_path(&path);
        Self {
            id,
            path,
            display_name,
            metadata,
        }
    }
}

#[derive(Debug)]
pub struct LoadedSource {
    pub source: SourceImage,
    pub image: DynamicImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnhancementPreset {
    Original,
    AdaptiveBlackAndWhite,
    EnhancedColor,
    MagicColor,
}

impl EnhancementPreset {
    pub const ALL: [Self; 4] = [
        Self::Original,
        Self::AdaptiveBlackAndWhite,
        Self::EnhancedColor,
        Self::MagicColor,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::AdaptiveBlackAndWhite => "Adaptive black and white",
            Self::EnhancedColor => "Enhanced color",
            Self::MagicColor => "Magic color",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancementSettings {
    pub preset: EnhancementPreset,
    pub adaptive_block_size: u32,
    pub adaptive_threshold_offset: f32,
    pub color_brightness: f32,
    pub color_contrast: f32,
    pub denoise_strength: f32,
    pub sharpening_strength: f32,
    pub magic_local_contrast: f32,
    pub magic_saturation: f32,
}

impl Default for EnhancementSettings {
    fn default() -> Self {
        Self {
            preset: EnhancementPreset::Original,
            adaptive_block_size: 31,
            adaptive_threshold_offset: 7.0,
            color_brightness: 0.0,
            color_contrast: 1.0,
            denoise_strength: 0.0,
            sharpening_strength: 0.35,
            magic_local_contrast: 1.15,
            magic_saturation: 1.12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Jpeg,
    Png,
    Bmp,
    Tiff,
}

impl OutputFormat {
    pub const ALL: [Self; 4] = [Self::Jpeg, Self::Png, Self::Bmp, Self::Tiff];

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::Bmp => "BMP",
            Self::Tiff => "TIFF",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputSettings {
    pub format: OutputFormat,
    pub jpeg_quality: u8,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            format: OutputFormat::Png,
            jpeg_quality: 92,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditState {
    pub quadrilateral: Quadrilateral,
    pub enhancement: EnhancementSettings,
    pub output: OutputSettings,
    pub revision: u64,
}

impl EditState {
    pub fn new() -> Self {
        Self {
            quadrilateral: Quadrilateral::full_image(),
            enhancement: EnhancementSettings::default(),
            output: OutputSettings::default(),
            revision: 0,
        }
    }

    pub fn for_dimensions(_width: u32, _height: u32) -> Self {
        Self::new()
    }

    pub fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    pub fn set_preset(&mut self, preset: EnhancementPreset) {
        if self.enhancement.preset != preset {
            self.enhancement.preset = preset;
            self.bump_revision();
        }
    }
}

impl Default for EditState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStatus {
    NotStarted,
    Loading,
    Ready,
    Previewing,
    Queued,
    Processing,
    Completed,
    Skipped,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionItem {
    pub source: SourceImage,
    pub edit: EditState,
    pub status: QueueStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDocument {
    pub schema_version: u32,
    pub items: Vec<SessionItem>,
}

impl Default for SessionDocument {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SESSION_SCHEMA,
            items: Vec::new(),
        }
    }
}

pub fn display_name_for_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Untitled image")
        .to_owned()
}

pub fn normalized_point(point: ImagePoint) -> ImagePoint {
    ImagePoint::new(point.x.clamp(0.0, 1.0), point.y.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_state_round_trips_through_json() {
        let mut edit = EditState::default();
        edit.set_preset(EnhancementPreset::MagicColor);
        let json = serde_json::to_string(&edit).unwrap();
        let restored: EditState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, edit);
    }

    #[test]
    fn path_display_name_does_not_include_parent_directories() {
        let path = PathBuf::from("/private/source/page.png");
        assert_eq!(display_name_for_path(&path), "page.png");
    }
}
