use std::marker::PhantomData;

use opencv::core::Mat;

/// Marker for the optional OpenCV integration boundary.
///
/// OpenCV types stay in this module so application crates never depend on the
/// generated binding surface. Processing currently defaults to the portable
/// native backend; parity work can replace its implementation behind this
/// boundary without changing persisted edit state.
pub struct OpenCvAdapter {
    _mat_type: PhantomData<Mat>,
}

impl OpenCvAdapter {
    pub const fn new() -> Self {
        Self {
            _mat_type: PhantomData,
        }
    }

    pub const fn backend_name(&self) -> &'static str {
        "opencv"
    }
}

impl Default for OpenCvAdapter {
    fn default() -> Self {
        Self::new()
    }
}
