use sentinel::domain::eyes::EyesService;
use tempfile::TempDir;

// ===========================================================================
// Helper: create a minimal 1x1 PNG
// ===========================================================================

fn create_minimal_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img = ImageBuffer::from_pixel(100, 100, Rgba([255u8, 0, 0, 255]));
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )
    .unwrap();
    buf
}

fn create_wide_png(width: u32, height: u32) -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img = ImageBuffer::from_pixel(width, height, Rgba([0u8, 128, 255, 255]));
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )
    .unwrap();
    buf
}

// ===========================================================================
// Tests: EyesService::new
// ===========================================================================

#[test]
fn test_new_creates_capture_directory() {
    let dir = TempDir::new().unwrap();
    let capture_dir = dir.path().join("captures");
    assert!(!capture_dir.exists());

    let _svc = EyesService::new(capture_dir.clone());
    assert!(capture_dir.exists());
}

#[test]
fn test_new_with_existing_directory() {
    let dir = TempDir::new().unwrap();
    // Should not panic even if directory already exists
    let _svc = EyesService::new(dir.path().to_path_buf());
}

// ===========================================================================
// Tests: save_capture
// ===========================================================================

#[test]
fn test_save_capture_with_name() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    let png_data = create_minimal_png();
    let result = svc.save_capture(&png_data, Some("test")).unwrap();

    assert!(result.image_path.ends_with("test.png"));
    assert!(std::path::Path::new(&result.image_path).exists());
    assert_eq!(result.width, 100);
    assert_eq!(result.height, 100);
    assert!(result.timestamp > 0);
}

#[test]
fn test_save_capture_without_name() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    let png_data = create_minimal_png();
    let result = svc.save_capture(&png_data, None).unwrap();

    assert!(result.image_path.contains("capture_"));
    assert!(result.image_path.ends_with(".png"));
    assert!(std::path::Path::new(&result.image_path).exists());
}

#[test]
fn test_save_capture_writes_correct_bytes() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    let png_data = create_minimal_png();
    let result = svc.save_capture(&png_data, Some("verify")).unwrap();

    let written = std::fs::read(&result.image_path).unwrap();
    assert_eq!(written, png_data);
}

#[test]
fn test_save_capture_reads_dimensions() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    let png_data = create_wide_png(320, 240);
    let result = svc.save_capture(&png_data, Some("sized")).unwrap();

    assert_eq!(result.width, 320);
    assert_eq!(result.height, 240);
}

#[test]
fn test_save_capture_short_data_returns_zero_dimensions() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    // Data too short for a valid PNG IHDR
    let short_data = vec![0u8; 10];
    let result = svc.save_capture(&short_data, Some("short")).unwrap();

    assert_eq!(result.width, 0);
    assert_eq!(result.height, 0);
}

// ===========================================================================
// Tests: list_captures
// ===========================================================================

#[test]
fn test_list_captures_empty() {
    let dir = TempDir::new().unwrap();
    let svc = EyesService::new(dir.path().to_path_buf());
    assert_eq!(svc.list_captures().len(), 0);
}

#[test]
fn test_list_captures_after_saves() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    svc.save_capture(&create_minimal_png(), Some("a")).unwrap();
    svc.save_capture(&create_minimal_png(), Some("b")).unwrap();
    svc.save_capture(&create_minimal_png(), Some("c")).unwrap();

    assert_eq!(svc.list_captures().len(), 3);
}

// ===========================================================================
// Tests: latest_capture
// ===========================================================================

#[test]
fn test_latest_capture_none_when_empty() {
    let dir = TempDir::new().unwrap();
    let svc = EyesService::new(dir.path().to_path_buf());
    assert!(svc.latest_capture().is_none());
}

#[test]
fn test_latest_capture_returns_last() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    svc.save_capture(&create_minimal_png(), Some("first")).unwrap();
    svc.save_capture(&create_minimal_png(), Some("second")).unwrap();
    svc.save_capture(&create_minimal_png(), Some("third")).unwrap();

    let latest = svc.latest_capture().unwrap();
    assert!(latest.image_path.contains("third"));
}

// ===========================================================================
// Tests: status
// ===========================================================================

#[test]
fn test_status_empty() {
    let dir = TempDir::new().unwrap();
    let svc = EyesService::new(dir.path().to_path_buf());
    let status = svc.status();

    assert!(status.running);
    assert_eq!(status.port, 0);
    assert_eq!(status.captures, 0);
    assert!(status.last_capture.is_none());
}

#[test]
fn test_status_after_captures() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    svc.save_capture(&create_minimal_png(), Some("one")).unwrap();
    svc.save_capture(&create_minimal_png(), Some("two")).unwrap();

    let status = svc.status();
    assert_eq!(status.captures, 2);
    assert!(status.last_capture.is_some());
    assert!(status.last_capture.unwrap().contains("two"));
}

// ===========================================================================
// Tests: clear
// ===========================================================================

#[test]
fn test_clear_empties_captures() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    svc.save_capture(&create_minimal_png(), Some("x")).unwrap();
    svc.save_capture(&create_minimal_png(), Some("y")).unwrap();
    assert_eq!(svc.list_captures().len(), 2);

    svc.clear();
    assert_eq!(svc.list_captures().len(), 0);
    assert!(svc.latest_capture().is_none());
}

#[test]
fn test_clear_on_empty_is_noop() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());
    svc.clear(); // Should not panic
    assert_eq!(svc.list_captures().len(), 0);
}

// ===========================================================================
// Tests: get_image_dimensions (via save_capture)
// ===========================================================================

#[test]
fn test_dimensions_various_sizes() {
    let dir = TempDir::new().unwrap();
    let mut svc = EyesService::new(dir.path().to_path_buf());

    for (w, h) in [(1, 1), (640, 480), (1920, 1080)] {
        let png = create_wide_png(w, h);
        let result = svc.save_capture(&png, Some(&format!("{w}x{h}"))).unwrap();
        assert_eq!(result.width, w, "Width mismatch for {w}x{h}");
        assert_eq!(result.height, h, "Height mismatch for {w}x{h}");
    }
}

// ===========================================================================
// Tests: resize_image
// ===========================================================================

#[test]
fn test_resize_image_shrinks_wide_image() {
    let png = create_wide_png(800, 600);
    let resized = EyesService::resize_image(&png, 400).unwrap();

    // Decode the resized image and check dimensions
    let img = image::load_from_memory(&resized).unwrap();
    let (w, h) = image::GenericImageView::dimensions(&img);
    assert_eq!(w, 400);
    // Height should be proportionally scaled: 600 * (400/800) = 300
    assert_eq!(h, 300);
}

#[test]
fn test_resize_image_no_op_when_within_max() {
    let png = create_wide_png(200, 150);
    let resized = EyesService::resize_image(&png, 400).unwrap();

    // Should return the original data unchanged
    assert_eq!(resized, png);
}

#[test]
fn test_resize_image_exact_max_width() {
    let png = create_wide_png(400, 300);
    let resized = EyesService::resize_image(&png, 400).unwrap();

    // Width equals max_width, should not resize
    assert_eq!(resized, png);
}

#[test]
fn test_resize_image_invalid_data() {
    let bad_data = vec![0u8; 10];
    let result = EyesService::resize_image(&bad_data, 400);
    assert!(result.is_err());
}

// ===========================================================================
// Tests: eyes_router (basic smoke test)
// ===========================================================================

#[test]
fn test_eyes_router_builds() {
    // Verify the router can be constructed without panic
    let _router = sentinel::domain::eyes::eyes_router();
}
