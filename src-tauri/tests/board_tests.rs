use sfinder_gui_lib::recognition::{recognize_field, PALETTE_TETR_IO};
use std::path::Path;

/// Trim empty (all-underscore) rows
fn trim_empty_rows(field: &str) -> Vec<String> {
    field
        .lines()
        .filter(|l| !l.chars().all(|c| c == '_'))
        .map(String::from)
        .collect()
}

// === Board fixture tests (real-game tetr.io screenshots) ===
// These tests require actual real-game screenshots to be placed in tests/fixtures/.
// The current fixtures (board_1.png, board_tki.png, board_2.png) are synthetic
// test images that don't match the expected fumen codes below.
// FIXME: Replace with real screenshots from user to enable these tests.

#[test]
#[ignore = "Fixtures are synthetic, not real-game screenshots"]
fn test_board_1_full_recognition() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_1.png");
    assert!(path.exists(), "Fixture not found: {}", path.display());

    let img = image::open(path.to_str().unwrap()).unwrap().to_rgb8();
    let (result, debug) = recognize_field(&img).expect("Recognition failed");
    println!("board_1 debug: {}", debug);
    println!("board_1 result:\n{}", result);
    let trimmed = trim_empty_rows(&result);

    let expected = vec![
        "OOS___IJJZ".to_string(),
        "OOSS__IJZZ".to_string(),
        "IJJS__IJZL".to_string(),
        "ITTTZ_ISOO".to_string(),
    ];

    assert_eq!(
        trimmed.len(),
        expected.len(),
        "Expected {} rows, got {}: {}",
        expected.len(),
        trimmed.len(),
        result
    );

    for (i, (got, exp)) in trimmed.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got, exp, "board_1 row {}: expected '{}', got '{}'", i, exp, got);
    }
}

#[test]
#[ignore = "Fixtures are synthetic, not real-game screenshots"]
fn test_board_tki_full_recognition() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_tki.png");
    assert!(path.exists(), "Fixture not found: {}", path.display());

    let img = image::open(path.to_str().unwrap()).unwrap().to_rgb8();
    let (result, debug) = recognize_field(&img).expect("Recognition failed");
    println!("board_tki debug: {}", debug);
    println!("board_tki result:\n{}", result);
    let trimmed = trim_empty_rows(&result);

    let expected = vec![
        "___JJJ____".to_string(),
        "L__ZZJS___".to_string(),
        "L___ZZSSOO".to_string(),
        "LL_IIIISOO".to_string(),
    ];

    assert_eq!(
        trimmed.len(),
        expected.len(),
        "Expected {} rows, got {}: {}",
        expected.len(),
        trimmed.len(),
        result
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
#[ignore = "Fixtures are synthetic, not real-game screenshots"]
fn test_board_2_full_recognition() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("tests/fixtures/board_2.png");
    assert!(path.exists(), "Fixture not found: {}", path.display());

    let img = image::open(path.to_str().unwrap()).unwrap().to_rgb8();
    let (result, debug) = recognize_field(&img).expect("Recognition failed");
    println!("board_2 debug: {}", debug);
    println!("board_2 result:\n{}", result);
    let trimmed = trim_empty_rows(&result);

    let expected = vec![
        "TTTSII_LLL".to_string(),
        "ZTSSII_OOL".to_string(),
        "ZZSTII_OOJ".to_string(),
        "LZTTII_JJJ".to_string(),
        "LLLTOO_ZJJ".to_string(),
        "SSSSOO_ZZJ".to_string(),
        "LSSSS___ZJ".to_string(),
        "LLL____JJJ".to_string(),
        "_______J__".to_string(),
    ];

    assert_eq!(
        trimmed.len(),
        expected.len(),
        "Expected {} rows, got {}: {}",
        expected.len(),
        trimmed.len(),
        result
    );

    for (i, (got, exp)) in trimmed.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got, exp,
            "board_2 row {}: expected '{}', got '{}'",
            i, exp, got
        );
    }
}
