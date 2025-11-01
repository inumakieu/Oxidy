use std::{io::{BufRead, BufReader, Read, Write}, process::{Child, ChildStdin, ChildStdout, Stdio}};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LspMessage<T> {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<InitializeClientCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeClientCapabilities {}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializedParams {}

#[derive(Debug, Serialize, Deserialize)]
pub struct DidOpenParams {
    pub textDocument: TextDocumentItem,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticTokenParams {
    pub textDocument: SemanticTokenTextDocumentItem,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticTokenTextDocumentItem {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextDocumentItem {
    pub uri: String,
    pub languageId: String,
    pub version: u64,
    pub text: String,
}

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

    pub fn read(&mut self) -> String {
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
                        return response;
                        // println!("{}", response);
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
        "".to_string()
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
