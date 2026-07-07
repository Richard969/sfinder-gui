use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_shell::ShellExt;
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
