#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Kick offsets keyed by (piece_type, from_rot, to_rot)
/// from_rot/to_rot: 0=spawn(N), 1=right(E), 2=reverse(S), 3=left(W)
pub type KickKey = (char, usize, usize);

#[allow(dead_code)]
pub struct KickTable {
    kicks: HashMap<KickKey, Vec<(i32, i32)>>,
}

impl KickTable {
    /// Get kick offsets for a rotation transition, or empty if none defined
    pub fn get(&self, piece: char, from: usize, to: usize) -> &[(i32, i32)] {
        // Try exact match first, fall back to JLSTZ table for any piece
        if let Some(v) = self.kicks.get(&(piece, from, to)) {
            return v;
        }
        // Fallback: try 'L' as the reference (most tables define L first and reference it)
        if piece != 'L' {
            if let Some(v) = self.kicks.get(&('L', from, to)) {
                return v;
            }
        }
        &[]
    }

}

/// Direction string → (from_rot, to_rot)
fn parse_direction(dir: &str) -> Option<(usize, usize)> {
    match dir {
        "NE" => Some((0, 1)),
        "ES" => Some((1, 2)),
        "SW" => Some((2, 3)),
        "WN" => Some((3, 0)),
        "NW" => Some((0, 3)),
        "WS" => Some((3, 2)),
        "SE" => Some((2, 1)),
        "EN" => Some((1, 0)),
        // 180° rotations
        "NS" => Some((0, 2)),
        "EW" => Some((1, 3)),
        "SN" => Some((2, 0)),
        "WE" => Some((3, 1)),
        _ => None,
    }
}

/// Parse "(dx,dy)" pairs — strips leading `@` prefix before numbers
fn parse_offsets(raw: &str) -> Vec<(i32, i32)> {
    let mut offsets = Vec::new();
    let s = raw.trim();

    if s.is_empty() {
        return offsets;
    }

    // Split on ")(" boundaries: "(dx,dy)(dx,dy)..."
    let inner = s.trim_start_matches('(').trim_end_matches(')');
    for part in inner.split(")(") {
        let clean = part.trim().trim_start_matches('(').trim_end_matches(')');
        // Strip @ prefix (T-piece fin kicks)
        let clean = clean.trim_start_matches('@').trim();
        if let Some(comma) = clean.find(',') {
            let xs = clean[..comma].trim().trim_start_matches('+');
            let ys = clean[comma + 1..].trim().trim_start_matches('+');
            if let (Ok(x), Ok(y)) = (xs.parse::<i32>(), ys.parse::<i32>()) {
                offsets.push((x, y));
            }
        }
    }

    offsets
}

/// Parse a sfinder .properties kick table file.
/// Format:
///   Piece.DIR=(dx,dy)(dx,dy)...
///   Piece.DIR=&OtherPiece.DIR    (reference)
pub fn parse_kick_file(path: &str) -> Result<KickTable, String> {
    let content = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("Failed to read kick file '{}': {}", path, e))?;

    // First pass: collect all direct definitions
    let mut direct: HashMap<(char, usize, usize), Vec<(i32, i32)>> = HashMap::new();
    // Second pass: resolve references
    let mut refs: Vec<(char, usize, usize, char, usize, usize)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // Parse: Piece.DIR=value
        let Some(eq) = line.find('=') else { continue };
        let left = line[..eq].trim();
        let right = line[eq + 1..].trim();

        // Parse left: Piece.DIR
        let mut left_parts = left.splitn(2, '.');
        let piece_name = left_parts.next().unwrap_or("").trim();
        let dir_name = left_parts.next().unwrap_or("").trim();

        if piece_name.len() != 1 || dir_name.len() < 2 {
            continue;
        }
        let piece = piece_name.chars().next().unwrap();
        let Some((from, to)) = parse_direction(dir_name) else {
            continue;
        };

        // Check for reference
        if right.starts_with('&') {
            let ref_target = right.strip_prefix('&').unwrap_or(right).trim(); // e.g. "L.NE"
            let mut ref_parts = ref_target.splitn(2, '.');
            let ref_piece = ref_parts.next().unwrap_or("").trim();
            let ref_dir = ref_parts.next().unwrap_or("").trim();
            if ref_piece.len() == 1 {
                let ref_piece_char = ref_piece.chars().next().unwrap();
                if let Some((ref_from, ref_to)) = parse_direction(ref_dir) {
                    refs.push((piece, from, to, ref_piece_char, ref_from, ref_to));
                    continue;
                }
            }
        }

        // Direct definition
        let offsets = parse_offsets(right);
        direct.insert((piece, from, to), offsets);
    }

    // Resolve references
    let mut kicks: HashMap<KickKey, Vec<(i32, i32)>> = direct;

    for (piece, from, to, ref_piece, ref_from, ref_to) in refs {
        let offsets = kicks
            .get(&(ref_piece, ref_from, ref_to))
            .cloned()
            .unwrap_or_default();
        kicks.insert((piece, from, to), offsets);
    }

    Ok(KickTable { kicks })
}

/// Built-in SRS kick table (fallback when no file provided)
pub fn srs_kick_table() -> KickTable {
    let mut kicks = HashMap::new();

    // JLSTZ (except I, O)
    for piece in ['J', 'L', 'S', 'Z', 'T'] {
        // NE: 0→1
        kicks.insert((piece, 0, 1), vec![(0,0),(-1,0),(-1,1),(0,-2),(-1,-2)]);
        // ES: 1→2
        kicks.insert((piece, 1, 2), vec![(0,0),(1,0),(1,-1),(0,2),(1,2)]);
        // SW: 2→3
        kicks.insert((piece, 2, 3), vec![(0,0),(1,0),(1,1),(0,-2),(1,-2)]);
        // WN: 3→0
        kicks.insert((piece, 3, 0), vec![(0,0),(-1,0),(-1,-1),(0,2),(-1,2)]);
        // NW: 0→3
        kicks.insert((piece, 0, 3), vec![(0,0),(1,0),(1,1),(0,-2),(1,-2)]);
        // WS: 3→2
        kicks.insert((piece, 3, 2), vec![(0,0),(-1,0),(-1,-1),(0,2),(-1,2)]);
        // SE: 2→1
        kicks.insert((piece, 2, 1), vec![(0,0),(-1,0),(-1,1),(0,-2),(-1,-2)]);
        // EN: 1→0
        kicks.insert((piece, 1, 0), vec![(0,0),(1,0),(1,-1),(0,2),(1,2)]);
    }

    // I-piece
    kicks.insert(('I', 0, 1), vec![(0,0),(-2,0),(1,0),(-2,-1),(1,2)]);
    kicks.insert(('I', 1, 2), vec![(0,0),(-1,0),(2,0),(-1,2),(2,-1)]);
    kicks.insert(('I', 2, 3), vec![(0,0),(2,0),(-1,0),(2,1),(-1,-2)]);
    kicks.insert(('I', 3, 0), vec![(0,0),(1,0),(-2,0),(1,-2),(-2,1)]);
    kicks.insert(('I', 0, 3), vec![(0,0),(-1,0),(2,0),(-1,2),(2,-1)]);
    kicks.insert(('I', 3, 2), vec![(0,0),(-2,0),(1,0),(-2,-1),(1,2)]);
    kicks.insert(('I', 2, 1), vec![(0,0),(1,0),(-2,0),(1,-2),(-2,1)]);
    kicks.insert(('I', 1, 0), vec![(0,0),(2,0),(-1,0),(2,1),(-1,-2)]);

    KickTable { kicks }
}
