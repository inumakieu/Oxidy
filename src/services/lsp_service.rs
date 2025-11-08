use std::collections::HashMap;
use std::{
    sync::mpsc::{self, Sender, Receiver},
    thread,
};
use std::process::Command;
use std::{io::{BufRead, BufReader, Read, Write}, process::{Child, ChildStdin, ChildStdout, Stdio}};

use crossterm::style::Color;

use crate::buffer::Buffer;
use crate::{
    lsp::{
        LspClient::LspClient, 
        LspMessage::{DidOpenParams, InitializeClientCapabilities, InitializeParams, InitializedParams, LspMessage, SemanticTokenParams, SemanticTokenTextDocumentItem, TextDocumentItem}, 
        LspResponse::{LspDidOpenResponseResult, LspResponse, LspResponseResult, LspSemanticResponseResult}
    }, 
    types::Token
};

pub struct LspService {
    sender: Sender<LspMessage<serde_json::Value>>,
    receiver: Receiver<LspResponse<serde_json::Value>>,
    process: Child,
    data: Option<LspResponseResult>
}

impl LspService {
    pub fn new() -> Option<Self> {
        let mut process = Command::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to start rust-analyzer");

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();

        let (tx_to_writer, rx_from_main): (Sender<LspMessage<serde_json::Value>>, Receiver<LspMessage<serde_json::Value>>) = mpsc::channel();
        let (tx_to_main, rx_from_reader): (Sender<LspResponse<serde_json::Value>>, Receiver<LspResponse<serde_json::Value>>) = mpsc::channel();

        // 🧵 Writer thread – owns stdin
        thread::spawn(move || {
            let mut writer = stdin;
            while let Ok(msg) = rx_from_main.recv() {
                if let Ok(json) = serde_json::to_string(&msg) {
                    let header = format!("Content-Length: {}\r\n\r\n", json.len());
                    let _ = writer.write_all(header.as_bytes());
                    let _ = writer.write_all(json.as_bytes());
                    let _ = writer.flush();
                }
            }
        });

        // 🧵 Reader thread – owns stdout
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                // Read Content-Length header
                let mut header = String::new();
                if reader.read_line(&mut header).ok().is_none() {
                    break;
                }
                if !header.starts_with("Content-Length") {
                    continue;
                }
                let content_len = header
                    .split(':')
                    .nth(1)
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);

                // Skip the CRLF line
                let mut discard = String::new();
                let _ = reader.read_line(&mut discard);

                // Read body
                let mut buf = vec![0u8; content_len];
                if reader.read_exact(&mut buf).is_err() {
                    break;
                }

                if let Ok(text) = String::from_utf8(buf) {
                    if let Ok(resp) = serde_json::from_str::<LspResponse<serde_json::Value>>(&text) {
                        let _ = tx_to_main.send(resp);
                    } else {
                        // eprintln!("⚠️ Failed to parse LSP response: {}", text);
                    }
                }
            }
        });

        Some(
            Self {
                sender: tx_to_writer,
                receiver: rx_from_reader,
                process,
                data: None
            }
        )
    }

    pub fn send<T: serde::Serialize>(&self, msg: LspMessage<T>) {
        let params_json = serde_json::to_value(msg.params).unwrap();

        let msg_value = LspMessage::<serde_json::Value> {
            jsonrpc: msg.jsonrpc,
            id: msg.id,
            method: msg.method,
            params: params_json,
        };

        let _ = self.sender.send(msg_value);    
    }

    pub fn read<T>(&self) -> Option<LspResponse<T>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        match self.receiver.recv().ok() {
            Some(resp_generic) => {
                // Convert the inner generic Value into your concrete type
                let result_typed: Result<T, _> = serde_json::from_value(resp_generic.result);
                match result_typed {
                    Ok(result) => Some(LspResponse {
                        jsonrpc: resp_generic.jsonrpc,
                        id: resp_generic.id,
                        result,
                    }),
                    Err(e) => {
                        eprintln!("⚠️ Failed to parse LSP response payload: {}", e);
                        None
                    }
                }
            }
            None => None,
        }
    }

    pub fn initialize(&mut self, root_uri: &str) {
        let init = LspMessage {
            jsonrpc: "2.0".into(),
            id: Some(1),
            method: "initialize".into(),
            params: InitializeParams {
                capabilities: Some(InitializeClientCapabilities {}),
                root_uri: Some(root_uri.into()),
            },
        };

        self.send(init);
        let response_result: LspResponse<LspResponseResult> = self.read().unwrap();
        self.data = Some(response_result.result);

        let initialized = LspMessage {
            jsonrpc: "2.0".into(),
            id: None,
            method: "initialized".into(),
            params: InitializedParams {},
        };
        self.send(initialized);
    }

    pub fn open_file(&mut self, uri: &str, contents: &str) {
        let mut absolute_path: String = "".to_string();
        match std::fs::canonicalize(uri) {
            Ok(absolute) => absolute_path = absolute.to_string_lossy().to_string(),
            Err(e) => eprintln!("Error: {}", e),
        }
        let open = LspMessage {
            jsonrpc: "2.0".into(),
            id: None,
            method: "textDocument/didOpen".into(),
            params: DidOpenParams {
                textDocument: TextDocumentItem {
                    uri: format!("file://{}", absolute_path).into(),
                    languageId: "rust".into(),
                    version: 1,
                    text: contents.to_string(),
                },
            },
        };

        self.send(open);
        let _: Option<LspResponse<LspDidOpenResponseResult>> = self.read();
    }
    
    pub fn request_semantic_tokens(&mut self, buffer: &Buffer) -> Vec<Vec<Token>> {
        let mut absolute_path: String = "".to_string();
        match std::fs::canonicalize(buffer.path.clone()) {
            Ok(absolute) => absolute_path = absolute.to_string_lossy().to_string(),
            Err(e) => eprintln!("Error: {}", e),
        }

        let syntax = LspMessage {
            jsonrpc: "2.0".into(),
            id: Some(4),
            method: "textDocument/semanticTokens/full".into(),
            params: SemanticTokenParams {
                textDocument: SemanticTokenTextDocumentItem {
                    uri: format!("file://{}", absolute_path).into(),
                },
            }
        };

        self.send(syntax);
        let response: LspResponse<LspSemanticResponseResult> = self.read().unwrap();

        let mut current_data: [i32; 5];
        let mut index = 0;
        let mut previousDeltaStart = 0;
        let mut previousDeltaLine = 0;

        let mut tokens: Vec<Vec<Token>> = vec![Vec::new(); buffer.lines.len()];
        let mut currTokens: Vec<Token> = Vec::new();

        let mut colors: HashMap<String, Color> = HashMap::new();
        colors.insert("namespace".into(), Color::Cyan);
        colors.insert("type".into(), Color::Rgb { r: 201, g: 195, b: 220 });
        colors.insert("class".into(), Color::Rgb { r: 201, g: 195, b: 220 });
        colors.insert("enum".into(), Color::Rgb { r: 201, g: 195, b: 220 });
        colors.insert("interface".into(), Color::Rgb { r: 201, g: 195, b: 220 });
        colors.insert("struct".into(), Color::Rgb { r: 201, g: 195, b: 220 });
        colors.insert("typeParameter".into(), Color::Cyan);
        colors.insert("parameter".into(), Color::Rgb { r: 201, g: 195, b: 220 });
        colors.insert("variable".into(), Color::Rgb { r: 230, g: 225, b: 233 });
        colors.insert("property".into(), Color::Grey);
        colors.insert("enumMember".into(), Color::Yellow);
        colors.insert("event".into(), Color::Green);
        colors.insert("function".into(), Color::Green);
        colors.insert("method".into(), Color::Green);
        colors.insert("macro".into(), Color::Rgb { r: 202, g: 190, b: 255 });
        colors.insert("keyword".into(), Color::Rgb { r: 202, g: 190, b: 255 });
        colors.insert("modifier".into(), Color::Rgb { r: 202, g: 190, b: 255 });
        colors.insert("comment".into(), Color::DarkGrey);
        colors.insert("string".into(), Color::Yellow);
        colors.insert("number".into(), Color::Magenta);
        colors.insert("regexp".into(), Color::Magenta);
        colors.insert("operator".into(), Color::DarkMagenta);

        while index + 4 < response.result.data.len() {
            current_data = response.result.data[index..index + 5].try_into().unwrap();
            
            let deltaLine = current_data[0];
            let deltaStart = current_data[1];
            let length = current_data[2];
            let tokenIndex = current_data[3];
            let tokenModifier = current_data[4];

            if deltaLine != 0 {
                previousDeltaStart = 0;
                tokens.insert(previousDeltaLine as usize, currTokens);
                currTokens = Vec::new();
            }

            let lineIndex = previousDeltaLine + deltaLine;
            let charStartIndex = previousDeltaStart + deltaStart;
            let line = buffer.lines[lineIndex as usize].clone();
            let start_byte = utf16_to_byte_index(line.as_str(), charStartIndex as usize);
            let end_byte = utf16_to_byte_index(line.as_str(), (charStartIndex + length) as usize);
            let token_slice = &line[start_byte..end_byte]; 
            
            if let Some(data) = &self.data {
                let token_type = data.capabilities.semanticTokensProvider.legend.tokenTypes[tokenIndex as usize].clone();
                
                let style = colors.get(&token_type);
                currTokens.push(
                    Token {
                        text: token_slice.to_string(),
                        style: style.copied(),
                        offset: charStartIndex as usize
                    }
                );
            }
            previousDeltaStart = charStartIndex;
            previousDeltaLine = lineIndex;
            index += 5;
        }

        return tokens
    }
}

fn utf16_to_byte_index(s: &str, utf16_index: usize) -> usize {
    let mut count = 0;
    for (byte_idx, ch) in s.char_indices() {
        if count == utf16_index {
            return byte_idx;
        }
        count += ch.len_utf16();
    }
    s.len()
}

impl Drop for LspService {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
