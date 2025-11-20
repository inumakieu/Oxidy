use std::collections::HashMap;
use std::{
    sync::mpsc::{self, Sender, Receiver},
    thread,
};
use std::process::Command;
use std::{io::{BufRead, BufReader, Read, Write}, process::{Child, Stdio}};
use std::fs::write;

use crossterm::style::Color;
use serde_json::Value;

use crate::buffer::Buffer;
use crate::lsp::LspResponse::LspDiagnostics;
use crate::{
    lsp::{
        LspMessage::{DidOpenParams, InitializeClientCapabilities, InitializeParams, InitializedParams, LspMessage, SemanticTokenParams, SemanticTokenTextDocumentItem, TextDocumentItem}, 
        LspResponse::{LspResponse, LspResponseResult, LspSemanticResponseResult}
    }, 
    types::Token
};
use crate::plugins::theme::Theme;

pub enum LspServiceEvent {
    Initialized,
    OpenedFile,
    ReceivedSemantics { semantics: LspSemanticResponseResult },
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspState {
    Uninitialized,
    Initializing,
    Initialized,
    OpeningFile,
    FileOpened,
    RequestingSemantics,
    SemanticsReceived,
}

pub struct LspService {
    sender: Sender<LspMessage<serde_json::Value>>,
    receiver: Receiver<LspResponse<serde_json::Value>>,
    process: Child,
    data: Option<LspResponseResult>,
    semantics: Option<LspSemanticResponseResult>,
    state: LspState,
}

impl LspService {
    pub fn new(name: String) -> Option<Self> {
        let mut process = Command::new(name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to start rust-analyzer");

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();

        let (tx_to_writer, rx_from_main): (Sender<LspMessage<serde_json::Value>>, Receiver<LspMessage<serde_json::Value>>) = mpsc::channel();
        let (tx_to_main, rx_from_reader): (Sender<LspResponse<serde_json::Value>>, Receiver<LspResponse<serde_json::Value>>) = mpsc::channel();

        
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
                    } else if let Ok(resp) = serde_json::from_str::<LspDiagnostics>(&text) {
                        // TODO: Show diagnostics
                    } else {
                        eprintln!("⚠️ Failed to parse LSP response: {}", text);
                    }
                }
            }
        });

        Some(
            Self {
                sender: tx_to_writer,
                receiver: rx_from_reader,
                process,
                data: None,
                semantics: None,
                state: LspState::Uninitialized
            }
        )
    }

    pub fn set_state(&mut self, state: LspState) {
        self.state = state;
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

    pub fn poll(&mut self) -> LspServiceEvent {
        // Try to read any incoming message
        if let Ok(resp_value) = self.receiver.try_recv() {
            match self.state {
                LspState::Initializing => {
                    if let Some(init_resp) = self.convert_response::<LspResponseResult>(resp_value) {
                        self.data = Some(init_resp.result);

                        let initialized = LspMessage {
                            jsonrpc: "2.0".into(),
                            id: None,
                            method: "initialized".into(),
                            params: InitializedParams {},
                        };
                        self.send(initialized);
                        self.state = LspState::Initialized;
                        return LspServiceEvent::Initialized;
                    }
                }

                LspState::RequestingSemantics => {
                    if let Some(resp) = self.convert_response::<LspSemanticResponseResult>(resp_value) {
                        self.semantics = Some(resp.result);
                        self.state = LspState::SemanticsReceived;
                        return LspServiceEvent::ReceivedSemantics {
                            semantics: self.semantics.clone().unwrap(),
                        };
                    }
                }

                _ => { /* ignore notifications, etc. */ }
            }
        }

        if self.state == LspState::OpeningFile {
            self.state = LspState::FileOpened;
            return LspServiceEvent::OpenedFile;
        }

        LspServiceEvent::None
    }


    fn convert_response<T>(&self, value: LspResponse<Value>) -> Option<LspResponse<T>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let result_typed: Result<T, _> = serde_json::from_value(value.result);
        match result_typed {
            Ok(result) => Some(LspResponse {
                jsonrpc: value.jsonrpc,
                id: value.id,
                result,
            }),
            Err(e) => {
                eprintln!("⚠️ Failed to parse LSP response payload: {}", e);
                None
            }
        }
    }

    pub fn initialize(&mut self, root_uri: &str) {
        if self.state != LspState::Uninitialized { return; }

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
        self.state = LspState::Initializing;
    }

    pub fn open_file(&mut self, uri: &str, contents: &str) {
        if self.state != LspState::Initialized { return; }

        let abs = std::fs::canonicalize(uri)
            .ok()
            .and_then(|p| Some(format!("file://{}", p.to_string_lossy())))
            .unwrap_or(uri.to_string());

        let open = LspMessage {
            jsonrpc: "2.0".into(),
            id: None,
            method: "textDocument/didOpen".into(),
            params: DidOpenParams {
                textDocument: TextDocumentItem {
                    uri: abs,
                    languageId: "rust".into(),
                    version: 1,
                    text: contents.to_string(),
                },
            },
        };

        self.send(open);
        self.state = LspState::OpeningFile;
    }

    pub fn request_semantic_tokens(&mut self, buffer: &Buffer) {
        if self.state != LspState::FileOpened { return; }

        let abs = std::fs::canonicalize(&buffer.path)
            .ok()
            .and_then(|p| Some(format!("file://{}", p.to_string_lossy())))
            .unwrap_or(buffer.path.clone());

        let syntax = LspMessage {
            jsonrpc: "2.0".into(),
            id: Some(4),
            method: "textDocument/semanticTokens/full".into(),
            params: SemanticTokenParams {
                textDocument: SemanticTokenTextDocumentItem { uri: abs },
            },
        };

        self.send(syntax);
        self.state = LspState::RequestingSemantics;
    }


    pub fn set_tokens(&self, buffer: &Buffer, theme: Theme) -> Vec<Vec<Token>> {
        let colors = theme.to_map();

        let mut current_data: [i32; 5];
        let mut index = 0;
        let mut previousDeltaStart = 0;
        let mut previousDeltaLine = 0;

        let mut tokens: Vec<Vec<Token>> = vec![Vec::new(); buffer.lines.len()];
        let mut currTokens: Vec<Token> = Vec::new();

        if let Some(semantics) = &self.semantics {
            while index + 4 < semantics.data.len() {
                current_data = semantics.data[index..index + 5].try_into().unwrap();
                
                let deltaLine = current_data[0];
                let deltaStart = current_data[1];
                let length = current_data[2];
                let tokenIndex = current_data[3];
                let tokenModifier = current_data[4];

                if deltaLine != 0 {
                    previousDeltaStart = 0;
                    tokens[previousDeltaLine as usize] = currTokens;
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
                    let mut final_key = token_type.clone();

                    let mut mods = vec![];
                    for bit in 0..data.capabilities.semanticTokensProvider.legend.tokenModifiers.len() {
                        if tokenModifier & (1 << bit) != 0 {
                            mods.push(data.capabilities.semanticTokensProvider.legend.tokenModifiers[bit].clone());
                        }
                    }

                    if !mods.is_empty() {
                        final_key = format!("{}.{:?}", token_type, mods.join("."));
                    }

                    let style = colors
                        .get(&final_key)
                        .or_else(|| colors.get(&token_type));
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
