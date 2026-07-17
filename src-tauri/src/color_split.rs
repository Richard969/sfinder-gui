use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct PieceOperation {
    #[serde(rename = "type")]
    pub piece_type: String,
    pub rotation: String,
    pub x: i32,
    pub y: i32,
}

// --- Piece shapes (spawn rotation, from tetris-fumen getPieces) ---

fn spawn_shape(piece: char) -> Vec<(i32, i32)> {
    match piece {
        'I' => vec![(-1, 0), (0, 0), (1, 0), (2, 0)],
        'L' => vec![(-1, 0), (0, 0), (1, 0), (1, 1)],
        'O' => vec![(0, 0), (1, 0), (0, 1), (1, 1)],
        'Z' => vec![(0, 0), (1, 0), (-1, 1), (0, 1)],
        'T' => vec![(-1, 0), (0, 0), (1, 0), (0, 1)],
        'J' => vec![(-1, 0), (0, 0), (1, 0), (-1, 1)],
        'S' => vec![(-1, 0), (0, 0), (0, 1), (1, 1)],
        _ => vec![],
    }
}

const ROTATIONS: &[&str] = &["spawn", "right", "reverse", "left"];

fn rotate(positions: &[(i32, i32)], rotation: &str) -> Vec<(i32, i32)> {
    match rotation {
        "right" => positions.iter().map(|(x, y)| (*y, -*x)).collect(),
        "reverse" => positions.iter().map(|(x, y)| (-x, -y)).collect(),
        "left" => positions.iter().map(|(x, y)| (-*y, *x)).collect(),
        _ => positions.to_vec(),
    }
}

fn cell_positions(piece_type: char, rotation: &str, x: i32, y: i32) -> Vec<(i32, i32)> {
    let shape = spawn_shape(piece_type);
    rotate(&shape, rotation)
        .into_iter()
        .map(|(px, py)| (px + x, py + y))
        .collect()
}

// --- Bounds / overlap checks ---

fn in_bounds(op: &PieceOperation) -> bool {
    let cells = cell_positions(
        op.piece_type.chars().next().unwrap_or('_'),
        &op.rotation, op.x, op.y,
    );
    // x: 0-9, y: 0-22 (pieces must be fully within visible field)
    cells.iter().all(|(x, y)| *x >= 0 && *x <= 9 && *y >= 0 && *y <= 22)
}

fn overlaps(cells: &[(i32, i32)], occupied: &HashSet<String>) -> bool {
    cells.iter().any(|(x, y)| occupied.contains(&format!("{},{}", x, y)))
}

// --- Combined placement validity ---

/// Trace down a column through full rows to find real support.
/// Returns true if there's support (occupied cell or floor) below the given position,
/// skipping through full rows which will be cleared.
/// `other_piece_cells`: cells belonging to other unplaced pieces — these are NOT support.
fn trace_support(
    x: i32,
    start_y: i32,
    occupied: &HashSet<String>,
    full_rows: &HashSet<i32>,
    other_piece_cells: &HashSet<String>,
) -> bool {
    let mut y = start_y;
    while y >= 0 {
        let key = format!("{},{}", x, y);
        if occupied.contains(&key) {
            return true; // found existing block
        }
        if other_piece_cells.contains(&key) {
            return false; // belongs to another unplaced piece — can't rest on it yet
        }
        if !full_rows.contains(&y) {
            return false; // empty cell in non-full row — no support
        }
        y -= 1; // skip through full row, check below
    }
    true // reached floor (y < 0)
}

fn can_place(
    op: &PieceOperation,
    occupied: &HashSet<String>,
    full_rows: &HashSet<i32>,
    other_piece_cells: &HashSet<String>,
) -> bool {
    // 1. All cells within bounds
    if !in_bounds(op) { return false; }

    // 2. Must not overlap already-placed pieces
    let piece_type = op.piece_type.chars().next().unwrap_or('_');
    let cells = cell_positions(piece_type, &op.rotation, op.x, op.y);
    if overlaps(&cells, occupied) { return false; }

    // 3. Support: at least 1 external below-cell must be supported.
    //    Full rows are transparent. Other unplaced pieces' cells are NOT support.
    let cell_set: HashSet<String> = cells.iter()
        .map(|(x, y)| format!("{},{}", x, y))
        .collect();

    let mut external_below = 0u32;
    let mut external_supported = 0u32;

    for (cx, cy) in &cells {
        let below = *cy - 1;
        if below < 0 {
            external_below += 1;
            external_supported += 1; // floor
            continue;
        }
        let below_key = format!("{},{}", cx, below);
        if cell_set.contains(&below_key) {
            continue; // piece's own cell (vertical stack)
        }
        external_below += 1;
        if trace_support(*cx, below, occupied, full_rows, other_piece_cells) {
            external_supported += 1;
        }
    }

    // Must have at least 1 support, and no below cell can belong to another unplaced piece
    let has_dependency = (0..cells.len()).any(|i| {
        let (cx, cy) = (cells[i].0, cells[i].1);
        let below = cy - 1;
        if below < 0 { return false; }
        let below_key = format!("{},{}", cx, below);
        if cell_set.contains(&below_key) { return false; }
        other_piece_cells.contains(&below_key)
    });

    if has_dependency {
        return false; // rests on another unplaced piece — wait for it
    }

    if external_below > 0 && external_supported == 0 {
        return false;
    }

    true
}

// --- Parse field string ---

fn parse_field(field_str: &str) -> Option<Vec<Vec<char>>> {
    if field_str.len() != 230 { return None; }
    let chars: Vec<char> = field_str.chars().collect();
    let mut grid = vec![vec!['_'; 10]; 23];
    for row in 0..23 {
        for col in 0..10 {
            grid[22 - row][col] = chars[row * 10 + col];
        }
    }
    Some(grid)
}

fn cell_at(grid: &[Vec<char>], x: i32, y: i32) -> char {
    if !(0..=9).contains(&x) || !(0..=22).contains(&y) { return '_'; }
    grid[y as usize][x as usize]
}

// --- Flood fill ---

const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

fn flood_fill(
    grid: &[Vec<char>], start_x: i32, start_y: i32, piece_type: char,
    visited: &mut [Vec<bool>],
) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    let mut queue = vec![(start_x, start_y)];
    visited[start_y as usize][start_x as usize] = true;

    while let Some((x, y)) = queue.pop() {
        cells.push((x, y));
        for (dx, dy) in &DIRS {
            let nx = x + dx; let ny = y + dy;
            if !(0..=9).contains(&nx) || !(0..=22).contains(&ny) { continue; }
            if visited[ny as usize][nx as usize] { continue; }
            if cell_at(grid, nx, ny) != piece_type { continue; }
            visited[ny as usize][nx as usize] = true;
            queue.push((nx, ny));
        }
    }
    cells
}

// --- Identify piece (reverse lookup) ---

fn identify_piece(cells: &[(i32, i32)], piece_type: char) -> Option<PieceOperation> {
    let shape = spawn_shape(piece_type);
    if shape.is_empty() || cells.len() != shape.len() { return None; }

    let cell_set: HashSet<String> = cells.iter()
        .map(|(x, y)| format!("{},{}", x, y))
        .collect();

    for rot_name in ROTATIONS {
        let ref_positions = rotate(&shape, rot_name);
        if ref_positions.len() != cells.len() { continue; }

        for ref_pos in &ref_positions {
            for cell in cells {
                let ox = cell.0 - ref_pos.0;
                let oy = cell.1 - ref_pos.1;

                if ref_positions.iter().all(|(px, py)| {
                    cell_set.contains(&format!("{},{}", px + ox, py + oy))
                }) {
                    return Some(PieceOperation {
                        piece_type: piece_type.to_string(),
                        rotation: rot_name.to_string(),
                        x: ox, y: oy,
                    });
                }
            }
        }
    }
    None
}

// --- Topological sort ---

fn find_valid_order(
    ops: &[PieceOperation],
    initial_occupied: HashSet<String>,
    full_rows: &HashSet<i32>,
) -> Option<Vec<PieceOperation>> {
    // Build mapping: every cell → piece index (for dependency tracking)
    let mut all_piece_cells: Vec<HashSet<String>> = Vec::with_capacity(ops.len());
    for op in ops {
        let piece_type = op.piece_type.chars().next().unwrap_or('_');
        let cells = cell_positions(piece_type, &op.rotation, op.x, op.y);
        all_piece_cells.push(cells.iter().map(|(x, y)| format!("{},{}", x, y)).collect());
    }

    let mut remaining: Vec<usize> = (0..ops.len()).collect();
    let mut result: Vec<PieceOperation> = Vec::new();
    let mut occupied: HashSet<String> = initial_occupied;
    while !remaining.is_empty() {
        // Build set of cells belonging to OTHER unplaced pieces
        let other_piece_cells: HashSet<String> = remaining.iter()
            .flat_map(|&idx| all_piece_cells[idx].iter().cloned())
            .collect();

        let mut placed = false;
        for i in 0..remaining.len() {
            let idx = remaining[i];
            // Remove THIS piece's cells from "other" set (it can't depend on itself)
            let mut other = other_piece_cells.clone();
            for cell in &all_piece_cells[idx] {
                other.remove(cell);
            }

            if can_place(&ops[idx], &occupied, full_rows, &other) {
                let piece_type = ops[idx].piece_type.chars().next().unwrap_or('_');
                let cells = cell_positions(piece_type, &ops[idx].rotation, ops[idx].x, ops[idx].y);
                for (px, py) in &cells {
                    occupied.insert(format!("{},{}", px, py));
                }
                result.push(ops[idx].clone());
                remaining.remove(i);
                placed = true;
                break;
            }
        }
        if !placed { return None; }
    }
    Some(result)
}

// --- Public API ---

fn auto_split(field_str: &str) -> Result<Vec<PieceOperation>, String> {
    let grid = parse_field(field_str).ok_or("Invalid field: expected 230 characters")?;

    // Collect garbage (X) as initial support
    let mut initial_occupied: HashSet<String> = HashSet::new();
    for y in 0..=22 {
        for x in 0..10 {
            if grid[y as usize][x as usize] == 'X' {
                initial_occupied.insert(format!("{},{}", x, y));
            }
        }
    }

    // Detect full rows (10/10 cells non-empty, including X) — these were line clears
    let mut full_rows: HashSet<i32> = HashSet::new();
    for y in 0..=22 {
        let full = (0..10).all(|x| grid[y as usize][x as usize] != '_');
        if full {
            full_rows.insert(y);
        }
    }

    let mut visited = vec![vec![false; 10]; 23];
    let mut pieces: Vec<PieceOperation> = Vec::new();

    for y in (0..=22).rev() {
        for x in 0..10 {
            let cell = grid[y as usize][x as usize];
            if cell == '_' || cell == 'X' || visited[y as usize][x as usize] { continue; }

            let cells = flood_fill(&grid, x, y, cell, &mut visited);
            if cells.len() != 4 { continue; }

            if let Some(op) = identify_piece(&cells, cell) {
                pieces.push(op);
            }
        }
    }

    if pieces.is_empty() {
        return Err("No piece-colored cells found. Draw pieces using I/L/O/Z/T/J/S colors, not gray (X) blocks.".to_string());
    }

    find_valid_order(&pieces, initial_occupied, &full_rows)
        .ok_or_else(|| "Cannot place all pieces. Check: 1) no floating pieces (all must rest on floor/garbage/other pieces), 2) pieces are within 10-column bounds, 3) kick table supports the required rotations.".to_string())
}

#[tauri::command]
pub fn auto_split_field(field_str: String) -> Result<Vec<PieceOperation>, String> {
    auto_split(&field_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fumen_v115() {
        // Field string from v115@9gBtDewhi0wwBtCewhRpg0xwR4BewhRpglwwR4Cewh?ilJeAgl
        let mut field = vec!['_'; 230];
        let data = "ZZ____IJJJTZZ___IOOJTTSS__IOOLTSS___ILLL";
        let base_idx = 230 - 40; // last 4 rows
        for (i, c) in data.chars().enumerate() {
            field[base_idx + i] = c;
        }
        let field_str: String = field.into_iter().collect();

        let result = auto_split(&field_str);
        assert!(result.is_ok(), "auto_split failed: {:?}", result.err());
        let ops = result.unwrap();
        assert_eq!(ops.len(), 7, "expected 7 pieces, got {}: {:?}", ops.len(), ops);

        // Check each piece type is present
        let types: Vec<String> = ops.iter().map(|o| o.piece_type.clone()).collect();
        for t in &["I", "L", "O", "Z", "T", "J", "S"] {
            assert!(types.contains(&t.to_string()), "missing piece type {}", t);
        }

        // Check all cells are in bounds
        for op in &ops {
            assert!(in_bounds(op), "piece {:?} out of bounds", op);
        }

        // Verify O comes before J (O rests on L, J rests on O)
        let o_idx = ops.iter().position(|o| o.piece_type == "O").unwrap();
        let j_idx = ops.iter().position(|o| o.piece_type == "J").unwrap();
        assert!(o_idx < j_idx, "expected O before J, got O@{} J@{} order: {:?}", o_idx, j_idx,
            ops.iter().map(|o| format!("{}-{}", o.piece_type, o.rotation)).collect::<Vec<_>>());

        eprintln!("Placement order: {:?}",
            ops.iter().map(|o| format!("{}-{}@({},{})", o.piece_type, o.rotation, o.x, o.y)).collect::<Vec<_>>());
    }

    #[test]
    fn test_fumen_with_line_clears() {
        // v115@3gywHewwDeG8Q4I8R4H8AeQ4A8JeAgl
        // T-piece above, S-piece in cleared rows (y=1,2 are full)
        let field_str = "________________________________________________________________________________________________________________________________________________________________________________________TTT________T____XXXXXXXSXXXXXXXXXSSXXXXXXXX_SX".to_string();

        let result = auto_split(&field_str);
        assert!(result.is_ok(), "auto_split failed: {:?}", result.err());
        let ops = result.unwrap();
        assert_eq!(ops.len(), 2, "expected 2 pieces, got {}: {:?}", ops.len(), ops);
        assert!(ops.iter().any(|o| o.piece_type == "T"), "missing T piece");
        assert!(ops.iter().any(|o| o.piece_type == "S"), "missing S piece");

        eprintln!("Placement order: {:?}",
            ops.iter().map(|o| format!("{}-{}@({},{})", o.piece_type, o.rotation, o.x, o.y)).collect::<Vec<_>>());
    }
}
