mod wqv;
use tokio;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadResponse {
    blob: Vec<u8>,
}

fn download_sync(_port_path: String) -> Result<DownloadResponse, String> {
    let contents = std::fs::read_to_string("./image.dat").map_err(|e| e.to_string())?;

    // Parse the hex string into bytes
    let bytes: Result<Vec<u8>, _> = contents
        .split(',')
        .map(str::trim)
        .map(|hex| u8::from_str_radix(hex.trim_start_matches("0x"), 16))
        .collect();

    let img = bytes.map_err(|e| e.to_string())?;

    std::thread::sleep(std::time::Duration::from_secs(1));
    Ok(DownloadResponse { blob: img })
}

#[tauri::command]
async fn download_image_from_watch(port_path: String) -> Result<DownloadResponse, String> {
    tokio::task::spawn_blocking(move || download_sync(port_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn list_serial_ports() -> Result<Vec<String>, String> {
    wqv::list_serial_usb_ports()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            download_image_from_watch,
            list_serial_ports
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
