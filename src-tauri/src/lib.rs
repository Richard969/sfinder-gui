mod color_split;
mod commands;
mod kick_table;
pub mod recognition;
mod sfinder;

use sfinder::CommandState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(CommandState::new())
        .invoke_handler(tauri::generate_handler![
            commands::check_java,
            commands::check_sfinder_jar,
            commands::run_sfinder_command,
            commands::cancel_command,
            commands::read_output_file,
            commands::get_bundled_jar,
            commands::recognize_field_from_path,
            commands::recognize_field_from_bytes,
            commands::capture_and_recognize,
            commands::start_capture,
            commands::get_capture_data,
            commands::crop_and_recognize,
            commands::close_overlay,
            color_split::auto_split_field,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
