use serde::Serialize;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CaptureResult {
    pub image_path: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EyesStatus {
    pub running: bool,
    pub port: u16,
    pub captures: usize,
    pub last_capture: Option<String>,
}

// ---------------------------------------------------------------------------
// EyesService — capture storage and image processing
// ---------------------------------------------------------------------------

pub struct EyesService {
    capture_dir: PathBuf,
    captures: Vec<CaptureResult>,
}

impl EyesService {
    pub fn new(capture_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&capture_dir).ok();
        Self {
            capture_dir,
            captures: Vec::new(),
        }
    }

    /// Save a PNG image from bytes to the capture directory.
    pub fn save_capture(
        &mut self,
        png_data: &[u8],
        name: Option<&str>,
    ) -> anyhow::Result<CaptureResult> {
        let filename = name
            .map(|n| format!("{n}.png"))
            .unwrap_or_else(|| {
                format!("capture_{}.png", chrono::Utc::now().timestamp_millis())
            });

        let path = self.capture_dir.join(&filename);
        std::fs::write(&path, png_data)?;

        let (width, height) = Self::get_image_dimensions(png_data);

        let result = CaptureResult {
            image_path: path.to_string_lossy().to_string(),
            width,
            height,
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.captures.push(result.clone());
        Ok(result)
    }

    /// Get dimensions from PNG bytes by reading the IHDR chunk directly.
    ///
    /// PNG layout: 8-byte signature, then IHDR chunk whose data starts at
    /// byte 16 with 4 bytes width followed by 4 bytes height (big-endian).
    fn get_image_dimensions(data: &[u8]) -> (u32, u32) {
        if data.len() >= 24 {
            let width =
                u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
            let height =
                u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
            (width, height)
        } else {
            (0, 0)
        }
    }

    /// List all captures.
    pub fn list_captures(&self) -> &[CaptureResult] {
        &self.captures
    }

    /// Get the latest capture.
    pub fn latest_capture(&self) -> Option<&CaptureResult> {
        self.captures.last()
    }

    /// Get status.
    pub fn status(&self) -> EyesStatus {
        EyesStatus {
            running: true,
            port: 0,
            captures: self.captures.len(),
            last_capture: self
                .latest_capture()
                .map(|c| c.image_path.clone()),
        }
    }

    /// Resize an image to `max_width` (for token efficiency when sending
    /// screenshots to the LLM). Returns the original bytes unchanged if
    /// the image is already within the limit.
    pub fn resize_image(
        data: &[u8],
        max_width: u32,
    ) -> anyhow::Result<Vec<u8>> {
        use image::GenericImageView;

        let img = image::load_from_memory(data)?;
        let (w, h) = img.dimensions();

        if w <= max_width {
            return Ok(data.to_vec());
        }

        let ratio = max_width as f64 / w as f64;
        let new_height = (h as f64 * ratio) as u32;
        let resized = img.resize(
            max_width,
            new_height,
            image::imageops::FilterType::Lanczos3,
        );

        let mut buf = Vec::new();
        resized.write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Png,
        )?;
        Ok(buf)
    }

    /// Clear all captures.
    pub fn clear(&mut self) {
        self.captures.clear();
    }
}

// ---------------------------------------------------------------------------
// HTTP / WebSocket router for the browser-based canvas
// ---------------------------------------------------------------------------

/// Build the axum router for the Eyes visual inspection server.
///
/// Routes:
///   GET /    — serves the canvas HTML page
///   GET /ws  — WebSocket endpoint (stub)
pub fn eyes_router() -> axum::Router {
    axum::Router::new()
        .route(
            "/",
            axum::routing::get(|| async {
                axum::response::Html(include_str!("../../static/eyes.html"))
            }),
        )
        .route("/ws", axum::routing::get(ws_handler))
}

async fn ws_handler() -> impl axum::response::IntoResponse {
    "WebSocket endpoint (not yet implemented)"
}
