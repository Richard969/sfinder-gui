use std::path::Path;

/// Test that the field recognizer can process a full Tetris board screenshot
/// and return a valid field string with 10-column rows.
#[test]
fn test_recognize_full_board() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/board_full.webp");
    let result = sfinder_gui_lib::recognition::recognize_field_from_file(
        path.to_str().unwrap(),
    );
    assert!(result.is_ok(), "Full board recognition failed: {:?}", result.err());

    let field = result.unwrap();
    let lines: Vec<&str> = field.trim().lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty(), "Empty field result");

    // All rows should be 10 columns
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(
            line.len(),
            10,
            "Row {} has {} columns (expected 10): {}",
            i,
            line.len(),
            line
        );
    }

    // Should have detected some non-empty cells
    let has_pieces = field.chars().any(|c| matches!(c, 'I'|'O'|'T'|'S'|'Z'|'J'|'L'|'X'));
    assert!(has_pieces, "No pieces detected in full board");
}

/// Test that the recognizer handles a partially-filled board
#[test]
fn test_recognize_partial_board() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/board_partial.webp");
    let result = sfinder_gui_lib::recognition::recognize_field_from_file(
        path.to_str().unwrap(),
    );
    assert!(result.is_ok(), "Partial board recognition failed: {:?}", result.err());

    let field = result.unwrap();
    let lines: Vec<&str> = field.trim().lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty(), "Empty field result");

    for (i, line) in lines.iter().enumerate() {
        assert_eq!(line.len(), 10, "Row {} has {} columns", i, line.len());
    }

    let has_pieces = field.chars().any(|c| matches!(c, 'I'|'O'|'T'|'S'|'Z'|'J'|'L'|'X'));
    assert!(has_pieces, "No pieces detected in partial board");
}

/// Test that the recognizer handles a nearly-empty board
#[test]
fn test_recognize_empty_board() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/board_empty.webp");
    let result = sfinder_gui_lib::recognition::recognize_field_from_file(
        path.to_str().unwrap(),
    );
    assert!(result.is_ok(), "Empty board recognition failed: {:?}", result.err());

    let field = result.unwrap();
    let lines: Vec<&str> = field.trim().lines().filter(|l| !l.is_empty()).collect();
    if !lines.is_empty() {
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line.len(), 10, "Row {} has {} columns", i, line.len());
        }
    }
    // Empty-ish board may still detect some pieces from the bottom stack
}
