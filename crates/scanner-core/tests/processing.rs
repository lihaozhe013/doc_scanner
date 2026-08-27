use std::fs;

use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use scanner_core::{
    CollisionPolicy, CoreError, EditState, OutputFormat, export_image,
    load_image,
};

#[test]
fn export_reopens_output_without_modifying_source() {
    let directory = temporary_directory();
    let source_path = directory.join("page.png");
    let mut source = RgbImage::new(24, 16);
    for (x, y, pixel) in source.enumerate_pixels_mut() {
        *pixel = Rgb([(x * 7) as u8, (y * 11) as u8, 80]);
    }
    DynamicImage::ImageRgb8(source)
        .save_with_format(&source_path, ImageFormat::Png)
        .unwrap();
    let source_bytes = fs::read(&source_path).unwrap();
    let output_directory = directory.join("exports");

    let mut edit = EditState::default();
    edit.output.format = OutputFormat::Png;
    let first = export_image(
        &source_path,
        &edit,
        &output_directory,
        CollisionPolicy::AutoRename,
    )
    .unwrap();
    let second = export_image(
        &source_path,
        &edit,
        &output_directory,
        CollisionPolicy::AutoRename,
    )
    .unwrap();

    assert_eq!(fs::read(&source_path).unwrap(), source_bytes);
    assert_eq!((first.width, first.height), (24, 16));
    assert_eq!(
        second.path.file_name().unwrap().to_str().unwrap(),
        "page (1).png"
    );
    assert_eq!(load_image(&first.path).unwrap().source.metadata.width, 24);

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn invalid_input_fails_before_creating_output() {
    let directory = temporary_directory();
    let source_path = directory.join("page.gif");
    let output_directory = directory.join("exports");
    fs::write(&source_path, b"not a supported input").unwrap();

    let result = export_image(
        &source_path,
        &EditState::default(),
        &output_directory,
        CollisionPolicy::AutoRename,
    );
    assert!(matches!(result, Err(CoreError::UnsupportedFormat { .. })));
    assert!(!output_directory.join("page.png").exists());

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn invalid_quadrilateral_does_not_write_output() {
    let directory = temporary_directory();
    let source_path = directory.join("page.png");
    let output_directory = directory.join("exports");
    DynamicImage::ImageRgb8(RgbImage::new(16, 16))
        .save_with_format(&source_path, ImageFormat::Png)
        .unwrap();
    let mut edit = EditState::default();
    edit.quadrilateral.points[1].x = edit.quadrilateral.points[0].x;

    let result = export_image(
        &source_path,
        &edit,
        &output_directory,
        CollisionPolicy::AutoRename,
    );
    assert!(matches!(result, Err(CoreError::Geometry(_))));
    assert!(!output_directory.join("page.png").exists());

    let _ = fs::remove_dir_all(directory);
}

fn temporary_directory() -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "scanner-integration-{}",
        scanner_core::ImageId::new().as_uuid()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}
