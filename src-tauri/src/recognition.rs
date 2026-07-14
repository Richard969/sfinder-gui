use image::{RgbImage, codecs::jpeg::JpegEncoder};
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

// ── Board detection ──

/// Find the pixel bounds of the actual Tetris board within the image.
/// Scans inward from edges looking for non-dark (colored) cells.
fn find_board_bounds(img: &RgbImage) -> (u32, u32, u32, u32) {
    let (w, h) = img.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;

    // Sample every 4th pixel for speed
    for y in (0..h).step_by(4) {
        for x in (0..w).step_by(4) {
            let px = img.get_pixel(x, y);
            let (_, _, l) = rgb_to_hsl(px[0], px[1], px[2]);
            if l > MIN_LIGHTNESS {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if max_x <= min_x || max_y <= min_y {
        // No colored cells found — use whole image
        return (0, 0, w, h);
    }

    // Add a small margin (2px) to avoid cutting off edges
    let margin = 2u32;
    let x0 = min_x.saturating_sub(margin);
    let y0 = min_y.saturating_sub(margin);
    let x1 = (max_x + margin).min(w);
    let y1 = (max_y + margin).min(h);
    (x0, y0, x1, y1)
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

    // 1. Find board bounds within the image
    let (bx0, by0, bx1, by1) = find_board_bounds(img);
    let bw = bx1 - bx0;
    let bh = by1 - by0;

    // 2. Build proportional grid over the detected board region
    let cell_w = bw as f64 / NUM_COLS as f64;
    let n_rows = (bh as f64 / cell_w).ceil() as usize;
    let n_rows = n_rows.max(1).min(40);

    let mut field = String::new();

    // Scan rows bottom-to-top (fumen convention)
    for row in (0..n_rows).rev() {
        let y_top = by0 as f64 + row as f64 * (bh as f64 / n_rows as f64);
        let y_bot = by0 as f64 + (row + 1) as f64 * (bh as f64 / n_rows as f64);
        let y_center = ((y_top + y_bot) / 2.0) as u32;
        let y_center = y_center.min(height - 1);

        for col in 0..NUM_COLS {
            let x_left = bx0 as f64 + col as f64 * (bw as f64 / NUM_COLS as f64);
            let x_right = bx0 as f64 + (col + 1) as f64 * (bw as f64 / NUM_COLS as f64);
            let x_center = ((x_left + x_right) / 2.0) as u32;
            let x_center = x_center.min(width - 1);

            let (r, g, b) = sample_cell_avg(img, x_center, y_center, 2);
            field.push(match_piece_color(r, g, b));
        }
        if row > 0 {
            field.push('\n');
        }
    }

    // 3. Trim leading/trailing empty rows
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

    Ok(lines[start..end].join("
"))
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
    fn test_find_board_bounds_all_dark() {
        // A 10x10 image with all black pixels should return full image bounds
        let img = RgbImage::from_fn(10, 10, |_, _| image::Rgb([0, 0, 0]));
        let (x0, y0, x1, y1) = find_board_bounds(&img);
        assert_eq!((x0, y0, x1, y1), (0, 0, 10, 10));
    }

    #[test]
    fn test_find_board_bounds_small_bright_region() {
        // 20x20 image, one bright pixel at (5,5)
        let mut img = RgbImage::from_fn(20, 20, |_, _| image::Rgb([0, 0, 0]));
        img.put_pixel(5, 5, image::Rgb([255, 255, 255]));
        let (x0, y0, x1, y1) = find_board_bounds(&img);
        // Should find the bright region with some margin
        assert!(x0 <= 5);
        assert!(y0 <= 5);
        assert!(x1 >= 5);
        assert!(y1 >= 5);
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

/// Stores raw RGBA pixels of captured monitors, keyed by (x,y) offset
struct CaptureStore {
    images: HashMap<(i32, i32), Vec<u8>>,
    dims: HashMap<(i32, i32), (u32, u32)>,
}

static CAPTURE: std::sync::LazyLock<Mutex<Option<CaptureStore>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Capture all monitors, encode as base64 PNG, store raw pixels for cropping
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
        let buf: Vec<u8> = capture.as_raw().to_vec(); // BGRA pixels

        // Convert BGRA → RGBA
        let mut rgba = Vec::with_capacity(buf.len());
        for chunk in buf.chunks(4) {
            rgba.push(chunk[2]); // R
            rgba.push(chunk[1]); // G
            rgba.push(chunk[0]); // B
            rgba.push(chunk[3]); // A
        }

        // Encode as JPEG → base64 data URL (faster than PNG)
        // Convert RGBA to RGB for JPEG, at half resolution for overlay display speed
        let scale = 2u32; // downsample by 2x
        let sw = w / scale;
        let sh = h / scale;
        let mut rgb_data = Vec::with_capacity((sw * sh * 3) as usize);
        for row in 0..sh {
            for col in 0..sw {
                let idx = ((row * scale * w + col * scale) * 4) as usize;
                rgb_data.push(rgba[idx]);     // R
                rgb_data.push(rgba[idx + 1]); // G
                rgb_data.push(rgba[idx + 2]); // B
            }
        }

        let rgb = image::RgbImage::from_raw(sw, sh, rgb_data)
            .ok_or("Failed to create RGB image")?;

        // Fast JPEG at quality 50, encode as base64 data URL
        let mut jpg_buf = Cursor::new(Vec::new());
        {
            let mut encoder = JpegEncoder::new_with_quality(&mut jpg_buf, 50);
            encoder.encode(&rgb.as_raw(), sw, sh, image::ExtendedColorType::Rgb8)
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

        // Store full-resolution raw RGBA for crop recognition (unscaled)
        store.images.insert((x, y), rgba);
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
    let monitor = store.images.iter().find(|((mx, my), _)| {
        let (mw, mh) = store.dims.get(&(*mx, *my)).unwrap_or(&(0, 0));
        x >= *mx && y >= *my && x < mx + *mw as i32 && y < my + *mh as i32
    }).ok_or("Selection outside all captured monitors")?;

    let ((mx, my), pixels) = monitor;
    let (mw, mh) = store.dims.get(&(*mx, *my)).unwrap();
    let ox = (x - mx) as u32;
    let oy = (y - my) as u32;
    let cw = w.min(*mw - ox);
    let ch = h.min(*mh - oy);

    // Extract RGBA pixels for the cropped region
    let mut cropped = Vec::with_capacity((cw * ch * 3) as usize);
    for row in oy..oy + ch {
        for col in ox..ox + cw {
            let idx = ((row * mw + col) * 4) as usize;
            cropped.push(pixels[idx]);     // R
            cropped.push(pixels[idx + 1]); // G
            cropped.push(pixels[idx + 2]); // B
        }
    }

    let img = RgbImage::from_raw(cw, ch, cropped)
        .ok_or("Failed to create cropped image")?;

    recognize_field(&img)
}

/// Clear capture data after use
pub fn clear_capture() {
    if let Ok(mut guard) = CAPTURE.lock() {
        *guard = None;
    }
}
