use sfinder_gui_lib::recognition::{recognize_field, PALETTE_TETR_IO};
use std::path::Path;

/// Trim empty (all-underscore) rows and all-garbage rows from top
fn trim_rows(field: &str) -> Vec<String> {
    let lines: Vec<&str> = field.lines().collect();

    // Find first row that has at least one colored piece (not _ or X)
    let mut start = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.chars().any(|c| c != '_' && c != 'X') {
            start = i;
            break;
        }
    }

    // Find last row that has at least one colored piece
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().rev() {
        if line.chars().any(|c| c != '_' && c != 'X') {
            end = i + 1;
            break;
        }
    }

    lines[start..end].iter().map(|s| s.to_string()).collect()
}

// === Board fixture tests (real-game tetr.io screenshots) ===

#[test]
fn test_board_1_full_recognition() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_1.png");
    assert!(path.exists(), "Fixture not found: {}", path.display());

    let img = image::open(path.to_str().unwrap()).unwrap().to_rgb8();
    let (result, debug) = recognize_field(&img).expect("Recognition failed");
    eprintln!("board_1 debug: {}", debug);
    eprintln!("board_1 result:\n{}", result);
    let trimmed = trim_rows(&result);

    let expected = vec![
        "OOS___IJJZ".to_string(),
        "OOSS__IJZZ".to_string(),
        "IJJS__IJZL".to_string(),
        "ITTTZ_ISOO".to_string(),
    ];

    assert_eq!(
        trimmed.len(),
        expected.len(),
        "Expected {} rows, got {}: {:?}",
        expected.len(),
        trimmed.len(),
        trimmed
    );

    for (i, (got, exp)) in trimmed.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got, exp, "board_1 row {}: expected '{}', got '{}'", i, exp, got);
    }
}

#[test]
fn test_board_tki_full_recognition() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_tki.png");
    assert!(path.exists(), "Fixture not found: {}", path.display());

    let img = image::open(path.to_str().unwrap()).unwrap().to_rgb8();
    let (result, debug) = recognize_field(&img).expect("Recognition failed");
    eprintln!("board_tki debug: {}", debug);
    eprintln!("board_tki result:\n{}", result);
    let trimmed = trim_rows(&result);

    let expected = vec![
        "___JJJ____".to_string(),
        "L__ZZJS___".to_string(),
        "L___ZZSSOO".to_string(),
        "LL_IIIISOO".to_string(),
    ];

    assert_eq!(
        trimmed.len(),
        expected.len(),
        "Expected {} rows, got {}: {:?}",
        expected.len(),
        trimmed.len(),
        trimmed
    );

    for (i, (got, exp)) in trimmed.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got, exp,
            "board_tki row {}: expected '{}', got '{}'",
            i, exp, got
        );
    }
}

#[test]
fn test_board_garbage() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_garbage.png");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let img = image::open(path.to_str().unwrap()).unwrap().to_rgb8();
    let (result, debug) = recognize_field(&img).expect("Recognition failed");
    eprintln!("board_garbage debug: {}", debug);
    eprintln!("board_garbage result:\n{}", result);

    // Expected from fumen DfRpFeg0...: garbage rows at bottom, colored at top
    let trimmed = trim_rows(&result);
    assert!(trimmed.len() >= 4, "Expected >= 4 rows, got {}", trimmed.len());
}
