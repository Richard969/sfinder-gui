use sfinder_gui_lib::recognition::{
    match_piece_color_with_palette, recognize_field, recognize_field_from_file, PALETTE_TETR_IO,
};
use std::path::Path;

#[test]
fn test_recognize_board_1() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_1.png");
    if !path.exists() {
        println!("Skipping test: fixture not found");
        return;
    }
    let field = recognize_field_from_file(path.to_str().unwrap()).expect("Recognition failed");
    println!("board_1 result:\n{}", field);
    let lines: Vec<&str> = field.lines().collect();
    assert!(lines.len() >= 4, "Expected >= 4 lines, got {}: {}", lines.len(), field);
}

#[test]
fn test_recognize_board_tki() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_tki.png");
    if !path.exists() {
        println!("Skipping test: fixture not found");
        return;
    }
    let field = recognize_field_from_file(path.to_str().unwrap()).expect("Recognition failed");
    println!("board_tki result:\n{}", field);
    let lines: Vec<&str> = field.lines().collect();
    assert!(lines.len() >= 4, "Expected >= 4 lines, got {}: {}", lines.len(), field);
}

#[test]
fn test_recognize_board_2() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_2.png");
    if !path.exists() {
        println!("Skipping test: fixture not found");
        return;
    }
    let field = recognize_field_from_file(path.to_str().unwrap()).expect("Recognition failed");
    println!("board_2 result:\n{}", field);
    let lines: Vec<&str> = field.lines().collect();
    assert!(lines.len() >= 4, "Expected >= 4 lines, got {}: {}", lines.len(), field);
}

#[test]
fn test_recognize_all_black_board() {
    let img = image::RgbImage::from_fn(10, 10, |_, _| image::Rgb([18u8, 18, 18]));
    let result = sfinder_gui_lib::recognition::recognize_field_simple(&img);
    match result {
        Ok(field) => assert!(
            field.chars().all(|c| c == '_' || c == '\n'),
            "got: {}",
            field
        ),
        Err(_) => {}
    }
}

#[test]
fn test_tetr_io_color_matching() {
    let color_tests: &[(u8, u8, u8, char)] = &[
        (52, 181, 133, 'I'),  // 34b585 teal
        (179, 153, 50, 'O'),  // b39932 yellow
        (164, 62, 154, 'T'),  // a43e9a purple
        (131, 179, 50, 'S'),  // 83b332 green
        (180, 52, 59, 'Z'),   // b4343b red
        (79, 62, 164, 'J'),   // 4f3ea4 blue
        (178, 98, 49, 'L'),   // b26231 orange
        (67, 67, 67, 'X'),    // 434343 garbage
        (10, 10, 10, '_'),    // empty black
    ];

    for (r, g, b, expected) in color_tests {
        let result = match_piece_color_with_palette(*r, *g, *b, &PALETTE_TETR_IO);
        assert_eq!(
            result, *expected,
            "rgb({},{},{}) → expected '{}', got '{}'",
            r, g, b, expected, result
        );
    }
}

#[test]
fn test_recognize_garbage_rows() {
    // 10×23 board, cell_size=32
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
            return image::Rgb([20, 20, 25]);
        }

        if row < 19 {
            let bevel = if cx < 3 || cy < 3 || cx >= cell_size - 3 || cy >= cell_size - 3 {
                100
            } else {
                160
            };
            return image::Rgb([bevel, bevel, bevel + 5]);
        }

        match row {
            22 => match col {
                0 => image::Rgb([52, 181, 133]),  // I
                1 => image::Rgb([180, 52, 59]),   // Z
                2 => image::Rgb([180, 52, 59]),   // Z
                3 => image::Rgb([20, 20, 25]),    // _
                4 => image::Rgb([164, 62, 154]),  // T
                5 => image::Rgb([20, 20, 25]),    // _
                6 => image::Rgb([178, 98, 49]),   // L
                7 => image::Rgb([178, 98, 49]),   // L
                8 => image::Rgb([164, 62, 154]),  // T
                9 => image::Rgb([164, 62, 154]),  // T
                _ => image::Rgb([20, 20, 25]),
            },
            21 => match col {
                0 => image::Rgb([180, 52, 59]),   // Z
                1 => image::Rgb([180, 52, 59]),   // Z
                6 => image::Rgb([79, 62, 164]),   // J
                7 => image::Rgb([79, 62, 164]),   // J
                8 => image::Rgb([79, 62, 164]),   // J
                9 => image::Rgb([164, 62, 154]),  // T
                _ => image::Rgb([20, 20, 25]),
            },
            20 => match col {
                6 => image::Rgb([79, 62, 164]),   // J
                8 => image::Rgb([179, 153, 50]),  // O
                9 => image::Rgb([179, 153, 50]),  // O
                _ => image::Rgb([20, 20, 25]),
            },
            19 => match col {
                8 => image::Rgb([179, 153, 50]),  // O
                9 => image::Rgb([179, 153, 50]),  // O
                _ => image::Rgb([20, 20, 25]),
            },
            _ => image::Rgb([20, 20, 25]),
        }
    });

    let (field, debug) = recognize_field(&img).expect("Recognition failed");
    println!("Debug: {}", debug);
    println!("Result:\n{}", field);

    let lines: Vec<&str> = field.lines().collect();
    assert!(lines.len() >= 16, "Expected >= 16 lines, got {}: {}", lines.len(), field);

    let max_x_count = lines.iter().filter(|l| l.chars().all(|c| c == 'X')).count();
    assert!(max_x_count >= 8, "Expected >= 8 all-X lines (garbage), got {}", max_x_count);
}
