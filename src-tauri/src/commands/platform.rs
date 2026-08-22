#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}
