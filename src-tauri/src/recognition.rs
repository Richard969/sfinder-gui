use image::{codecs::jpeg::JpegEncoder, RgbImage};
use screenshots::Screen;
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Mutex;

/// Expected number of columns in a Tetris board
const NUM_COLS: usize = 10;

// ── Grid detection ──

/// Detect grid cell width by finding vertical edges of blocks.
/// Samples only the bottom 60% (skips active pieces at top).
/// Uses luminance (Y) transitions since garbage blocks are grey but brighter than bg.
fn detect_cell_width(img: &RgbImage) -> f64 {
    let (width, height) = img.dimensions();
    if width < 10 || height < 10 {
        return width as f64 / 10.0;
    }

    // Sample rows from bottom 60% only (skip active piece area at top)
    let y_start = height / 5;
    let sample_ys = [
        y_start + (height - y_start) / 4,
        y_start + (height - y_start) / 2,
        y_start + 3 * (height - y_start) / 4,
    ];
    let mut all_edges: Vec<u32> = Vec::new();

    for &y in &sample_ys {
        let y = y.min(height - 1);
        // Get baseline luminance (median of first 10 pixels)
        let bg_lum: f64 = (0..10.min(width as usize))
            .map(|x| {
                let px = img.get_pixel(x as u32, y);
                0.299 * px[0] as f64 + 0.587 * px[1] as f64 + 0.114 * px[2] as f64
            })
            .sum::<f64>() / 10.0_f64.min(width as f64);

        let mut prev_above = false;
        let mut edges: Vec<u32> = Vec::new();
        for x in 0..width {
            let px = img.get_pixel(x, y);
            let lum = 0.299 * px[0] as f64 + 0.587 * px[1] as f64 + 0.114 * px[2] as f64;
            let is_above = lum > bg_lum + 15.0;
            if is_above && !prev_above {
                edges.push(x);
            }
            prev_above = is_above;
        }
        all_edges.extend(edges);
    }

    if all_edges.len() < 20 {
        return width as f64 / 10.0;
    }

    all_edges.sort();
    let mut groups: Vec<Vec<u32>> = Vec::new();
    for &edge in &all_edges {
        if let Some(last_group) = groups.last_mut() {
            let avg = last_group.iter().sum::<u32>() / last_group.len() as u32;
            if (edge as i32 - avg as i32).abs() <= 5 {
                last_group.push(edge);
                continue;
            }
        }
        groups.push(vec![edge]);
    }

    if groups.len() < 8 {
        return width as f64 / 10.0;
    }

    let mut group_x: Vec<u32> = groups
        .iter()
        .map(|g| {
            let mut sorted = g.clone();
            sorted.sort();
            sorted[sorted.len() / 2]
        })
        .collect();
    group_x.sort_unstable();

    let gaps: Vec<u32> = group_x.windows(2).map(|w| w[1] - w[0]).collect();
    if gaps.is_empty() {
        return width as f64 / 10.0;
    }

    let mut gap_hist: Vec<(u32, usize)> = Vec::new();
    for &gap in &gaps {
        let mut found = false;
        for entry in gap_hist.iter_mut() {
            if (entry.0 as i32 - gap as i32).abs() <= 3 {
                entry.1 += 1;
                entry.0 = (entry.0 + gap) / 2;
                found = true;
                break;
            }
        }
        if !found {
            gap_hist.push((gap, 1));
        }
    }

    gap_hist.sort_by_key(|&(_, count)| std::cmp::Reverse(count));

    if let Some(&(best_gap, _)) = gap_hist.first() {
        let cell_w = best_gap.max(4) as f64;
        if cell_w <= width as f64 * 0.25 {
            return cell_w;
        }
    }

    width as f64 / 10.0
}

// ── Color utilities ──

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

fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = -0.14713 * r - 0.28886 * g + 0.436 * b;
    let v = 0.615 * r - 0.51499 * g - 0.10001 * b;
    (y, u, v)
}

// ── Classification ──

/// Tetr.io piece reference colors (R, G, B).
const REFERENCE_COLORS: &[(u8, u8, u8, char)] = &[
    (0, 0, 0, '_'),       // empty (black background)
    (52, 181, 133, 'I'),  // teal
    (179, 153, 50, 'O'),  // yellow
    (164, 62, 154, 'T'),  // purple
    (131, 179, 50, 'S'),  // green
    (180, 52, 59, 'Z'),   // red
    (79, 62, 164, 'J'),   // blue
    (178, 98, 49, 'L'),   // orange
    (128, 128, 128, 'X'), // garbage
];

/// Match a pixel to a Tetris piece type.
/// 1. HSL: very dark → empty (_)
/// 2. HSL: low saturation → garbage (X) or empty (_), skip color matching
/// 3. YUV: nearest match (prevents grey→J false positives)
pub fn match_piece_color(r: u8, g: u8, b: u8) -> char {
    let (_, s, l) = rgb_to_hsl(r, g, b);

    // Stage 1: very dark → empty
    if l < 15.0 {
        return '_';
    }

    // Stage 2: low saturation → garbage or empty (skip YUV matching)
    if s < 20.0 {
        // Garbage is grey but brighter than empty
        if l > 30.0 {
            return 'X';
        }
        return '_';
    }

    // Stage 3: high saturation → YUV nearest match
    let (y, u, v) = rgb_to_yuv(r, g, b);
    let mut best = '_';
    let mut best_dist = f64::MAX;
    for &(ref_r, ref_g, ref_b, pc) in REFERENCE_COLORS {
        if pc == '_' || pc == 'X' {
            continue; // skip empty/garbage reference for high-sat pixels
        }
        let (ry, ru, rv) = rgb_to_yuv(ref_r, ref_g, ref_b);
        let dy = y - ry;
        let du = u - ru;
        let dv = v - rv;
        let d = 2.0 * dy * dy + du * du + dv * dv;
        if d < best_dist {
            best_dist = d;
            best = pc;
        }
    }
    best
}

// ── Recognition ──

/// Recognize a Tetris board from an RGB image, returns (field, debug_info).
pub fn recognize_field(img: &RgbImage) -> Result<(String, String), String> {
    let (width, height) = img.dimensions();
    if width < 10 || height < 10 {
        return Err("Image too small (minimum 10×10 pixels)".to_string());
    }

    let cell_w = detect_cell_width(img);
    let n_rows = (height as f64 / cell_w).ceil() as usize;
    let n_rows = n_rows.max(1).min(40);

    let mut raw_lines: Vec<String> = Vec::new();
    let mut debug_cells: Vec<String> = Vec::new();

    for row in (0..n_rows).rev() {
        let y_top = row as f64 * (height as f64 / n_rows as f64);
        let y_bot = (row + 1) as f64 * (height as f64 / n_rows as f64);
        let y_center = ((y_top + y_bot) / 2.0) as u32;
        let y_center = y_center.min(height - 1);

        let mut line = String::with_capacity(NUM_COLS);
        for col in 0..NUM_COLS {
            let x_left = col as f64 * cell_w;
            let x_right = (col + 1) as f64 * cell_w;
            let x_center = ((x_left + x_right) / 2.0) as u32;
            let x_center = x_center.min(width - 1);

            let px = img.get_pixel(x_center, y_center);
            let (r, g, b) = (px[0], px[1], px[2]);
            let ch = match_piece_color(r, g, b);
            line.push(ch);

            debug_cells.push(format!(
                "r{}c{}: rgb({},{},{}) -> '{}'",
                row, col, r, g, b, ch
            ));
        }
        raw_lines.push(line);
    }

    // Trim leading/trailing empty rows
    let mut start = 0;
    while start < raw_lines.len() && raw_lines[start].chars().all(|c| c == '_') {
        start += 1;
    }
    if start == raw_lines.len() {
        return Err("Board appears empty. Is the screenshot showing a Tetris field?".to_string());
    }
    let mut end = raw_lines.len();
    while end > start && raw_lines[end - 1].chars().all(|c| c == '_') {
        end -= 1;
    }

    let trimmed: Vec<&str> = raw_lines[start..end].iter().map(|s| s.as_str()).collect();

    let debug = format!(
        "cell_w={:.1}px, n_rows={}, trimmed={}..{}. debug: {}",
        cell_w,
        n_rows,
        start,
        end,
        debug_cells.join(", ")
    );

    Ok((trimmed.join("\n"), debug))
}

/// Backward-compatible: recognize and return just the field string.
pub fn recognize_field_simple(img: &RgbImage) -> Result<String, String> {
    recognize_field(img).map(|(field, _)| field)
}

// ── File / bytes helpers ──

pub fn recognize_field_from_file(path: &str) -> Result<String, String> {
    let img = image::open(path)
        .map_err(|e| format!("Failed to open image '{}': {}", path, e))?
        .to_rgb8();
    recognize_field_simple(&img)
}

pub fn recognize_field_from_bytes(bytes: &[u8]) -> Result<String, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?
        .to_rgb8();
    recognize_field_simple(&img)
}

// ── Screenshot capture state ──

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

struct CaptureStore {
    images: HashMap<(i32, i32), image::RgbaImage>,
    dims: HashMap<(i32, i32), (u32, u32)>,
}

static CAPTURE: std::sync::LazyLock<Mutex<Option<CaptureStore>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

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

        let img: image::RgbaImage =
            image::RgbaImage::from_raw(w, h, capture.as_raw().to_vec())
                .ok_or("Failed to convert captured image")?;

        let scale = 2u32;
        let sw = w / scale;
        let sh = h / scale;
        let small = image::imageops::resize(&img, sw, sh, image::imageops::FilterType::Nearest);
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

        store.images.insert((x, y), img);
        store.dims.insert((x, y), (w, h));
    }

    *CAPTURE.lock().map_err(|e| e.to_string())? = Some(store);
    Ok(CaptureData { monitors })
}

pub fn crop_and_recognize(x: i32, y: i32, w: u32, h: u32) -> Result<String, String> {
    let guard = CAPTURE.lock().map_err(|e| e.to_string())?;
    let store = guard.as_ref().ok_or("No capture data. Capture first.")?;

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

    let cropped = image::imageops::crop_imm(img, ox, oy, cw, ch).to_image();

    let debug_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::temp_dir())
        .join("sfinder_cropped_debug.png");
    if let Err(e) = cropped.save(&debug_path) {
        eprintln!("Failed to save debug image: {}", e);
    }

    // Debug: write YCbCr classification log for every cell to a file
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::temp_dir())
        .join("sfinder_classify_debug.txt");

    let rgb = {
        let (w, h) = (cropped.width(), cropped.height());
        let raw = cropped.as_raw();
        let mut rgb_data = Vec::with_capacity((w * h * 3) as usize);
        for chunk in raw.chunks(4) {
            rgb_data.push(chunk[0]);
            rgb_data.push(chunk[1]);
            rgb_data.push(chunk[2]);
        }
        RgbImage::from_raw(w, h, rgb_data).ok_or("Failed to convert to RGB")?
    };

    let (field, debug) = recognize_field(&rgb)?;
    // Append cropped image info to debug
    let full_debug = format!("{}\ncropped={}x{}", debug, rgb.width(), rgb.height());
    let _ = std::fs::write(&log_path, &full_debug);
    Ok(field)
}

pub fn clear_capture() {
    if let Ok(mut guard) = CAPTURE.lock() {
        *guard = None;
    }
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
        // tetr.io red Z: ~(210, 75, 85) → should be Z
        assert_eq!(match_piece_color(210, 75, 85), 'Z');
    }

    #[test]
    fn test_match_cyan_is_i() {
        assert_eq!(match_piece_color(52, 181, 133), 'I');
    }

    // ... additional tests preserved from original ...
}
