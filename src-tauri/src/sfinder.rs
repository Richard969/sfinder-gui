use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
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

    // --output-base (only if user specified one)
    if let Some(ref output_base) = config.output_base {
        if !output_base.is_empty() {
            args.push("--output-base".to_string());
            args.push(output_base.clone());
        }
    }

    let cmd = config.command.clone();
    args.push(cmd);

    if !config.tetfu.is_empty() {
        args.push("--tetfu".to_string());
        args.push(config.tetfu.clone());
    }
    if let Some(page) = config.page {
        args.push("--page".to_string());
        args.push(page.to_string());
    }
    if let Some(cl) = config.clear_line {
        args.push("--clear-line".to_string());
        args.push(cl.to_string());
    }
    if let Some(ref patterns) = config.patterns {
        if !patterns.is_empty() {
            args.push("--patterns".to_string());
            args.push(patterns.clone());
        }
    }
    if let Some(ref hold) = config.hold {
        args.push("--hold".to_string());
        args.push(hold.clone());
    }
    if let Some(ref drop) = config.drop {
        args.push("--drop".to_string());
        args.push(drop.clone());
    }
    if let Some(ref kicks) = config.kicks {
        args.push("--kicks".to_string());
        args.push(kicks.clone());
    }
    if let Some(ref format) = config.format {
        if !format.is_empty() {
            args.push("--format".to_string());
            args.push(format.clone());
        }
    }
    if config.split.unwrap_or(false) {
        args.push("--split".to_string());
        args.push("yes".to_string());
    }
    if config.specified_only.unwrap_or(false) {
        args.push("--specified-only".to_string());
    }
    if config.reserved.unwrap_or(false) {
        args.push("--reserved".to_string());
    }
    if let Some(ref field_path) = config.field_path {
        if !field_path.is_empty() {
            args.push("--field-path".to_string());
            args.push(field_path.clone());
        }
    }
    if let Some(ref patterns_path) = config.patterns_path {
        if !patterns_path.is_empty() {
            args.push("--patterns-path".to_string());
            args.push(patterns_path.clone());
        }
    }
    if let Some(threads) = config.threads {
        args.push("--threads".to_string());
        args.push(threads.to_string());
    }
    if let Some(max_layer) = config.max_layer {
        args.push("--max-layer".to_string());
        args.push(max_layer.to_string());
    }
    if let Some(ref key) = config.key {
        args.push("--key".to_string());
        args.push(key.clone());
    }
    args
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

                return Ok(SfinderOutput {
                    stdout,
                    stderr,
                    exit_code: payload.code.unwrap_or(-1),
                    output_files,
                    command_line,
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
