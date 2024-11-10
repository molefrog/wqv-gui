use chrono::{DateTime, NaiveDate, Utc};
use serialport;
use std::vec;

// TODO:
// save image as hex
// simple CLI to print help and select an adapter

/**
 * Reverse engineered protocol for the Casio WQV-1 watch.
 * https://www.mgroeber.de/misc/wqvprot.htm
 */

// On the WQV-1, the checksum is always a 16-bit sum of all the preceding bytes,
// exclusing the BOF code itself. The checksum is computed before escaping the
// original contents for transfer.
fn checksum(data: &[u8]) -> u16 {
    let mut chsum: u16 = 0;
    for &b in data {
        chsum += b as u16;
    }
    chsum
}

// unescapes all 0x7D <xored> sequences
// The contents of the frame between BOF and EOF are escaped to avoid
// confusing data bytes with BOF/EOF symbols: any byte that is either
// BOF, EOF, or 7Dh will be replaced by the combination (7Dh), (byte XOR 20h).
fn unescp_data(data: &[u8]) -> Vec<u8> {
    let mut fin = Vec::with_capacity(data.len());
    let mut it = data.iter();

    while let Some(&b) = it.next() {
        match b {
            0x7D => {
                let unesc = *it
                    .next()
                    .expect("found escape char, but there is no more data!")
                    ^ 0x20;

                fin.push(unesc);
            }
            _ => fin.push(b),
        }
    }

    fin
}

type Frm = (u8, u8, Vec<u8>);

pub enum Addr {
    Auto,
    Broadcast,
    #[allow(dead_code)]
    Fixed(u8),
}

pub struct ProtocolState {
    pub watch_addr: u8,
    port: Box<dyn serialport::SerialPort>,
    rem_buf: Vec<u8>,
}

impl ProtocolState {
    pub fn new(port: Box<dyn serialport::SerialPort>) -> Self {
        Self {
            watch_addr: 0xff, // our address, will be set by the watch in handshake
            port,
            rem_buf: vec![], // buffer for incomplete frames
        }
    }

    pub fn read_data_transmission<F>(&mut self, mut on_chunk: F) -> Result<Vec<u8>, &'static str>
    where
        F: FnMut(&[u8]),
    {
        let mut all_data = Vec::new();

        // ...here, <get> and <ret> are cycled through a list of values to implement
        // some kind of simple packet numbering:
        let put_cmds = [0x32, 0x34, 0x36, 0x38, 0x3A, 0x3C, 0x3E, 0x30];
        let ack_cmds = [0x41, 0x61, 0x81, 0xA1, 0xC1, 0xE1, 0x01, 0x21];

        loop {
            let (_, cmd, data) = self.read_frame()?;

            if cmd == 0x31 {
                // End of transmission
                break;
            }

            if data.len() == 0 || data[0] != 0x05 {
                return Err("Invalid data chunk format, expected 0x05 as first byte");
            }

            // Call the callback with the new chunk
            on_chunk(&data[1..]);

            // Add frame data to buffer
            all_data.extend_from_slice(&data[1..]);

            let cmd_idx = put_cmds
                .iter()
                .position(|&x| x == cmd)
                .expect("Unknown CMD during transmission, are we sending files?");

            // Send next command in sequence
            self.send_frame(Addr::Auto, ack_cmds[cmd_idx % 8], &[])
                .unwrap();

            println!(
                "~> {:X} <~ {:X} read {} bytes",
                put_cmds[cmd_idx % 8],
                ack_cmds[cmd_idx % 8],
                &data[1..].len()
            );
        }

        Ok(all_data)
    }

    pub fn read_cmd_frame(&mut self, expected_cmd: u8) -> Result<Frm, &'static str> {
        loop {
            let (addr, cmd, data) = self.read_frame()?;

            if cmd == expected_cmd {
                return Ok((addr, cmd, data));
            }
        }
    }

    pub fn read_cmd_log(&mut self, expected_cmd: u8) -> Result<Frm, &'static str> {
        let result = self.read_cmd_frame(expected_cmd)?;
        println!(
            "<~ ADR={:X} CMD={:X} DATA=({}) {:X?}",
            result.0,
            result.1,
            result.2.len(),
            &result.2[..10.min(result.2.len())]
        );

        Ok(result)
    }

    pub fn read_frame(&mut self) -> Result<Frm, &'static str> {
        let mut buf = Vec::<u8>::with_capacity(1024);
        let mut frame = Vec::new();

        let mut found_c0 = false;

        'scan: loop {
            let result = if self.rem_buf.len() > 0 {
                buf.clear();
                let b_read = self.rem_buf.len();
                println!("Found {} bytes from the last read", b_read);
                buf.append(&mut self.rem_buf);

                Ok(b_read)
            } else {
                buf.resize(1024, 0); // fill with zeros
                self.port.read(&mut buf)
            };

            match result {
                Ok(bytes_read) => {
                    for (i, &byte) in buf[..bytes_read].iter().enumerate() {
                        if byte == 0xc0 {
                            // Found first C0, start collecting frame
                            found_c0 = true;
                        } else if found_c0 && byte == 0xc1 {
                            // put the rest of the data away for the next read
                            self.rem_buf.clear();
                            self.rem_buf.extend_from_slice(&buf[i + 1..bytes_read]);

                            // todo: might be some bytes left
                            break 'scan;
                        } else if found_c0 {
                            frame.push(byte);
                        }
                    }
                }

                Err(_) => {}
            }
        }

        let unesc_frame = unescp_data(&frame);

        if unesc_frame.len() < 4 {
            return Err("frame is too short, can't locate its header");
        }

        // address and command
        let adr = unesc_frame[0];
        let cmd = unesc_frame[1];

        // the last 2 bytes are the checksum
        let [chkh, chkl] = unesc_frame.last_chunk::<2>().unwrap();

        let chk_expected = u16::from_be_bytes([*chkh, *chkl]);
        let chk_actual = checksum(&unesc_frame[..unesc_frame.len() - 2]);

        if chk_expected != chk_actual {
            println!(
                "Checksum check failed, expected {:X}, got {:X}, frame {:X?}",
                chk_expected, chk_actual, frame
            );
            return Err("checksum check failed");
        }

        let data = unesc_frame[2..unesc_frame.len() - 2].to_vec();

        // device assigns us an address
        if cmd == 0x93 && data.len() >= 5 {
            self.watch_addr = data[4];
        }

        Ok((adr, cmd, data))
    }

    pub fn send_frame(&mut self, adr_p: Addr, cmd: u8, data: &[u8]) -> Result<(), &'static str> {
        let adr: u8 = match adr_p {
            Addr::Auto => self.watch_addr,
            Addr::Fixed(a) => a,
            Addr::Broadcast => 0xff,
        };

        let mut headndata = vec![adr, cmd];
        let mut escdata = Vec::<u8>::new();

        for b in data {
            match *b {
                0xc1 | 0xc0 | 0x7d => {
                    escdata.push(0x7d);
                    escdata.push(0x20 ^ *b);
                }
                _ => {
                    escdata.push(*b);
                }
            }
            headndata.push(*b);
        }

        let chk = checksum(headndata.as_slice());
        let mut a = vec![0xc0, adr, cmd];

        a.extend(escdata);
        a.push((chk >> 8) as u8);
        a.push((chk & 0xff) as u8);
        a.push(0xc1);

        self.port.write(&a).unwrap();

        println!("~> ADR={:X} CMD={:X} DAT={:X?}", adr, cmd, data);
        Ok(())
    }
}

pub struct ImageBlob {
    pub name: String,
    pub date: DateTime<Utc>,
    pub img: Vec<u8>,
}

/* Parses the image blob from the watch, as per docs:
    struct {
        char name[24]; // space padded
        unsigned char year_minus_2000, month, day;
        unsigned char hour, minute; // there was a mistake in the spec! mm <-> hh

        unsigned char pixel[120*120/2]; // one nibble per pixel
    };
*/
pub fn parse_image_blob(data: &[u8]) -> Result<ImageBlob, String> {
    if data.len() != 7229 {
        return Err(format!(
            "Image data must be exactly 7229 bytes, but {} bytes given",
            data.len()
        ));
    }

    let name =
        String::from_utf8(data[..24].to_vec()).expect("Invalid UTF-8 sequence in image name");

    let name_trimmed = String::from(name.trim());

    let date = NaiveDate::from_ymd_opt(2000 + (data[24] as i32), data[25] as u32, data[26] as u32)
        .unwrap()
        .and_hms_opt(data[27] as u32, data[28] as u32, 0)
        .unwrap()
        .and_utc();

    Ok(ImageBlob {
        name: name_trimmed,
        date,
        img: data[29..].to_vec(),
    })
}

/** public API */
pub fn list_serial_usb_ports() -> Result<Vec<String>, String> {
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;

    Ok(ports
        .iter()
        // todo: this is mac specific
        .filter(|x| x.port_name.contains("usbserial"))
        .map(|x| x.port_name.clone())
        .collect())
}
