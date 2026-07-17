use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use crate::commands::{PathResultEntry, CoverResultEntry};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

use crate::commands::{SfinderCommandConfig, SfinderOutput};

pub struct CommandState {
    pub child: Mutex<Option<CommandChild>>,
    pub sfinder_jar_path: Mutex<String>,
    pub java_path: Mutex<String>,
}

impl CommandState {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            sfinder_jar_path: Mutex::new(String::new()),
            java_path: Mutex::new("java".to_string()),
        }
    }

    /// Get jar path (clone, drops guard before returning)
    pub fn get_jar_path(&self) -> String {
        self.sfinder_jar_path.lock().unwrap().clone()
    }

    /// Get java path (clone, drops guard before returning)
    pub fn get_java_path(&self) -> String {
        self.java_path.lock().unwrap().clone()
    }

    /// Store child for cancellation
    pub fn set_child(&self, child: CommandChild) {
        *self.child.lock().unwrap() = Some(child);
    }

    /// Remove child after completion
    pub fn take_child(&self) {
        *self.child.lock().unwrap() = None;
    }
}

/// Build the CLI argument vector from the command config
fn build_cli_args(config: &SfinderCommandConfig) -> Vec<String> {
    let mut args = Vec::new();
    let is_cover = config.command == "cover";

    // --output-base
    if let Some(ref output_base) = config.output_base {
        if !output_base.is_empty() {
            args.push("--output-base".to_string());
            args.push(output_base.clone());
        }
    }

    args.push(config.command.clone());

    // --tetfu
    for t in &config.tetfu {
        if !t.is_empty() {
            args.push("--tetfu".to_string());
            args.push(t.clone());
        }
    }

    // --page (not valid for cover)
    if !is_cover {
        if let Some(page) = config.page {
            args.push("--page".to_string());
            args.push(page.to_string());
        }
    }

    // --clear-line / --max-clearline (cover uses different flag)
    if let Some(cl) = config.clear_line {
        if is_cover {
            args.push("--max-clearline".to_string());
        } else {
            args.push("--clear-line".to_string());
        }
        args.push(cl.to_string());
    }

    // --patterns
    if let Some(ref patterns) = config.patterns {
        if !patterns.is_empty() {
            args.push("--patterns".to_string());
            args.push(patterns.clone());
        }
    }

    // --hold
    if let Some(ref hold) = config.hold {
        args.push("--hold".to_string());
        args.push(hold.clone());
    }

    // --drop
    if let Some(ref drop) = config.drop {
        args.push("--drop".to_string());
        args.push(drop.clone());
    }

    // --kicks
    if let Some(ref kicks) = config.kicks {
        args.push("--kicks".to_string());
        args.push(kicks.clone());
    }

    // --format (not valid for cover)
    if !is_cover {
        if let Some(ref format) = config.format {
            if !format.is_empty() {
                args.push("--format".to_string());
                args.push(format.clone());
            }
        }
    }

    // --split (not valid for cover)
    if !is_cover
        && config.split.unwrap_or(false) {
            args.push("--split".to_string());
            args.push("yes".to_string());
        }

    // --specified-only (not valid for cover)
    if !is_cover
        && config.specified_only.unwrap_or(false) {
            args.push("--specified-only".to_string());
        }

    // --reserved
    if config.reserved.unwrap_or(false) {
        args.push("--reserved".to_string());
    }

    // --field-path
    if let Some(ref field_path) = config.field_path {
        if !field_path.is_empty() {
            args.push("--field-path".to_string());
            args.push(field_path.clone());
        }
    }

    // --patterns-path
    if let Some(ref patterns_path) = config.patterns_path {
        if !patterns_path.is_empty() {
            args.push("--patterns-path".to_string());
            args.push(patterns_path.clone());
        }
    }

    // --threads
    if let Some(threads) = config.threads {
        args.push("--threads".to_string());
        args.push(threads.to_string());
    }

    // --max-layer
    if let Some(max_layer) = config.max_layer {
        args.push("--max-layer".to_string());
        args.push(max_layer.to_string());
    }

    // --key (not valid for cover)
    if !is_cover {
        if let Some(ref key) = config.key {
            args.push("--key".to_string());
            args.push(key.clone());
        }
    }

    // --mode (cover only: normal / tspin)
    if is_cover {
        if let Some(ref mode) = config.mode {
            if !mode.is_empty() {
                args.push("--mode".to_string());
                args.push(mode.clone());
            }
        }
    }

    args
}

/// Parse path CSV and compute coverage per fumen
/// CSV format: pattern,coverage,used,unused,fumen(semicolon-separated)
pub fn compute_path_coverage(csv_path: &str) -> (Vec<PathResultEntry>, u32) {
    let Ok(content) = std::fs::read_to_string(csv_path) else {
        return (vec![], 0);
    };

    // fumen → (pattern_count, used_pieces)
    let mut map: HashMap<String, (u32, String)> = HashMap::new();
    let mut total_rows: u32 = 0;

    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 5 { continue; }
        total_rows += 1;
        let used = cols[2].trim().to_string();
        let fumen_str = cols[4].trim();
        if fumen_str.is_empty() { continue; }

        for fumen in fumen_str.split(';') {
            let fumen = fumen.trim();
            if fumen.is_empty() { continue; }
            let entry = map.entry(fumen.to_string()).or_insert((0, used.clone()));
            entry.0 += 1;
            if !used.is_empty() {
                entry.1 = used.clone();
            }
        }
    }

    let mut results: Vec<PathResultEntry> = map.into_iter()
        .map(|(fumen, (coverage, used))| PathResultEntry { fumen, coverage, used })
        .collect();
    results.sort_by_key(|b| std::cmp::Reverse(b.coverage));
    (results, total_rows)
}

/// Find minimal set of fumens that cover all patterns (set cover approximation)
/// Returns the minimal fumen codes as PathResultEntry list with coverage=pattern_count
pub fn find_strict_minimal(results: &[PathResultEntry], csv_path: &str) -> Vec<PathResultEntry> {
    // Re-parse CSV to get pattern→fumens mapping
    let Ok(content) = std::fs::read_to_string(csv_path) else { return vec![] };

    // Group patterns by fumen for lookup
    let mut pattern_fumens: Vec<HashSet<String>> = vec![];
    let mut pattern_idx: HashMap<String, Vec<usize>> = HashMap::new(); // fumen → which patterns it covers

    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 5 { continue; }
        let fumen_str = cols[4].trim();
        if fumen_str.is_empty() { continue; }
        let fumens: HashSet<String> = fumen_str.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let idx = pattern_fumens.len();
        for f in &fumens {
            pattern_idx.entry(f.clone()).or_default().push(idx);
        }
        pattern_fumens.push(fumens);
    }

    if pattern_fumens.is_empty() { return vec![]; }

    // Greedy set cover: pick fumen that covers most uncovered patterns each iteration
    let mut uncovered: HashSet<usize> = (0..pattern_fumens.len()).collect();
    let mut selected: Vec<String> = vec![];
    let mut fumen_map: HashMap<String, PathResultEntry> = HashMap::new();

    // Build fumen map from original results
    for r in results {
        fumen_map.entry(r.fumen.clone()).or_insert_with(|| r.clone());
    }

    while !uncovered.is_empty() {
        let mut best_fumen = String::new();
        let mut best_count = 0;

        // Check each fumen's coverage over remaining uncovered patterns
        for r in results {
            if selected.contains(&r.fumen) { continue; }
            let mut count = 0;
            if let Some(indices) = pattern_idx.get(&r.fumen) {
                for &idx in indices {
                    if uncovered.contains(&idx) { count += 1; }
                }
            }
            if count > best_count {
                best_count = count;
                best_fumen = r.fumen.clone();
            }
        }

        if best_count == 0 { break; }

        selected.push(best_fumen.clone());
        // Remove covered patterns
        if let Some(indices) = pattern_idx.get(&best_fumen) {
            for &idx in indices {
                uncovered.remove(&idx);
            }
        }
    }

    selected.into_iter().filter_map(|f| fumen_map.remove(&f)).collect()
}

/// Parse cover CSV and return per-pattern results.
/// CSV format: pattern,coverage,used,unused,fumen(semicolon-separated)
pub fn parse_cover_csv(csv_path: &str) -> (Vec<CoverResultEntry>, u32) {
    let Ok(content) = std::fs::read_to_string(csv_path) else {
        return (vec![], 0);
    };
    let mut results = Vec::new();
    let mut total: u32 = 0;

    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 5 { continue; }
        total += 1;
        results.push(CoverResultEntry {
            pattern: cols[0].trim().to_string(),
            fumen: cols[4].trim().to_string(),
            coverage: cols[1].trim().parse().unwrap_or(0),
            used: cols[2].trim().to_string(),
        });
    }
    (results, total)
}

fn resolve_output_dir(config: &SfinderCommandConfig) -> String {
    config.output_base.as_ref()
        .filter(|p| !p.is_empty())
        .cloned()
        .unwrap_or_else(|| "output".to_string())
}

fn find_output_files(config: &SfinderCommandConfig) -> Vec<String> {
    let mut files = Vec::new();
    let output_dir = resolve_output_dir(config);

    let dir = Path::new(&output_dir);
    if !dir.exists() {
        return files;
    }

    let cmd_base = config.command.as_str();

    let patterns = [
        format!("{}.html", cmd_base),
        format!("{}_unique.html", cmd_base),
        format!("{}_minimal.html", cmd_base),
        format!("{}.txt", cmd_base),
    ];

    for pattern in &patterns {
        let file_path = dir.join(pattern);
        if file_path.exists() {
            files.push(file_path.to_string_lossy().to_string());
        }
    }

    files
}

pub fn get_bundled_jar_path(app: &AppHandle) -> Option<String> {
    // Check resource_dir/binaries/sfinder.jar
    if let Ok(dir) = app.path().resource_dir() {
        let path = dir.join("binaries").join("sfinder.jar");
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }
    // Fallback: check next to the executable (portable)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join("sfinder.jar");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    None
}

pub async fn execute_sfinder(
    app: &AppHandle,
    state: &CommandState,
    config: &SfinderCommandConfig,
) -> Result<SfinderOutput, String> {
    // Resolve JAR path: config → stored state → bundled
    let jar_path = {
        let from_config = config.jar_path.as_ref().filter(|p| !p.is_empty());
        let from_state = {
            let s = state.get_jar_path();
            if s.is_empty() { None } else { Some(s) }
        };
        from_config.or(from_state.as_ref()).or(get_bundled_jar_path(app).as_ref()).cloned()
    };
    let java_path = config.java_path.as_ref()
        .filter(|p| !p.is_empty())
        .cloned()
        .unwrap_or_else(|| state.get_java_path());
    let java_exe = if java_path.is_empty() { "java".to_string() } else { java_path };
    let Some(jar_path) = jar_path else {
        return Err("sfinder.jar not found. Place it in Settings or bundle with the app.".to_string());
    };

    let sfinder_args = build_cli_args(config);
    let mut full_args = vec!["-jar".to_string(), jar_path.clone()];
    full_args.extend(sfinder_args.clone());
    let command_line = format!("{} {}", java_exe, full_args.join(" "));

    // Spawn process
    let (mut rx, child) = app
        .shell()
        .command(&java_exe)
        .args(&full_args)
        .spawn()
        .map_err(|e| format!("Failed to spawn sfinder: {}", e))?;

    // Store child for cancellation (guard dropped immediately)
    state.set_child(child);

    let mut stdout = String::new();
    let mut stderr = String::new();

    // Process output stream
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(bytes) => {
                stdout.push_str(&String::from_utf8_lossy(&bytes));
            }
            CommandEvent::Stderr(bytes) => {
                stderr.push_str(&String::from_utf8_lossy(&bytes));
            }
            CommandEvent::Terminated(payload) => {
                state.take_child();
                let output_files = find_output_files(config);
                let (path_results, path_total) = if config.command == "path" {
                    compute_path_coverage("output/path.csv")
                } else {
                    (vec![], 0)
                };
                let strict_minimal = if config.command == "path" && !path_results.is_empty() {
                    let sm = find_strict_minimal(&path_results, "output/path.csv");
                    if sm.is_empty() { None } else { Some(sm) }
                } else { None };
                let path_results = if path_results.is_empty() { None } else { Some(path_results) };
                let path_total_patterns = if path_total > 0 { Some(path_total) } else { None };

                let (cover_results, cover_total) = if config.command == "cover" {
                    let csv_path = format!("{}/cover.csv", resolve_output_dir(config));
                    parse_cover_csv(&csv_path)
                } else {
                    (vec![], 0)
                };
                let cover_results = if cover_results.is_empty() { None } else { Some(cover_results) };
                let cover_total_patterns = if cover_total > 0 { Some(cover_total) } else { None };

                return Ok(SfinderOutput {
                    stdout,
                    stderr,
                    exit_code: payload.code.unwrap_or(-1),
                    output_files,
                    command_line,
                    path_results,
                    path_total_patterns,
                    strict_minimal,
                    cover_results,
                    cover_total_patterns,
                });
            }
            CommandEvent::Error(err) => {
                state.take_child();
                return Err(format!("Process error: {}", err));
            }
            _ => {}
        }
    }

    state.take_child();
    Err("Process ended without termination event".to_string())
}
