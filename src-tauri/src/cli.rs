use chrono::Timelike;
use serialport;
use std::time::Duration;

mod wqv;
use wqv::{list_serial_usb_ports, parse_image_blob, Addr, ProtocolState};

fn main() {
    let ports = list_serial_usb_ports().unwrap();
    let usbir_port = ports.first().expect("No USB serial port present!");

    println!("Found USB IR {}", usbir_port);

    let port = serialport::new(&*usbir_port, 115_200)
        .timeout(Duration::from_secs(60))
        .open()
        .expect("Failed to open port");

    let mut proto = ProtocolState::new(port);

    let now = chrono::Local::now().time();

    // First: watch sends greeting, we respond with time
    proto.read_cmd_log(0xb3).unwrap();
    proto
        .send_frame(
            Addr::Broadcast,
            0xa3,
            &[
                now.hour() as u8,
                now.minute() as u8,
                now.second() as u8,
                0x00, // 1/256th of a second, assume 0
            ],
        )
        .unwrap();

    // Second: watch mirrors our time and sends the address, we ack
    proto.read_cmd_log(0x93).unwrap();
    proto.send_frame(Addr::Auto, 0x63, &[]).unwrap();
    println!("[*] We got the address {:X}", proto.watch_addr);

    // Third: watch sends one command, we ack, handshake is done!
    proto.read_cmd_log(0x11).unwrap();
    proto.send_frame(Addr::Auto, 0x01, &[]).unwrap();

    // Fourth: watch wants to send an image with some seq 04h FAh 1Ch 3Dh, we ack
    proto.read_cmd_log(0x10).unwrap();
    proto.send_frame(Addr::Auto, 0x21, &[]).unwrap();

    // Fifth: watch sends 11h, we ack with 20h and some unknown number
    proto.read_cmd_log(0x11).unwrap();
    proto.send_frame(Addr::Auto, 0x20, &[0x06]).unwrap();

    // Sixth: watch sends the image data, we read it
    let data = proto.read_data_transmission().unwrap();
    println!("[X] Data transmission complete, got {} bytes", data.len());

    let blob = parse_image_blob(&data).unwrap();
    println!("name = {}", blob.name);
    println!("date = {}", blob.date);
    println!("{:02X?}", data);
    assert_eq!(
        blob.img.len(),
        7200,
        "Image data length must be exactly 7200 bytes"
    );

    // end!
    proto.send_frame(Addr::Auto, 0x42, &[0x06]).unwrap();
    proto.read_cmd_log(0x53).unwrap();
    proto.send_frame(Addr::Auto, 0x63, &[]).unwrap();
}
