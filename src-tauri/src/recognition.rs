use image::{RgbImage, ImageFormat};
use imageproc::edges::canny;
use imageproc::hough::{detect_lines, LineDetectionOptions};
use screenshots::Screen;
use std::io::Cursor;
use std::collections::HashMap;
use std::sync::Mutex;
use base64::Engine;
use serde::Serialize;

/// Standard Tetris piece reference colors (R, G, B) in sRGB
/// These are the canonical colors from the Tetris guideline
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

/// Minimum votes for a Hough line to be accepted
const VOTE_THRESHOLD: u32 = 80;

/// Non-maxima suppression radius for Hough
const SUPPRESSION_RADIUS: u32 = 4;

/// Angle tolerance in degrees for classifying lines
const ANGLE_TOLERANCE: u32 = 12;

/// Minimum aspect ratio for the image to be considered a board screenshot
const MIN_IMAGE_SIZE: u32 = 100;

/// Color distance threshold — above this is treated as empty
const COLOR_DISTANCE_THRESHOLD: f64 = 1.0;

/// Expected number of columns in a Tetris board
const NUM_COLS: usize = 10;

/// Expected minimum number of rows
const NUM_ROWS_MIN: usize = 1;

/// Detect grid lines from edge image and classify into vertical/horizontal groups
fn find_grid_lines(
    edges: &image::GrayImage,
    img_width: u32,
    img_height: u32,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let options = LineDetectionOptions {
        vote_threshold: VOTE_THRESHOLD,
        suppression_radius: SUPPRESSION_RADIUS,
    };

    let lines = detect_lines(edges, options);

    if lines.is_empty() {
        return Err("No grid lines detected. Try a clearer screenshot.".to_string());
    }

    // Classify lines by angle
    // In imageproc's PolarLine:
    //   angle_in_degrees = 0°  → horizontal-ish line (line along x-axis)
    //   angle_in_degrees = 90° → vertical-ish line (line along y-axis)
    let mut verticals: Vec<f64> = Vec::new();
    let mut horizontals: Vec<f64> = Vec::new();

    for line in &lines {
        let angle = line.angle_in_degrees;
        let r = line.r as f64;

        // Vertical lines: angle ≈ 90°
        if angle.abs_diff(90) <= ANGLE_TOLERANCE {
            verticals.push(r);
        }
        // Horizontal lines: angle ≈ 0° (wrapping 180° to 0°)
        // In [0, 180), a horizontal line could be at 0° or near 180°
        // 180° wraps to 0° with negated r, but imageproc reports [0, 180]
        // So we check both: near 0° and near 180°
        if angle <= ANGLE_TOLERANCE || angle >= (180 - ANGLE_TOLERANCE) {
            horizontals.push(line.r as f64);
        }
    }

    if verticals.len() < 3 || horizontals.len() < 3 {
        return Err(format!(
            "Found {} vertical and {} horizontal lines — need at least 3 each. Try a closer crop of the board.",
            verticals.len(),
            horizontals.len()
        ));
    }

    // Cluster lines by position and pick the most "grid-like" sets
    let vertical_lines = cluster_and_sort(&verticals, img_width as f64 * 0.03);
    let horizontal_lines = cluster_and_sort(&horizontals, img_height as f64 * 0.03);

    // Pick the best 11 vertical lines that form a regular grid
    let vertical_lines = pick_grid_lines(&vertical_lines, NUM_COLS + 1, 0.25)?;
    let horizontal_lines = pick_grid_lines(&horizontal_lines, 0, 0.35)?;

    if horizontal_lines.len() < NUM_ROWS_MIN + 1 {
        return Err(format!(
            "Only {} board rows detected (need at least {}). Try a closer crop.",
            horizontal_lines.len() - 1,
            NUM_ROWS_MIN
        ));
    }

    Ok((vertical_lines, horizontal_lines))
}

/// Cluster nearby r values (within `tolerance`) by averaging them,
/// then sort by position.
fn cluster_and_sort(values: &[f64], tolerance: f64) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut clusters: Vec<Vec<f64>> = Vec::new();
    for &v in &sorted {
        if let Some(last) = clusters.last_mut() {
            if (v - last[0]).abs() <= tolerance {
                last.push(v);
                continue;
            }
        }
        clusters.push(vec![v]);
    }

    clusters
        .iter()
        .map(|c| c.iter().sum::<f64>() / c.len() as f64)
        .collect()
}

/// Pick the best set of N equally-spaced lines from the candidates.
/// If `target_n` is 0, finds the largest consistent set.
fn pick_grid_lines(
    candidates: &[f64],
    target_n: usize,
    max_spacing_variance: f64,
) -> Result<Vec<f64>, String> {
    if candidates.len() < 2 {
        return Err("Not enough grid lines detected.".to_string());
    }

    // If we have exactly what we need, return it
    if target_n > 0 && candidates.len() == target_n {
        return Ok(candidates.to_vec());
    }

    // Try to find the most regular sub-grid by sliding window
    // For vertical lines, we want exactly target_n lines
    // For horizontal lines, we want the most consistent grid

    if target_n > 0 {
        // We need exactly target_n lines — find the best subset
        // First try sliding window of size target_n
        let mut best_score = f64::MAX;
        let mut best_set: Option<&[f64]> = None;

        if candidates.len() >= target_n {
            for window in candidates.windows(target_n) {
                let spacings: Vec<f64> = window.windows(2).map(|w| w[1] - w[0]).collect();
                let mean = spacings.iter().sum::<f64>() / spacings.len() as f64;
                let variance = spacings
                    .iter()
                    .map(|s| ((s - mean) / mean).abs())
                    .sum::<f64>()
                    / spacings.len() as f64;

                // Prefer sets closer to the image center (board is usually centered)
                let center_offset = (window[0] + window[target_n - 1]) / 2.0;

                let score = variance + center_offset.abs() * 0.001;
                if score < best_score {
                    best_score = score;
                    best_set = Some(window);
                }
            }
        }

        if let Some(set) = best_set {
            return Ok(set.to_vec());
        }
        return Err(format!(
            "Could not find {} consistent grid lines (found {} candidates)",
            target_n,
            candidates.len()
        ));
    } else {
        // No target N — find the largest consistent set
        // Compute all spacings, find median spacing, then filter lines
        // that maintain consistent spacing
        let mut best_set: Vec<f64> = candidates.to_vec();

        // Filter out outliers by checking spacing consistency
        if best_set.len() > 3 {
            let spacings: Vec<f64> = best_set.windows(2).map(|w| w[1] - w[0]).collect();
            let mean = spacings.iter().sum::<f64>() / spacings.len() as f64;

            let mut filtered = Vec::new();
            filtered.push(best_set[0]);
            for i in 1..best_set.len() {
                let gap = best_set[i] - best_set[i - 1];
                if (gap - mean).abs() / mean <= max_spacing_variance {
                    filtered.push(best_set[i]);
                } else {
                    // Try to skip one: if the next gap compensates, this might be a double-line
                    if i + 1 < best_set.len() {
                        let next_gap = best_set[i + 1] - best_set[i - 1];
                        if (next_gap - mean).abs() / mean <= max_spacing_variance * 1.5 {
                            filtered.push(best_set[i + 1]);
                            continue;
                        }
                    }
                }
            }
            if filtered.len() >= 2 {
                best_set = filtered;
            }
        }

        Ok(best_set)
    }
}

/// Convert RGB to a perceptually-uniform luminance value for color comparison
fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = -0.168736 * r - 0.331264 * g + 0.5 * b;
    let v = 0.5 * r - 0.418688 * g - 0.081312 * b;

    (y, u, v)
}

/// Compute perceptual color distance in YUV space
fn color_distance(c1: (u8, u8, u8), c2: (u8, u8, u8)) -> f64 {
    let (y1, u1, v1) = rgb_to_yuv(c1.0, c1.1, c1.2);
    let (y2, u2, v2) = rgb_to_yuv(c2.0, c2.1, c2.2);
    let dy = y1 - y2;
    let du = u1 - u2;
    let dv = v1 - v2;
    // Weight luminance more heavily for distinguishing pieces from background
    (dy * dy * 2.0 + du * du + dv * dv).sqrt()
}

/// Match a sampled cell color to the nearest Tetris piece color.
/// First checks if the cell differs significantly from the background.
fn match_piece_color(sampled: (u8, u8, u8), bg_color: Option<(u8, u8, u8)>) -> char {
    // If the cell is too dark, it's background/empty
    let (y, _u, _v) = rgb_to_yuv(sampled.0, sampled.1, sampled.2);
    if y < 0.15 {
        return '_';
    }

    // If background is available, discard cells close to it
    if let Some(bg) = bg_color {
        if color_distance(sampled, bg) < 0.3 {
            return '_';
        }
    }

    let mut best_char = '_';
    let mut best_dist = COLOR_DISTANCE_THRESHOLD;

    for &(ref_r, ref_g, ref_b, piece_char) in REFERENCE_COLORS {
        let dist = color_distance(sampled, (ref_r, ref_g, ref_b));
        if dist < best_dist {
            best_dist = dist;
            best_char = piece_char;
        }
    }

    best_char
}

/// Sample background color from empty cells at the top of the detected grid.
fn sample_background(img: &image::RgbImage, rows: usize, cols: usize,
    horizontal_lines: &[f64], vertical_lines: &[f64]) -> Option<(u8, u8, u8)> {
    let (w, h) = img.dimensions();
    let mut r_sum = 0u64; let mut g_sum = 0u64; let mut b_sum = 0u64;
    let mut count = 0u64;

    let sample_rows = 3.min(rows);
    let sample_cols = 5.min(cols);

    for row in 0..sample_rows {
        let y_top = horizontal_lines[row] as u32;
        let y_bot = horizontal_lines[row + 1] as u32;
        let y_center = ((y_top + y_bot) / 2).min(h - 1);
        for col in 0..sample_cols {
            let x_left = vertical_lines[col] as u32;
            let x_right = vertical_lines[col + 1] as u32;
            let x_center = ((x_left + x_right) / 2).min(w - 1);
            let px = img.get_pixel(x_center, y_center);
            let (y_val, _, _) = rgb_to_yuv(px[0], px[1], px[2]);
            if y_val < 0.4 {
                r_sum += px[0] as u64;
                g_sum += px[1] as u64;
                b_sum += px[2] as u64;
                count += 1;
            }
        }
    }

    if count > 0 {
        Some(((r_sum / count) as u8, (g_sum / count) as u8, (b_sum / count) as u8))
    } else {
        let px = img.get_pixel(5, 5);
        Some((px[0], px[1], px[2]))
    }
}

/// Recognize a Tetris board from an RGB image and return a fumen field string.
/// The field string uses the same format as the tetris-fumen JavaScript package:
/// rows from bottom to top, each row 10 chars, using piece letters.
pub fn recognize_field(img: &RgbImage) -> Result<String, String> {
    let (width, height) = img.dimensions();

    if width < MIN_IMAGE_SIZE || height < MIN_IMAGE_SIZE {
        return Err(format!(
            "Image too small ({}x{}). Minimum is {}px.",
            width, height, MIN_IMAGE_SIZE
        ));
    }

    // 1. Convert to grayscale
    let gray = image::imageops::grayscale(img);

    // 2. Edge detection with Canny
    // Use automatic thresholds based on image intensity statistics
    let edges = canny(&gray, 15.0, 40.0);

    // 3. Detect grid lines via Hough transform
    let (vertical_lines, horizontal_lines) = find_grid_lines(&edges, width, height)?;

    // 4. Build cell grid
    // For each cell defined by adjacent grid lines, sample the color at its center
    let num_rows = horizontal_lines.len() - 1;
    let num_cols = vertical_lines.len() - 1;

    // Sample adaptive background color from top cells
    let bg = sample_background(img, num_rows, num_cols, &horizontal_lines, &vertical_lines);

    let mut field = String::new();

    // Build rows from top (highest y) to bottom (lowest y)
    // But fumen expects bottom-to-top, so we iterate rows in reverse
    for row in (0..num_rows).rev() {
        let y_top = horizontal_lines[row] as u32;
        let y_bot = horizontal_lines[row + 1] as u32;
        let y_center = ((y_top + y_bot) / 2).min(height - 1);

        for col in 0..num_cols {
            let x_left = vertical_lines[col] as u32;
            let x_right = vertical_lines[col + 1] as u32;
            let x_center = ((x_left + x_right) / 2).min(width - 1);

            let pixel = img.get_pixel(x_center, y_center);
            let piece = match_piece_color((pixel[0], pixel[1], pixel[2]), bg);
            field.push(piece);
        }
        if row > 0 {
            field.push('\n');
        }
    }

    if field.trim().is_empty() || field.chars().all(|c| c == '_' || c == '\n') {
        return Err("Board appears empty. Is the screenshot showing a Tetris field?".to_string());
    }

    Ok(field)
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
        let img = image::RgbaImage::from_raw(w, h, rgba.clone())
            .ok_or("Failed to create image from capture")?;
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

        // Fast JPEG at medium quality (50) — fast enough for overlay display
        let mut jpg_buf = Cursor::new(Vec::new());
        {
            use image::codecs::jpeg::JpegEncoder;
            let mut encoder = JpegEncoder::new_with_quality(&mut jpg_buf, 50);
            encoder.encode(&rgb.as_raw(), sw, sh, image::ExtendedColorType::Rgb8)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(jpg_buf.into_inner());
        let data_url = format!("data:image/jpeg;base64,{}", b64);

        // Store full-resolution raw RGBA for crop recognition (unscaled)
        store.images.insert((x, y), rgba);
        store.dims.insert((x, y), (w, h));

        monitors.push(MonitorInfo {
            data_url,
            width: sw,
            height: sh,
            x,
            y,
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_yuv_known() {
        // Pure red should give specific YUV values
        let (y, u, v) = rgb_to_yuv(255, 0, 0);
        assert!((y - 0.299).abs() < 0.001);
        assert!((u + 0.168736).abs() < 0.001);
        assert!((v - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_color_distance_zero() {
        let dist = color_distance((100, 100, 100), (100, 100, 100));
        assert!(dist < 0.001);
    }

    #[test]
    fn test_match_piece_color_red_is_z() {
        // Z piece is red
        let piece = match_piece_color((240, 0, 0), Some((0, 0, 0)));
        assert_eq!(piece, 'Z');
    }

    #[test]
    fn test_match_piece_color_cyan_is_i() {
        let piece = match_piece_color((0, 240, 240), Some((0, 0, 0)));
        assert_eq!(piece, 'I');
    }

    #[test]
    fn test_match_piece_color_dark_is_empty() {
        // Very dark color should be empty (below threshold)
        let piece = match_piece_color((10, 10, 10), Some((0, 0, 0)));
        assert_eq!(piece, '_');
    }
}
