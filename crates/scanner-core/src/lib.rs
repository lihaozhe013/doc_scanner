pub mod backend;
pub mod effects;
pub mod error;
pub mod export;
pub mod geometry;
pub mod model;
pub mod pipeline;
pub mod session;

pub use error::CoreError;
pub use export::{
    CollisionPolicy, ExportResult, export_image, next_available_output_path,
};
pub use geometry::{
    Bounds, CoordinateMapper, GeometryError, ImagePoint, OutputDimensions,
    Point, Quadrilateral, ScreenPoint, ValidatedQuadrilateral,
};
pub use model::{
    CURRENT_SESSION_SCHEMA, CanonicalOrientation, EditState, EnhancementPreset,
    EnhancementSettings, ImageId, ImageMetadata, LoadedSource, OutputFormat,
    OutputSettings, QueueStatus, RasterFormat, SessionDocument, SessionItem,
    SourceImage, display_name_for_path,
};
pub use pipeline::{
    DEFAULT_PREVIEW_MAX_DIMENSION, ProcessingMode, ProcessingResult,
    load_image, process_image,
};
pub use session::{decode_session, encode_session};
