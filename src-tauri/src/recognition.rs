use image::{codecs::png::PngEncoder, ImageEncoder, RgbImage};
use screenshots::Screen;
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Mutex;

/// Expected number of columns in a Tetris board
const NUM_COLS: usize = 10;

/// A color palette for a specific Tetris game client.
#[derive(Clone)]
pub struct ColorPalette {
    pub name: &'static str,
    pub colors: &'static [(u8, u8, u8, char)],
}

/// Tetr.io default skin (user's eyedropper values).
pub const PALETTE_TETR_IO: ColorPalette = ColorPalette {
    name: "tetr.io",
    colors: &[
        (0, 0, 0, '_'),       // empty
        (52, 181, 133, 'I'),  // 34b585 teal
        (179, 153, 50, 'O'),  // b39932 yellow
        (164, 62, 154, 'T'),  // a43e9a purple
        (131, 179, 50, 'S'),  // 83b332 green
        (180, 52, 59, 'Z'),   // b4343b red
        (79, 62, 164, 'J'),   // 4f3ea4 blue
        (178, 98, 49, 'L'),   // b26231 orange
        (67, 67, 67, 'X'),    // 434343 garbage
    ],
};

/// Jstris default skin (user-provided hex values).
pub const PALETTE_JSTRIS: ColorPalette = ColorPalette {
    name: "jstris",
    colors: &[
        (0, 0, 0, '_'),       // 000000 empty
        (17, 149, 205, 'I'),  // 1195cd cyan/blue
        (227, 159, 2, 'O'),   // e39f02 yellow
        (175, 41, 138, 'T'),  // af298a purple
        (89, 177, 1, 'S'),    // 59b101 green
        (215, 15, 55, 'Z'),   // d70f37 red
        (33, 65, 198, 'J'),   // 2141c6 blue
        (227, 91, 2, 'L'),    // e35b02 orange
        (153, 153, 153, 'X'), // 999999 garbage
    ],
};

/// All available palettes for auto-detection.
pub const PALETTES: &[ColorPalette] = &[PALETTE_TETR_IO, PALETTE_JSTRIS];

/// Auto-detect which palette matches the image by comparing average color distances.
fn detect_palette(img: &RgbImage) -> &'static ColorPalette {
    let (width, height) = img.dimensions();
    let sample_ys = [height / 3, height / 2, 2 * height / 3];

    // Collect high-saturation sample points from bottom 60%
    let mut samples: Vec<(u8, u8, u8)> = Vec::new();
    for &y in &sample_ys {
        for x in 0..width {
            let px = img.get_pixel(x, y);
            let (_, s, l) = rgb_to_hsl(px[0], px[1], px[2]);
            if s > 30.0 && l > 20.0 && l < 80.0 {
                samples.push((px[0], px[1], px[2]));
            }
        }
    }

    if samples.len() < 10 {
        return &PALETTE_TETR_IO; // default
    }

    // For each palette, compute average distance from samples to nearest color
    let mut best_palette = &PALETTE_TETR_IO;
    let mut best_score = f64::MAX;

    for palette in PALETTES {
        let mut total_dist = 0.0;
        let mut count = 0;
        for &(r, g, b) in &samples {
            let mut min_dist = f64::MAX;
            for &(ref_r, ref_g, ref_b, pc) in palette.colors {
                if pc == '_' || pc == 'X' {
                    continue;
                }
                let (y1, u1, v1) = rgb_to_yuv(r, g, b);
                let (y2, u2, v2) = rgb_to_yuv(ref_r, ref_g, ref_b);
                let dy = y1 - y2;
                let du = u1 - u2;
                let dv = v1 - v2;
                let d = (2.0 * dy * dy + du * du + dv * dv).sqrt();
                if d < min_dist {
                    min_dist = d;
                }
            }
            total_dist += min_dist;
            count += 1;
        }
        if count > 0 {
            let avg = total_dist / count as f64;
            if avg < best_score {
                best_score = avg;
                best_palette = palette;
            }
        }
    }

    best_palette
}

// ── Grid detection ──

/// Detect the board region by finding topmost and bottommost rows with significant content.
/// Returns (y_top, y_bottom) exclusive bounds of the actual board.
fn detect_board_region(img: &RgbImage) -> (u32, u32) {
    let (width, height) = img.dimensions();
    if height < 10 {
        return (0, height);
    }

    let mut y_bottom = height;
    let mut y_top = 0;

    // Find bottommost row with >2 non-empty cells
    for y in (0..height).rev() {
        let mut non_empty = 0;
        for x in 0..width {
            let px = img.get_pixel(x, y);
            let (_, _, l) = rgb_to_hsl(px[0], px[1], px[2]);
            if l > 20.0 {
                non_empty += 1;
            }
        }
        if non_empty > 2 {
            y_bottom = y + 1;
            break;
        }
    }

    // Find topmost row with >2 non-empty cells
    for y in 0..y_bottom {
        let mut non_empty = 0;
        for x in 0..width {
            let px = img.get_pixel(x, y);
            let (_, _, l) = rgb_to_hsl(px[0], px[1], px[2]);
            if l > 20.0 {
                non_empty += 1;
            }
        }
        if non_empty > 2 {
            y_top = y;
            break;
        }
    }

    (y_top, y_bottom)
}

/// Detect grid cell width by finding vertical edges of blocks.
/// Samples only the bottom 60% (skips active pieces at top).
fn detect_cell_width(img: &RgbImage) -> f64 {
    let (width, height) = img.dimensions();
    if width < 10 || height < 10 {
        return width as f64 / 10.0;
    }

    let y_start = height / 5;
    let sample_ys = [
        y_start + (height - y_start) / 4,
        y_start + (height - y_start) / 2,
        y_start + 3 * (height - y_start) / 4,
    ];
    let mut all_edges: Vec<u32> = Vec::new();

    for &y in &sample_ys {
        let y = y.min(height - 1);
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

/// Match a pixel to a Tetris piece type using a specific palette.
pub fn match_piece_color_with_palette(r: u8, g: u8, b: u8, palette: &ColorPalette) -> char {
    // Stage 1: greyscale detection
    let rg = (r as i32 - g as i32).abs();
    let gb = (g as i32 - b as i32).abs();
    let rb = (r as i32 - b as i32).abs();
    if rg < 8 && gb < 8 && rb < 8 {
        let (_, _, l) = rgb_to_hsl(r, g, b);
        if l > 25.0 {
            return 'X';
        }
        return '_';
    }

    // Stage 2: YUV nearest match
    let (y, u, v) = rgb_to_yuv(r, g, b);
    let mut best = '_';
    let mut best_dist = f64::MAX;
    for &(ref_r, ref_g, ref_b, pc) in palette.colors {
        if pc == '_' || pc == 'X' {
            continue;
        }
        let (ry, ru, rv) = rgb_to_yuv(ref_r, ref_g, ref_b);
        let dy = y - ry;
        let du = u - ru;
        let dv = v - rv;
        let d = (2.0 * dy * dy + du * du + dv * dv).sqrt();
        if d < best_dist {
            best_dist = d;
            best = pc;
        }
    }
    if best_dist < 0.15 {
        return best;
    }

    // Stage 3: fallback
    let (_, _, l) = rgb_to_hsl(r, g, b);
    if l > 20.0 {
        'X'
    } else {
        '_'
    }
}

/// Auto-detect palette and match.
pub fn match_piece_color(r: u8, g: u8, b: u8, palette: &ColorPalette) -> char {
    match_piece_color_with_palette(r, g, b, palette)
}

// ── Recognition ──

/// Recognize a Tetris board from an RGB image, returns (field, debug_info).
pub fn recognize_field(img: &RgbImage) -> Result<(String, String), String> {
    let (width, height) = img.dimensions();
    if width < 10 || height < 10 {
        return Err("Image too small (minimum 10×10 pixels)".to_string());
    }

    // Detect actual board region (skip UI elements above/below)
    let (y_top, y_bottom) = detect_board_region(img);
    let board_height = y_bottom - y_top;
    if board_height < 10 {
        return Err("Could not detect board region".to_string());
    }

    let palette = detect_palette(img);
    let cell_w = detect_cell_width(img);
    let n_rows = (board_height as f64 / cell_w).ceil() as usize;
    let n_rows = n_rows.max(1).min(40);

    let mut raw_lines: Vec<String> = Vec::new();
    let mut debug_cells: Vec<String> = Vec::new();

    for row in (0..n_rows).rev() {
        let y_top = y_top as f64 + row as f64 * cell_w;
        let y_bot = y_top as f64 + cell_w;
        let y_center = ((y_top + y_bot) / 2.0) as u32;
        let y_center = y_center.min(height - 1);

        let mut line = String::with_capacity(NUM_COLS);
        for col in 0..NUM_COLS {
            let x_left = col as f64 * cell_w;
            let x_right = (col + 1) as f64 * cell_w;
            let x_center = ((x_left + x_right) / 2.0) as u32;
            let x_center = x_center.min(width - 1);

            // 5x5 region averaging for anti-aliased edges
            let block_size = (cell_w / 4.0) as u32;
            let x0 = x_center.saturating_sub(block_size);
            let y0 = y_center.saturating_sub(block_size);
            let x1 = (x_center + block_size).min(width - 1);
            let y1 = (y_center + block_size).min(height - 1);
            let mut r_sum = 0u64;
            let mut g_sum = 0u64;
            let mut b_sum = 0u64;
            let mut count = 0u64;
            for py in y0..=y1 {
                for px in x0..=x1 {
                    let px2 = img.get_pixel(px, py);
                    r_sum += px2[0] as u64;
                    g_sum += px2[1] as u64;
                    b_sum += px2[2] as u64;
                    count += 1;
                }
            }
            let r = (r_sum / count) as u8;
            let g = (g_sum / count) as u8;
            let b = (b_sum / count) as u8;
            let ch = match_piece_color_with_palette(r, g, b, palette);
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

    // Trim sparse rows from both edges
    let max_pieces = trimmed.iter().map(|l| l.chars().filter(|c| *c != '_').count()).max().unwrap_or(0);
    let threshold = (max_pieces / 3).max(3);

    let mut new_start = 0;
    for (i, line) in trimmed.iter().enumerate() {
        if line.chars().filter(|c| *c != '_').count() >= threshold {
            break;
        }
        new_start = i + 1;
    }

    let mut new_end = trimmed.len();
    for (i, line) in trimmed.iter().enumerate().rev() {
        if line.chars().filter(|c| *c != '_').count() >= threshold {
            break;
        }
        new_end = i;
    }

    let trimmed = &trimmed[new_start..new_end.max(new_start)];

    // If any row has no garbage (X), trim garbage rows above it
    let has_clean_row = trimmed.iter().any(|l| !l.contains('X'));

    if has_clean_row {
        let mut final_start = 0;
        for (i, line) in trimmed.iter().enumerate() {
            if line.contains('X') {
                final_start = i + 1;
            } else {
                break;
            }
        }
        let trimmed = &trimmed[final_start..];
        let debug = format!(
            "palette={}, cell_w={:.1}px, n_rows={}, trimmed={}..{}, garbage_trim={}",
            palette.name, cell_w, n_rows, start, end, final_start
        );
        return Ok((trimmed.join("\n"), debug));
    }

    // Trim all-garbage rows from edges (garbage buffer above/below board)
    let mut final_start = 0;
    for (i, line) in trimmed.iter().enumerate() {
        if line.chars().all(|c| c == 'X' || c == '_') && line.chars().filter(|&c| c == 'X').count() >= 5 {
            final_start = i + 1;
        } else {
            break;
        }
    }

    let mut final_end = trimmed.len();
    for (i, line) in trimmed.iter().enumerate().rev() {
        if line.chars().all(|c| c == 'X' || c == '_') && line.chars().filter(|&c| c == 'X').count() >= 5 {
            final_end = i;
        } else {
            break;
        }
    }

    let trimmed = &trimmed[final_start..final_end.max(final_start)];

    let debug = format!(
        "palette={}, cell_w={:.1}px, n_rows={}, trimmed={}..{}",
        palette.name, cell_w, n_rows, start, end,
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

        let img: image::RgbaImage = image::RgbaImage::from_raw(w, h, capture.as_raw().to_vec())
            .ok_or("Failed to convert captured image")?;

        // Downsample for overlay (PNG, lossless)
        let scale = 2u32;
        let sw = w / scale;
        let sh = h / scale;
        let small = image::imageops::resize(&img, sw, sh, image::imageops::FilterType::Nearest);
        let small_rgb = image::DynamicImage::ImageRgba8(small).to_rgb8();
        let mut png_buf = Cursor::new(Vec::new());
        {
            let mut encoder = PngEncoder::new(&mut png_buf);
            encoder
                .write_image(small_rgb.as_raw(), sw, sh, image::ExtendedColorType::Rgb8)
                .map_err(|e| format!("Failed to encode PNG: {}", e))?;
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(png_buf.into_inner());
        let data_url = format!("data:image/png;base64,{}", b64);

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
    use crate::recognition::{match_piece_color_with_palette, rgb_to_hsl, PALETTE_JSTRIS};

    #[test]
    fn test_rgb_to_hsl_red() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 100.0).abs() < 1.0);
        assert!((l - 50.0).abs() < 2.0);
    }

    #[test]
    fn test_jstris_color_matching() {
        use crate::recognition::match_piece_color_with_palette;

        let color_tests: &[(u8, u8, u8, char)] = &[
            (17, 149, 205, 'I'),   // 1195cd cyan
            (227, 159, 2, 'O'),    // e39f02 yellow
            (175, 41, 138, 'T'),   // af298a purple
            (89, 177, 1, 'S'),     // 59b101 green
            (215, 15, 55, 'Z'),    // d70f37 red
            (33, 65, 198, 'J'),    // 2141c6 blue
            (227, 91, 2, 'L'),     // e35b02 orange
            (153, 153, 153, 'X'),  // 999999 garbage
            (10, 10, 10, '_'),     // empty
        ];

        for (r, g, b, expected) in color_tests {
            let result = match_piece_color_with_palette(*r, *g, *b, &PALETTE_JSTRIS);
            assert_eq!(
                result, *expected,
                "rgb({},{},{}) → expected '{}', got '{}'",
                r, g, b, expected, result
            );
        }
    }

    #[test]
    fn test_garbage_rows() { /* ... */ }
}
