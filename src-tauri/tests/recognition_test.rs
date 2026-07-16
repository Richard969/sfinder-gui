use std::path::Path;

#[test]
fn test_recognize_tetr_io_board() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/tetr_io_test1.png");
    if !path.exists() {
        println!("Skipping test: fixture not found");
        return;
    }
    let result = sfinder_gui_lib::recognition::recognize_field_from_file(path.to_str().unwrap());
    match result {
        Ok(field) => println!("Result:\n{}", field),
        Err(e) => panic!("Failed: {}", e),
    }
}

#[test]
fn test_recognize_all_black_board() {
    let img = image::RgbImage::from_fn(10, 10, |_, _| image::Rgb([18u8, 18, 18]));
    let result = sfinder_gui_lib::recognition::recognize_field_simple(&img);
    match result {
        Ok(field) => assert!(field.chars().all(|c| c == '_' || c == '\n'), "got: {}", field),
        Err(_) => {}
    }
}

#[test]
fn test_tetr_io_color_matching() {
    // Test match_piece_color directly for precise results
    use sfinder_gui_lib::recognition::match_piece_color;

    let color_tests: &[(u8, u8, u8, char)] = &[
        (52, 181, 133, 'I'),  // cyan
        (179, 153, 50, 'O'),  // yellow
        (164, 62, 154, 'T'),  // purple
        (131, 179, 50, 'S'),  // green
        (210, 75, 85, 'Z'),   // red
        (79, 62, 164, 'J'),   // blue
        (178, 98, 49, 'L'),   // orange
        (140, 140, 145, 'X'), // garbage grey
        (10, 10, 10, '_'),    // empty black
    ];

    for (r, g, b, expected) in color_tests {
        let result = match_piece_color(*r, *g, *b);
        assert_eq!(
            result, *expected,
            "rgb({},{},{}) → expected '{}', got '{}'",
            r, g, b, expected, result
        );
    }
}

/// Test that garbage rows are correctly identified.
/// Simulates a tetr.io board with:
/// - Bottom 13 rows of grey garbage (with 3D bevel effect)
/// - Middle 4 rows of colored blocks (like the user's screenshot)
/// - Top 6 rows empty
#[test]
fn test_recognize_garbage_rows() {
    use sfinder_gui_lib::recognition::recognize_field;

    // Board dimensions: 10 cols x 23 rows
    let cell_size = 32u32;
    let cols = 10u32;
    let rows = 23u32;
    let width = cols * cell_size;
    let height = rows * cell_size;

    let img = image::RgbImage::from_fn(width, height, |x, y| {
        let row = y / cell_size; // 0=top, 22=bottom
        let col = x / cell_size;
        let cx = x % cell_size;
        let cy = y % cell_size;

        // Top 6 rows: empty (dark background)
        if row < 6 {
            return image::Rgb([20, 20, 25]);
        }

        // Bottom 13 rows (row 6-18): grey garbage with bevel
        if row < 19 {
            // Simple 3D bevel: brighter in center, darker at edges
            let bevel = if cx < 3 || cy < 3 || cx >= cell_size - 3 || cy >= cell_size - 3 {
                100 // dark edge
            } else {
                160 // bright center
            };
            return image::Rgb([bevel, bevel, bevel + 5]);
        }

        // Middle 4 rows (row 19-22): colored blocks
        // Row 22 (bottom): I Z Z _ T _ L L T T (from user's expected result)
        match row {
            22 => match col {
                0 => image::Rgb([52, 181, 133]),   // I
                1 => image::Rgb([180, 52, 59]),    // Z
                2 => image::Rgb([180, 52, 59]),    // Z
                3 => image::Rgb([20, 20, 25]),     // _
                4 => image::Rgb([164, 62, 154]),   // T
                5 => image::Rgb([20, 20, 25]),     // _
                6 => image::Rgb([178, 98, 49]),    // L
                7 => image::Rgb([178, 98, 49]),    // L
                8 => image::Rgb([164, 62, 154]),   // T
                9 => image::Rgb([164, 62, 154]),   // T
                _ => image::Rgb([20, 20, 25]),
            },
            21 => match col {
                0 => image::Rgb([180, 52, 59]),    // Z
                1 => image::Rgb([180, 52, 59]),    // Z
                6 => image::Rgb([79, 62, 164]),    // J
                7 => image::Rgb([79, 62, 164]),    // J
                8 => image::Rgb([79, 62, 164]),    // J
                9 => image::Rgb([164, 62, 154]),   // T
                _ => image::Rgb([20, 20, 25]),
            },
            20 => match col {
                6 => image::Rgb([79, 62, 164]),    // J
                8 => image::Rgb([179, 153, 50]),   // O
                9 => image::Rgb([179, 153, 50]),   // O
                _ => image::Rgb([20, 20, 25]),
            },
            19 => match col {
                8 => image::Rgb([179, 153, 50]),   // O
                9 => image::Rgb([179, 153, 50]),   // O
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

    // The bottom 4 (colored) rows should contain I, Z, T, L, J, O
    // (recognition scans bottom-to-top, so these appear first)
    assert!(
        lines[0..4].iter().any(|l| l.contains('I') || l.contains('Z') || l.contains('T')),
        "Bottom 4 lines should contain colored blocks: {:?}", &lines[0..4]
    );

    // Garbage rows (X) should appear somewhere
    let max_x_count = lines.iter().filter(|l| l.chars().all(|c| c == 'X')).count();
    assert!(max_x_count >= 8, "Expected >= 8 all-X lines (garbage), got {}", max_x_count);
}
