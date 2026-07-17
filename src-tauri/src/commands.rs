use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri::WebviewWindowBuilder;
use tauri_plugin_shell::ShellExt;
use screenshots::Screen;
use crate::sfinder::{self, CommandState};

// --- Type definitions ---

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SfinderJarInfo {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SfinderCommandConfig {
    pub command: String,
    pub tetfu: Vec<String>,
    #[serde(rename = "jarPath")]
    pub jar_path: Option<String>,
    #[serde(rename = "javaPath")]
    pub java_path: Option<String>,
    #[serde(rename = "page")]
    pub page: Option<u32>,
    #[serde(rename = "clearLine")]
    pub clear_line: Option<u32>,
    pub patterns: Option<String>,
    pub hold: Option<String>,
    pub drop: Option<String>,
    pub kicks: Option<String>,
    pub format: Option<String>,
    pub split: Option<bool>,
    #[serde(rename = "specifiedOnly")]
    pub specified_only: Option<bool>,
    pub reserved: Option<bool>,
    #[serde(rename = "outputBase")]
    pub output_base: Option<String>,
    #[serde(rename = "fieldPath")]
    pub field_path: Option<String>,
    #[serde(rename = "patternsPath")]
    pub patterns_path: Option<String>,
    pub threads: Option<u32>,
    #[serde(rename = "maxLayer")]
    pub max_layer: Option<u32>,
    pub key: Option<String>,
    pub mode: Option<String>,
    #[serde(rename = "coverLogic")]
    pub cover_logic: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PathResultEntry {
    pub fumen: String,
    pub coverage: u32,
    pub used: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoverResultEntry {
    pub pattern: String,
    pub fumen: String,
    pub coverage: u32,
    pub used: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SfinderOutput {
    pub stdout: String,
    pub stderr: String,
    #[serde(rename = "exitCode")]
    pub exit_code: i32,
    #[serde(rename = "outputFiles")]
    pub output_files: Vec<String>,
    #[serde(rename = "commandLine")]
    pub command_line: String,
    #[serde(rename = "pathResults")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_results: Option<Vec<PathResultEntry>>,
    #[serde(rename = "pathTotalPatterns")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_total_patterns: Option<u32>,
    #[serde(rename = "strictMinimal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_minimal: Option<Vec<PathResultEntry>>,
    #[serde(rename = "coverResults")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_results: Option<Vec<CoverResultEntry>>,
    #[serde(rename = "coverTotalPatterns")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_total_patterns: Option<u32>,
}

// --- Commands ---

#[tauri::command]
pub async fn check_java(app: AppHandle, java_path: Option<String>) -> Result<JavaInfo, String> {
    let java_exe = java_path.as_deref().filter(|p| !p.is_empty()).unwrap_or("java");
    let shell = app.shell();
    let output = shell
        .command(java_exe)
        .args(["-version"])
        .output()
        .await
        .map_err(|e| format!("Failed to run java: {}", e))?;

    // Java outputs version to stderr
    let version_text = String::from_utf8_lossy(&output.stderr);
    let installed = output.status.success();

    if installed {
        // Parse version from output like: 'openjdk version "17.0.1" 2021-10-19'
        let version = version_text
            .lines()
            .next()
            .and_then(|line| {
                line.split('"').nth(1).map(|v| v.to_string())
            })
            .or_else(|| {
                // Try alternate format: 'java version "1.8.0_292"'
                version_text
                    .lines()
                    .next()
                    .and_then(|line| {
                        line.split('"').nth(1).map(|v| v.to_string())
                    })
            });

        Ok(JavaInfo {
            installed: true,
            version,
            path: Some(java_exe.to_string()),
        })
    } else {
        Ok(JavaInfo {
            installed: false,
            version: None,
            path: None,
        })
    }
}

#[tauri::command]
pub async fn check_sfinder_jar(path: String) -> Result<SfinderJarInfo, String> {
    let path = std::path::Path::new(&path);

    if !path.exists() {
        return Ok(SfinderJarInfo {
            found: false,
            path: None,
            version: None,
        });
    }

    // Try to read manifest from the JAR
    // For now, just verify the file exists
    Ok(SfinderJarInfo {
        found: true,
        path: Some(path.to_string_lossy().to_string()),
        version: None,
    })
}

#[tauri::command]
pub async fn run_sfinder_command(
    app: AppHandle,
    config: SfinderCommandConfig,
    state: State<'_, CommandState>,
) -> Result<SfinderOutput, String> {
    sfinder::execute_sfinder(&app, &state, &config).await
}

#[tauri::command]
pub async fn cancel_command(
    state: State<'_, CommandState>,
) -> Result<(), String> {
    let mut child = state.child.lock().map_err(|e| e.to_string())?;

    if let Some(c) = child.take() {
        c.kill().map_err(|e| format!("Failed to kill process: {}", e))?;
        Ok(())
    } else {
        Err("No command is currently running".to_string())
    }
}

#[tauri::command]
pub async fn get_bundled_jar(app: AppHandle) -> Result<Option<String>, String> {
    Ok(sfinder::get_bundled_jar_path(&app))
}

#[tauri::command]
pub async fn read_output_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file '{}': {}", path, e))
}

#[tauri::command]
pub async fn recognize_field_from_path(path: String) -> Result<String, String> {
    crate::recognition::recognize_field_from_file(&path)
}

#[tauri::command]
pub async fn recognize_field_from_bytes(bytes: Vec<u8>) -> Result<String, String> {
    crate::recognition::recognize_field_from_bytes(&bytes)
}

/// Capture a screen region and recognize the Tetris board in one shot.
/// Tries multiple screenshot tools in order, then runs recognition.
#[tauri::command]
pub async fn capture_and_recognize() -> Result<String, String> {
    crate::recognition::capture_all_monitors().and_then(|_| {
        // For the old system-tool approach, just use first monitor full-screen
        crate::recognition::crop_and_recognize(0, 0, u32::MAX, u32::MAX)
    })
}

/// Capture all monitors and open the overlay selection window.
/// Creates the overlay immediately; screenshots load via get_capture_data.
#[tauri::command]
pub async fn start_capture(app: tauri::AppHandle) -> Result<(), String> {
    // If overlay exists and is just hidden, show it
    if let Some(existing) = app.get_webview_window("capture-overlay") {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    // Minimize main window so it's not in the screenshot
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.minimize();
    }

    // List screens to calculate virtual desktop bounds (no pixel capture yet)
    let screens = Screen::all().map_err(|e| format!("Failed to list screens: {}", e))?;
    let (min_x, min_y, max_x, max_y) = screens.iter().fold(
        (i32::MAX, i32::MAX, i32::MIN, i32::MIN),
        |(mx, my, mx2, my2), s| {
            let info = s.display_info;
            (mx.min(info.x), my.min(info.y), mx2.max(info.x + info.width as i32), my2.max(info.y + info.height as i32))
        },
    );
    let (win_w, win_h) = if min_x <= max_x {
        ((max_x - min_x) as u32, (max_y - min_y) as u32)
    } else {
        (1920, 1080)
    };

    // Create overlay window spanning all monitors
    let _ = WebviewWindowBuilder::new(
        &app,
        "capture-overlay",
        tauri::WebviewUrl::App("overlay.html".into()),
    )
    .title("")
    .decorations(false)
    .position(min_x as f64, min_y as f64)
    .inner_size(win_w as f64, win_h as f64)
    .transparent(true)
    .skip_taskbar(true)
    .always_on_top(true)
    .build()
    .map_err(|e| format!("Failed to create overlay window: {}", e))?;

    // Give overlay focus so it receives mouse events immediately
    if let Some(window) = app.get_webview_window("capture-overlay") {
        let _ = window.set_focus();
    }

    Ok(())
}

/// Get the stored capture data (trigger capture if not yet done).
#[tauri::command]
pub async fn get_capture_data() -> Result<crate::recognition::CaptureData, String> {
    let data = crate::recognition::capture_all_monitors()?;
    Ok(data)
}

/// Crop and recognize a region from the captured screen data.
/// The overlay sends (x, y, w, h) in global screen coordinates.
/// Re-captures screens on every call for WYSIWYG (what you see is what you get).
#[tauri::command]
pub async fn crop_and_recognize(
    app: tauri::AppHandle,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Result<String, String> {
    eprintln!("[overlay] crop_and_recognize: x={}, y={}, w={}, h={}", x, y, w, h);
    // Hide overlay so it doesn't appear in the screenshot (frontend will close it after IPC response)
    if let Some(window) = app.get_webview_window("capture-overlay") {
        let _ = window.hide();
    }
    eprintln!("[overlay] overlay hidden");

    // Re-capture now so the screenshot matches what the user sees
    if let Err(e) = crate::recognition::capture_all_monitors() {
        eprintln!("[overlay] capture_all_monitors failed: {}", e);
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.unminimize();
            let _ = main.set_focus();
        }
        return Err(e);
    }
    eprintln!("[overlay] monitors captured");

    match crate::recognition::crop_and_recognize(x, y, w, h) {
        Ok(field) => {
            eprintln!("[overlay] recognition OK, {} rows, first line: {:?}", field.lines().count(), field.lines().next());
            eprintln!("[overlay] full field:\n{}", field);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.unminimize();
                let _ = main.set_focus();
                eprintln!("[overlay] main window restored");
            }
            // Use app.emit (global) not main.emit so JS listen() catches it
            eprintln!("[overlay] emitting screenshot-result via app.emit");
            let _ = app.emit("screenshot-result", &field);
            Ok(field)
        }
        Err(e) => {
            eprintln!("[overlay] recognition error: {}", e);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.unminimize();
                let _ = main.set_focus();
            }
            let _ = app.emit("screenshot-error", &e);
            Err(e)
        }
    }
}

/// Close the overlay window by label.
/// Emits a cancel event so the toolbar can reset its capturing state.
#[tauri::command]
pub async fn close_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
    if let Some(window) = app.get_webview_window("capture-overlay") {
        window.close().map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}
