use std::{io::{BufRead, BufReader, Read, Write}, process::{Child, ChildStdin, ChildStdout, Stdio}};

use crate::lsp::{LspMessage::LspMessage, LspResponse::LspResponse};

pub struct LspClient {
    pub process: Child,
    pub stdin: ChildStdin,
    pub stdout: ChildStdout
}

impl LspClient {
    pub fn spawn() -> Option<Self> {
        let output = std::process::Command::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();

        if let Ok(mut child) = output {
            let stdin = child.stdin.take().unwrap();
            let stdout = child.stdout.take().unwrap();

            return Some(
                LspClient {
                    process: child,
                    stdin,
                    stdout
                }
            )
        }

        None
    }

    pub fn send<T: serde::Serialize>(&mut self, json: LspMessage<T>) {
        // println!("Sending json.");
        let json_body = serde_json::to_string(&json).unwrap();
        
        let length = json_body.len();
        let header = format!("Content-Length: {}\r\n\r\n", length);
        let body_bytes = json_body.as_bytes();
        let header_bytes = header.as_bytes();
        let _ = self.stdin.write_all(header_bytes);
        let _ = self.stdin.write_all(body_bytes);
        let _ = self.stdin.flush();
    }

    pub fn read<T>(&mut self) -> Option<LspResponse<T>>
    where T: for<'de> serde::Deserialize<'de> {
        // println!("Process status: {:?}", self.process.try_wait());
        let mut reader = BufReader::new(&mut self.stdout);

        let mut buf = [0u8; 1];
        let mut value = "".to_string();
        let mut content_length: i32 = -1;
        loop {
            if content_length > 0 {
                let mut response_buf = vec![0; content_length as usize];

                match reader.read_exact(&mut response_buf) {
                    Ok(_) => {
                        // println!("Successfully read {} bytes: {:?}", content_length, response_buf);
                        let response: String = response_buf.into_iter()
                            .map(|val| char::from_u32(val as u32).unwrap_or('\u{FFFD}'))
                            .collect();
                        let resp_struct = serde_json::from_str(response.as_str());
                        match resp_struct {
                            Ok(resp_struct) => return Some(resp_struct),
                            Err(_) => return None
                        }
                    }
                    Err(e) => {
                        // Handle the error, e.g., if EOF is reached prematurely
                        // eprintln!("Error reading bytes: {}", e);
                    }
                }
                break;
            }
            let size = reader.read(&mut buf);
            
            match size {
                Ok(size) => {
                    if size == 0 {
                        // println!("EOF.");
                        break;
                    }

                    // print!("{}", buf[0] as char);
                    value.push(buf[0] as char);
                    if value == "Content-Length: " {
                        value = "".to_string();
                    }
                    if value.contains("\r\n\r\n") {
                        value = value.replace("\r\n\r\n", "");
                        content_length = value.parse::<i32>().unwrap();
                    }
                }
                Err(error) => {
                    // println!("{:?}", error);
                    break;
                }
            }
        }
        return None
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
