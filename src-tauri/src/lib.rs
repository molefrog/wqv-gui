mod wqv;

use chrono::Timelike;
use serde::Serialize;
use std::time::Duration;
use tauri::ipc::Channel;
use tokio;
use wqv::Addr;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadResponse {
    blob: Vec<u8>,
}

/**
 * An event channel to get updates on the download progress (packets, chunks of data)
 */
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
enum DownloadEvent {
    PacketsReceived { packets: u32 },
    Chunk { offset: usize, chunk: Vec<u8> },
}

fn download_sync(
    port_path: String,
    on_event: Channel<DownloadEvent>,
) -> Result<DownloadResponse, String> {
    let mut pckt_rcv = 0;

    let port = serialport::new(&port_path, 115_200)
        .timeout(Duration::from_secs(20))
        .open()
        .expect("Failed to open port");

    let mut proto = wqv::ProtocolState::new(port);

    let now = chrono::Local::now().time();

    // First: watch sends greeting, we respond with time
    proto.read_cmd_log(0xb3).unwrap();
    proto
        .send_frame(
            wqv::Addr::Broadcast,
            0xa3,
            &[
                now.hour() as u8,
                now.minute() as u8,
                now.second() as u8,
                0x00, // 1/256th of a second, assume 0
            ],
        )
        .unwrap();

    pckt_rcv += 1;
    on_event
        .send(DownloadEvent::PacketsReceived { packets: pckt_rcv })
        .ok();

    // Second: watch mirrors our time and sends the address, we ack
    proto.read_cmd_log(0x93).unwrap();

    proto.send_frame(Addr::Auto, 0x63, &[]).unwrap();
    println!("[*] We got the address {:X}", proto.watch_addr);

    pckt_rcv += 1;
    on_event
        .send(DownloadEvent::PacketsReceived { packets: pckt_rcv })
        .ok();

    // Third: watch sends one command, we ack, handshake is done!
    proto.read_cmd_log(0x11).unwrap();
    proto.send_frame(Addr::Auto, 0x01, &[]).unwrap();

    pckt_rcv += 1;
    on_event
        .send(DownloadEvent::PacketsReceived { packets: pckt_rcv })
        .ok();

    // Fourth: watch wants to send an image with some seq 04h FAh 1Ch 3Dh, we ack
    proto.read_cmd_log(0x10).unwrap();
    proto.send_frame(Addr::Auto, 0x21, &[]).unwrap();

    pckt_rcv += 1;
    on_event
        .send(DownloadEvent::PacketsReceived { packets: pckt_rcv })
        .ok();

    // Fifth: watch sends 11h, we ack with 20h and some unknown number
    proto.read_cmd_log(0x11).unwrap();
    proto.send_frame(Addr::Auto, 0x20, &[0x06]).unwrap();

    pckt_rcv += 1;
    on_event
        .send(DownloadEvent::PacketsReceived { packets: pckt_rcv })
        .ok();

    let mut offset = 0;
    // Sixth: watch sends the image data, we read it
    let data = proto
        .read_data_transmission(|chunk| {
            on_event
                .send(DownloadEvent::Chunk {
                    offset: offset,
                    chunk: chunk.to_vec(),
                })
                .ok();
            offset += chunk.len();
        })
        .unwrap();
    println!("[X] Data transmission complete, got {} bytes", offset);

    // end!
    proto.send_frame(Addr::Auto, 0x42, &[0x06]).unwrap();
    proto.read_cmd_log(0x53).unwrap();
    proto.send_frame(Addr::Auto, 0x63, &[]).unwrap();

    pckt_rcv += 1;
    on_event
        .send(DownloadEvent::PacketsReceived { packets: pckt_rcv })
        .ok();

    Ok(DownloadResponse { blob: data })
}

/**
 * Mock download, reads from a file instead of the watch
 */
fn download_sync_mock(
    _port_path: String,
    on_event: Channel<DownloadEvent>,
) -> Result<DownloadResponse, String> {
    let contents = std::fs::read_to_string("./image.dat").map_err(|e| e.to_string())?;

    // Parse the hex string into bytes
    let bytes: Result<Vec<u8>, _> = contents
        .split(',')
        .map(str::trim)
        .map(|hex| u8::from_str_radix(hex.trim_start_matches("0x"), 16))
        .collect();

    let img = bytes.map_err(|e| e.to_string())?;

    // Send image data in chunks
    for (i, chunk) in img.chunks(128).enumerate() {
        let offset = i * 128;
        on_event
            .send(DownloadEvent::Chunk {
                offset,
                chunk: chunk.to_vec(),
            })
            .unwrap();

        on_event
            .send(DownloadEvent::PacketsReceived { packets: i as u32 })
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    Ok(DownloadResponse { blob: img })
}

#[tauri::command]
async fn download_image_from_watch(
    port_path: String,
    use_mock: bool,
    on_event: Channel<DownloadEvent>,
) -> Result<DownloadResponse, String> {
    tokio::task::spawn_blocking(move || {
        if use_mock {
            download_sync_mock(port_path, on_event)
        } else {
            download_sync(port_path, on_event)
        }
    })
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
