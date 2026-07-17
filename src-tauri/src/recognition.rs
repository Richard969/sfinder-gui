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

/// Tetr.io default skin
pub const PALETTE_TETR_IO: ColorPalette = ColorPalette {
    name: "tetr.io",
    colors: &[
        (0, 0, 0, '_'),
        (52, 181, 133, 'I'),
        (179, 153, 50, 'O'),
        (164, 62, 154, 'T'),
        (131, 179, 50, 'S'),
        (180, 52, 59, 'Z'),
        (79, 62, 164, 'J'),
        (178, 98, 49, 'L'),
        (67, 67, 67, 'X'),
    ],
};

/// Jstris default skin
pub const PALETTE_JSTRIS: ColorPalette = ColorPalette {
    name: "jstris",
    colors: &[
        (0, 0, 0, '_'),
        (17, 149, 205, 'I'),
        (227, 159, 2, 'O'),
        (175, 41, 138, 'T'),
        (89, 177, 1, 'S'),
        (215, 15, 55, 'Z'),
        (33, 65, 198, 'J'),
        (227, 91, 2, 'L'),
        (153, 153, 153, 'X'),
    ],
};

pub const PALETTES: &[ColorPalette] = &[PALETTE_TETR_IO, PALETTE_JSTRIS];

fn detect_palette(img: &RgbImage) -> &'static ColorPalette {
    let (width, height) = img.dimensions();

    // Collect high-saturation samples from the middle 3 rows of the board
    let y_start = height * 3 / 5;
    let y_end = height * 4 / 5;

    let mut samples: Vec<(u8, u8, u8)> = Vec::new();
    for y in y_start..y_end {
        for x in 0..width {
            let px = img.get_pixel(x, y);
            let (_, s, l) = rgb_to_hsl(px[0], px[1], px[2]);
            if s > 25.0 && l > 25.0 && l < 80.0 {
                samples.push((px[0], px[1], px[2]));
            }
        }
    }
    if samples.len() < 10 {
        return &PALETTE_TETR_IO;
    }

    // For each palette, compute average distance from samples to nearest non-grey color
    let mut best_palette = &PALETTE_TETR_IO as &'static ColorPalette;
    let mut best_score = f64::MAX;

    for palette in PALETTES {
        let mut total_dist = 0.0;
        let mut count = 0;
        for &(ref_r, ref_g, ref_b) in samples.iter().take(200) {
            // Skip very grey samples
            let (_, s, _) = rgb_to_hsl(ref_r, ref_g, ref_b);
            if s < 15.0 {
                continue;
            }
            let mut min_dist = f64::MAX;
            let (y1, u1, v1) = rgb_to_yuv(ref_r, ref_g, ref_b);
            for &(pr, pg, pb, pc) in palette.colors.iter() {
                if pc == '_' || pc == 'X' {
                    continue;
                }
                let (y2, u2, v2) = rgb_to_yuv(pr, pg, pb);
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

pub fn match_piece_color_with_palette(r: u8, g: u8, b: u8, palette: &ColorPalette) -> char {
    let (_, s, l) = rgb_to_hsl(r, g, b);
    let max_c = r.max(g).max(b);
    let min_c = r.min(g).min(b);
    let rgb_spread = max_c - min_c;

    if s < 12.0 || rgb_spread < 15 {
        if l > 20.0 && l < 75.0 && rgb_spread < 25 && l > 30.0 {
            return 'X';
        }
        if l <= 25.0 {
            return '_';
        }
        return 'X';
    }

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
    if best_dist < 0.3 {
        return best;
    }

    if l > 20.0 {
        'X'
    } else {
        '_'
    }
}

pub fn detect_board_region(img: &RgbImage) -> (u32, u32) {
    let (width, height) = img.dimensions();
    if height < 10 {
        return (0, height);
    }

    let mut y_bottom = height;
    let mut y_top = 0;

    for y in (0..height).rev() {
        let mut non_empty = 0;
        for x in 0..width {
            let px = img.get_pixel(x, y);
            let (_, _, l) = rgb_to_hsl(px[0], px[1], px[2]);
            if l > 30.0 {
                non_empty += 1;
            }
        }
        if non_empty > 8 {
            y_bottom = y + 1;
            break;
        }
    }

    for y in 0..y_bottom {
        let mut non_empty = 0;
        for x in 0..width {
            let px = img.get_pixel(x, y);
            let (_, _, l) = rgb_to_hsl(px[0], px[1], px[2]);
            if l > 30.0 {
                non_empty += 1;
            }
        }
        if non_empty > 8 {
            y_top = y;
            break;
        }
    }

    (y_top, y_bottom)
}

fn detect_cell_width(img: &RgbImage) -> f64 {
    let (width, height) = img.dimensions();
    if width < 10 || height < 10 {
        return width as f64 / 10.0;
    }
    // Simple approach: image width / 10
    width as f64 / 10.0
}

pub fn recognize_field(img: &RgbImage) -> Result<(String, String), String> {
    let (width, height) = img.dimensions();
    if width < 10 || height < 10 {
        return Err("Image too small".to_string());
    }

    let (y_top, y_bottom) = detect_board_region(img);
    let board_height = y_bottom - y_top;
    if board_height < 10 {
        return Err("Could not detect board region".to_string());
    }

    let palette = detect_palette(img);
    let cell_w = detect_cell_width(img);
    let n_rows = ((board_height as f64 / cell_w).round() as usize).clamp(1, 40);

    let mut raw_lines: Vec<String> = Vec::new();

    for row in (0..n_rows).rev() {
        let y_center_f = y_top as f64 + (row as f64 + 0.5) * cell_w;
        let y_center = (y_center_f as u32).min(height - 1);

        let mut line = String::with_capacity(NUM_COLS);
        for col in 0..NUM_COLS {
            let x_left = col as f64 * cell_w;
            let x_right = (col + 1) as f64 * cell_w;
            let x_center_f = (x_left + x_right) / 2.0;
            let x_center = (x_center_f as u32).min(width - 1);

            let px = img.get_pixel(x_center, y_center);
            let ch = match_piece_color_with_palette(px[0], px[1], px[2], palette);
            line.push(ch);
        }
        raw_lines.push(line);
    }

    // Trim all-underscore rows
    let mut start = 0;
    while start < raw_lines.len() && raw_lines[start].chars().all(|c| c == '_') {
        start += 1;
    }
    if start == raw_lines.len() {
        return Err("Board appears empty".to_string());
    }
    let mut end = raw_lines.len();
    while end > start && raw_lines[end - 1].chars().all(|c| c == '_') {
        end -= 1;
    }

    let trimmed: Vec<&str> = raw_lines[start..end]
        .iter()
        .rev()
        .map(|s| s.as_str())
        .collect();

    let debug = format!(
        "palette={}, cell_w={:.1}px, n_rows={}, trimmed={}..{}",
        palette.name,
        cell_w,
        n_rows,
        start,
        end,
    );

    Ok((trimmed.join("\n"), debug))
}

pub fn recognize_field_simple(img: &RgbImage) -> Result<String, String> {
    recognize_field(img).map(|(field, _)| field)
}

pub fn recognize_field_from_file(path: &str) -> Result<String, String> {
    let img = image::open(path)
        .map_err(|e| format!("Failed to open '{}': {}", path, e))?
        .to_rgb8();
    recognize_field_simple(&img)
}

pub fn recognize_field_from_bytes(bytes: &[u8]) -> Result<String, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?
        .to_rgb8();
    recognize_field_simple(&img)
}

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

        let scale = 2u32;
        let sw = w / scale;
        let sh = h / scale;
        let small = image::imageops::resize(&img, sw, sh, image::imageops::FilterType::Nearest);
        let small_rgb = image::DynamicImage::ImageRgba8(small).to_rgb8();
        let mut png_buf = Cursor::new(Vec::new());
        {
            let encoder = PngEncoder::new(&mut png_buf);
            encoder
                .write_image(
                    small_rgb.as_raw(),
                    sw,
                    sh,
                    image::ExtendedColorType::Rgb8,
                )
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
        .unwrap_or_else(std::env::temp_dir)
        .join("sfinder_cropped_debug.png");
    if let Err(e) = cropped.save(&debug_path) {
        eprintln!("Failed to save debug image: {}", e);
    }

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

    recognize_field_simple(&rgb)
}

pub fn clear_capture() {
    if let Ok(mut guard) = CAPTURE.lock() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::recognition::*;

    #[test]
    fn test_rgb_to_hsl_red() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 100.0).abs() < 1.0);
        assert!((l - 50.0).abs() < 2.0);
    }

    #[test]
    fn test_jstris_color_matching() {
        let color_tests: &[(u8, u8, u8, char)] = &[
            (17, 149, 205, 'I'),
            (227, 159, 2, 'O'),
            (175, 41, 138, 'T'),
            (89, 177, 1, 'S'),
            (215, 15, 55, 'Z'),
            (33, 65, 198, 'J'),
            (227, 91, 2, 'L'),
            (153, 153, 153, 'X'),
            (10, 10, 10, '_'),
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
    fn test_garbage_rows() {
        use crate::recognition::{recognize_field, rgb_to_hsl, PALETTE_TETR_IO, match_piece_color_with_palette};

        let cell_size = 32u32;
        let cols = 10u32;
        let rows = 23u32;
        let width = cols * cell_size;
        let height = rows * cell_size;

        let img = image::RgbImage::from_fn(width, height, |x, y| {
            let row = y / cell_size;
            let col = x / cell_size;
            let cx = x % cell_size;
            let cy = y % cell_size;

            if row < 6 {
                return image::Rgb([10, 10, 15]);
            }

            if row < 19 {
                let bevel = if cx < 3 || cy < 3 || cx >= cell_size - 3 || cy >= cell_size - 3 {
                    80
                } else {
                    150
                };
                return image::Rgb([bevel, bevel, bevel + 5]);
            }

            match row {
                22 => match col {
                    0 => image::Rgb([52, 181, 133]),
                    1 => image::Rgb([180, 52, 59]),
                    2 => image::Rgb([180, 52, 59]),
                    3 => image::Rgb([10, 10, 15]),
                    4 => image::Rgb([164, 62, 154]),
                    5 => image::Rgb([10, 10, 15]),
                    6 => image::Rgb([178, 98, 49]),
                    7 => image::Rgb([178, 98, 49]),
                    8 => image::Rgb([164, 62, 154]),
                    9 => image::Rgb([164, 62, 154]),
                    _ => image::Rgb([10, 10, 15]),
                },
                21 => match col {
                    0 => image::Rgb([180, 52, 59]),
                    1 => image::Rgb([180, 52, 59]),
                    6 => image::Rgb([79, 62, 164]),
                    7 => image::Rgb([79, 62, 164]),
                    8 => image::Rgb([79, 62, 164]),
                    9 => image::Rgb([164, 62, 154]),
                    _ => image::Rgb([10, 10, 15]),
                },
                20 => match col {
                    6 => image::Rgb([79, 62, 164]),
                    8 => image::Rgb([179, 153, 50]),
                    9 => image::Rgb([179, 153, 50]),
                    _ => image::Rgb([10, 10, 15]),
                },
                19 => match col {
                    8 => image::Rgb([179, 153, 50]),
                    9 => image::Rgb([179, 153, 50]),
                    _ => image::Rgb([10, 10, 15]),
                },
                _ => image::Rgb([10, 10, 15]),
            }
        });

        let (field, debug) = recognize_field(&img).expect("Recognition failed");
        eprintln!("Debug: {}", debug);
        eprintln!("Result:\n{}", field);

        let lines: Vec<&str> = field.lines().collect();
        assert!(lines.len() >= 16, "Expected >= 16 lines, got {}: {}", lines.len(), field);

        let max_x_count = lines.iter().filter(|l| l.chars().all(|c| c == 'X')).count();
        assert!(max_x_count >= 8, "Expected >= 8 all-X lines (garbage), got {}", max_x_count);
    }
}
