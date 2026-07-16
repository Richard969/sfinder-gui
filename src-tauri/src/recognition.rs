use image::{RgbImage, codecs::jpeg::JpegEncoder, GenericImageView};
use screenshots::Screen;
use std::io::Cursor;
use std::collections::HashMap;
use std::sync::Mutex;
use base64::Engine;
use serde::Serialize;

/// Standard Tetris piece reference colors (R, G, B) in sRGB
const REFERENCE_COLORS: &[(u8, u8, u8, char)] = &[
    (0,   240, 240, 'I'),  // cyan
    (240, 240, 0,   'O'),  // yellow
    (160, 0,   240, 'T'),  // purple
    (0,   240, 0,   'S'),  // green
    (240, 0,   0,   'Z'),  // red
    (0,   0,   240, 'J'),  // blue
    (240, 160, 0,   'L'),  // orange
    (128, 128, 128, 'X'),  // garbage (gray)
];

/// Expected number of columns in a Tetris board
const NUM_COLS: usize = 10;

/// Minimum HSL lightness for a cell to be considered "not empty"
const MIN_LIGHTNESS: f64 = 25.0;

// ── Color utilities ──

/// Convert RGB to HSL (H: 0-360, S: 0-100, L: 0-100)
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0 * 100.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 50.0 { d / (2.0 - max - min) } else { d / (max + min) } * 100.0;
    let h = match max {
        x if x == r => (g - b) / d + (if g < b { 6.0 } else { 0.0 }),
        x if x == g => (b - r) / d + 2.0,
        _ => (r - g) / d + 4.0,
    } * 60.0;
    (h, s, l)
}

/// Convert RGB to YUV for perceptual distance
fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = -0.168736 * r - 0.331264 * g + 0.5 * b;
    let v = 0.5 * r - 0.418688 * g - 0.081312 * b;
    (y, u, v)
}

fn color_distance(c1: (u8, u8, u8), c2: (u8, u8, u8)) -> f64 {
    let (y1, u1, v1) = rgb_to_yuv(c1.0, c1.1, c1.2);
    let (y2, u2, v2) = rgb_to_yuv(c2.0, c2.1, c2.2);
    let dy = y1 - y2;
    let du = u1 - u2;
    let dv = v1 - v2;
    (2.0 * dy * dy + du * du + dv * dv).sqrt()
}

// ── Recognition ──

/// Match a pixel color to a Tetris piece type.
/// Uses YUV distance against reference colors (primary) + HSL hue check (secondary).
fn match_piece_color(r: u8, g: u8, b: u8) -> char {
    // Quick reject: very dark cells are empty
    let (_, s, l) = rgb_to_hsl(r, g, b);
    if l < MIN_LIGHTNESS {
        return '_';
    }
    // Very low saturation + moderate lightness → garbage (gray)
    if s < 12.0 && l > 30.0 {
        return 'X';
    }

    // Nearest reference color — no distance cutoff, always classify
    let mut best = 'X';
    let mut best_dist = f64::MAX;
    for &(ref_r, ref_g, ref_b, pc) in REFERENCE_COLORS {
        let d = color_distance((r, g, b), (ref_r, ref_g, ref_b));
        if d < best_dist {
            best_dist = d;
            best = pc;
        }
    }
    best
}

/// Sample the average color of a small region around (cx, cy).
/// Helps with anti-aliased edges.
fn sample_cell_avg(img: &RgbImage, cx: u32, cy: u32, radius: u32) -> (u8, u8, u8) {
    let (w, h) = img.dimensions();
    let x0 = cx.saturating_sub(radius);
    let y0 = cy.saturating_sub(radius);
    let x1 = (cx + radius).min(w - 1);
    let y1 = (cy + radius).min(h - 1);
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let mut count = 0u64;
    for py in y0..=y1 {
        for px in x0..=x1 {
            let pixel = img.get_pixel(px, py);
            let (_, _, l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            if l > MIN_LIGHTNESS * 0.7 {
                r += pixel[0] as u64;
                g += pixel[1] as u64;
                b += pixel[2] as u64;
                count += 1;
            }
        }
    }
    if count == 0 {
        let p = img.get_pixel(cx, cy);
        return (p[0], p[1], p[2]);
    }
    ((r / count) as u8, (g / count) as u8, (b / count) as u8)
}

/// Recognize a Tetris board from an RGB image and return a fumen field string.
pub fn recognize_field(img: &RgbImage) -> Result<String, String> {
    let (width, height) = img.dimensions();

    if width < 10 || height < 10 {
        return Err("Image too small (minimum 10×10 pixels)".to_string());
    }

    // Proportional grid over the full image (user selected the board area)
    let cell_w = width as f64 / NUM_COLS as f64;
    let n_rows = (height as f64 / cell_w).ceil() as usize;
    let n_rows = n_rows.max(1).min(40);

    let mut field = String::new();

    // Scan rows bottom-to-top (fumen convention: row 0 = bottom)
    for row in (0..n_rows).rev() {
        let y_top = row as f64 * (height as f64 / n_rows as f64);
        let y_bot = (row + 1) as f64 * (height as f64 / n_rows as f64);
        let y_center = ((y_top + y_bot) / 2.0) as u32;
        let y_center = y_center.min(height - 1);

        for col in 0..NUM_COLS {
            let x_left = col as f64 * (width as f64 / NUM_COLS as f64);
            let x_right = (col + 1) as f64 * (width as f64 / NUM_COLS as f64);
            let x_center = ((x_left + x_right) / 2.0) as u32;
            let x_center = x_center.min(width - 1);

            let (r, g, b) = sample_cell_avg(img, x_center, y_center, 2);
            field.push(match_piece_color(r, g, b));
        }
        if row > 0 {
            field.push('\n');
        }
    }

    // Trim leading/trailing empty rows
    let lines: Vec<&str> = field.lines().collect();
    let mut start = 0;
    while start < lines.len() && lines[start].chars().all(|c| c == '_') {
        start += 1;
    }
    if start == lines.len() {
        return Err("Board appears empty. Is the screenshot showing a Tetris field?".to_string());
    }
    let mut end = lines.len();
    while end > start && lines[end - 1].chars().all(|c| c == '_') {
        end -= 1;
    }

    Ok(lines[start..end].join("\n"))
}

/// Load an image from file path and recognize the Tetris board.
pub fn recognize_field_from_file(path: &str) -> Result<String, String> {
    let img = image::open(path)
        .map_err(|e| format!("Failed to open image '{}': {}", path, e))?
        .to_rgb8();
    recognize_field(&img)
}

/// Recognize from raw image bytes (PNG, JPEG, etc.)
pub fn recognize_field_from_bytes(bytes: &[u8]) -> Result<String, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?
        .to_rgb8();
    recognize_field(&img)
}

// ── Tests ──
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_hsl_red() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 100.0).abs() < 1.0);
        assert!((l - 50.0).abs() < 2.0);
    }

    #[test]
    fn test_match_red_is_z() {
        assert_eq!(match_piece_color(200, 30, 30), 'Z');
    }

    #[test]
    fn test_match_cyan_is_i() {
        assert_eq!(match_piece_color(0, 200, 200), 'I');
    }

    #[test]
    fn test_match_dark_is_empty() {
        assert_eq!(match_piece_color(10, 10, 10), '_');
    }

    #[test]
    fn test_match_green_is_s() {
        assert_eq!(match_piece_color(20, 200, 20), 'S');
    }

    #[test]
    fn test_rgb_to_yuv_black() {
        let (y, _, _) = rgb_to_yuv(0, 0, 0);
        assert!((y).abs() < 0.01);
    }

    #[test]
    fn test_rgb_to_hsl_black() {
        let (h, s, l) = rgb_to_hsl(0, 0, 0);
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 0.0).abs() < 1.0);
        assert!((l - 0.0).abs() < 1.0);
    }
}
// ─── Screenshot capture state ───

#[derive(Clone, Serialize)]
pub struct MonitorInfo {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize)]
pub struct CaptureData {
    pub monitors: Vec<MonitorInfo>,
}

/// Stores captured RgbaImage of each monitor, keyed by (x,y) offset
struct CaptureStore {
    images: HashMap<(i32, i32), image::RgbaImage>,
    dims: HashMap<(i32, i32), (u32, u32)>,
}

static CAPTURE: std::sync::LazyLock<Mutex<Option<CaptureStore>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Capture all monitors, encode as base64 JPEG, store images for cropping
pub fn capture_all_monitors() -> Result<CaptureData, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to access screens: {}", e))?;
    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    let mut store = CaptureStore {
        images: HashMap::new(),
        dims: HashMap::new(),
    };
    let mut monitors = Vec::new();

    for screen in &screens {
        let capture = screen
            .capture()
            .map_err(|e| format!("Failed to capture screen: {}", e))?;

        let w = capture.width();
        let h = capture.height();
        let info = screen.display_info;
        let x = info.x;
        let y = info.y;

        // Convert screenshots::image::RgbaImage → image::RgbaImage (bridging crate versions)
        let raw = capture.as_raw().to_vec();
        let img: image::RgbaImage = image::RgbaImage::from_raw(w, h, raw)
            .ok_or("Failed to convert captured image")?;

        // Encode as JPEG → base64 data URL (downsampled 2x for speed)
        let scale = 2u32;
        let sw = w / scale;
        let sh = h / scale;
        let small = image::imageops::resize(&img, sw, sh, image::imageops::FilterType::Nearest);
        // Convert RGBA → RGB for JPEG (JPEG doesn't support alpha)
        let small_rgb = image::DynamicImage::ImageRgba8(small).to_rgb8();
        let mut jpg_buf = Cursor::new(Vec::new());
        {
            let mut encoder = JpegEncoder::new_with_quality(&mut jpg_buf, 50);
            encoder
                .encode(small_rgb.as_raw(), sw, sh, image::ExtendedColorType::Rgb8)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(jpg_buf.into_inner());
        let data_url = format!("data:image/jpeg;base64,{}", b64);

        monitors.push(MonitorInfo {
            data_url,
            width: sw,
            height: sh,
            x,
            y,
        });

        // Store full-resolution RgbaImage for crop recognition
        store.images.insert((x, y), img);
        store.dims.insert((x, y), (w, h));
    }

    *CAPTURE.lock().map_err(|e| e.to_string())? = Some(store);
    Ok(CaptureData { monitors })
}

/// Crop a region from the captured screen data and recognize the field
pub fn crop_and_recognize(x: i32, y: i32, w: u32, h: u32) -> Result<String, String> {
    let guard = CAPTURE.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No capture data. Capture first.")?;

    // Find which monitor contains this region
    let ((mx, my), img) = store
        .images
        .iter()
        .find(|((mx, my), _)| {
            let (mw, mh) = store.dims.get(&(*mx, *my)).unwrap_or(&(0, 0));
            x >= *mx && y >= *my && x < mx + *mw as i32 && y < my + *mh as i32
        })
        .ok_or("Selection outside all captured monitors")?;

    let ox = (x - mx) as u32;
    let oy = (y - my) as u32;
    let cw = w.min(img.width() - ox);
    let ch = h.min(img.height() - oy);

    // Crop subregion using image crate
    let cropped = image::imageops::crop_imm(img, ox, oy, cw, ch).to_image();

    // Debug: save cropped image for inspection
    let debug_path = std::env::temp_dir().join("sfinder_cropped_debug.png");
    if let Err(e) = cropped.save(&debug_path) {
        eprintln!("Failed to save debug image: {}", e);
    }

    // Convert to RGB for recognition (drop alpha, no blending)
    let rgb = {
        let (w, h) = (cropped.width(), cropped.height());
        let raw = cropped.as_raw();
        let mut rgb_data = Vec::with_capacity((w * h * 3) as usize);
        for chunk in raw.chunks(4) {
            rgb_data.push(chunk[0]); // R
            rgb_data.push(chunk[1]); // G
            rgb_data.push(chunk[2]); // B
        }
        RgbImage::from_raw(w, h, rgb_data).ok_or("Failed to convert to RGB")?
    };
    recognize_field(&rgb)
}

/// Clear capture data after use
pub fn clear_capture() {
    if let Ok(mut guard) = CAPTURE.lock() {
        *guard = None;
    }
}
