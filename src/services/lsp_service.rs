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
        LspMessage::{DidOpenParams, InitializeClientCapabilities, TextDocumentClientCapabilities, TextDocumentSyncClientCapabilities, InitializeParams, InitializedParams, LspMessage, SemanticTokenParams, SemanticTokenTextDocumentItem, TextDocumentItem}, 
        LspResponse::{LspResponse, LspResponseResult, LspSemanticResponseResult, SemanticTokensFull}
    }, 
    types::Token
};
use crate::plugins::theme::Theme;
use crate::log;

pub enum LspServiceEvent {
    Initialized,
    OpenedFile,
    ReceivedDelta,
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
    RequestingDelta,
    DeltaReceived
}

pub struct LspService {
    sender: Sender<LspMessage<serde_json::Value>>,
    receiver: Receiver<LspResponse<serde_json::Value>>,
    process: Child,
    data: Option<LspResponseResult>,
    semantics: Option<LspSemanticResponseResult>,

    last_result_id: Option<String>,
    cached_semantic_data: Vec<i32>,
    server_supports_delta: bool,

    state: LspState,
}

impl LspService {
    pub fn new(name: String, args: Vec<String>) -> Option<Self> {
        if name.is_empty() { return None }

        let mut prcs = Command::new(name)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        if !prcs.is_ok() {
            return None
        }

        let mut process = prcs.unwrap();

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();

        let (tx_to_writer, rx_from_main): (Sender<LspMessage<serde_json::Value>>, Receiver<LspMessage<serde_json::Value>>) = mpsc::channel();
        let (tx_to_main, rx_from_reader): (Sender<LspResponse<serde_json::Value>>, Receiver<LspResponse<serde_json::Value>>) = mpsc::channel();

        let stderr = process.stderr.take().unwrap();

        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};

            let reader = BufReader::new(stderr);

            for line in reader.lines() {
                match line {
                    Ok(line) => log!("LSP STDERR: {}", line),
                    Err(e) => log!("error reading lsp stderr: {}", e),
                }
            }
        });
        
        thread::spawn(move || {
            let mut writer = stdin;
            while let Ok(msg) = rx_from_main.recv() {
                if let Ok(json) = serde_json::to_string(&msg) {
                    log!("{:?}", json);

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
                data: None,
                semantics: None,

                last_result_id: None,
                cached_semantic_data: vec![],
                server_supports_delta: false,

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
            log!("{:?}", resp_value);
            if resp_value.method.is_some() && resp_value.id.is_none() {
                let method = resp_value.method.unwrap().as_str();

                // You may want to handle standard ones like "$/progress", etc.
                // But for now, just ignore all of them.
                return LspServiceEvent::None;
            }

            match self.state {
                LspState::Initializing => {
                    if let Some(init_resp) = self.convert_response::<LspResponseResult>(resp_value) {
                        let caps = &init_resp.result.capabilities.semanticTokensProvider;

                        if let Some(provider) = &caps.full {
                            // The LSP may return:
                            // full: true
                            // or full: { delta: true }
                            match provider {
                                SemanticTokensFull::Options { delta } => self.server_supports_delta = delta.unwrap_or(false),
                                SemanticTokensFull::Boolean(_) => {}
                            }
                        }

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
                LspState::RequestingDelta => {
                    eprintln!("DELTA");
                    return LspServiceEvent::ReceivedDelta;
                }

                LspState::RequestingSemantics => {
                    if let Some(resp) = self.convert_response::<LspSemanticResponseResult>(resp_value) {
                        match &resp.result {
                            LspSemanticResponseResult::Full(full) => {
                                self.cached_semantic_data = full.data.clone();
                                self.last_result_id = full.resultId.clone();
                            }

                            LspSemanticResponseResult::Delta(delta) => {
                                for edit in &delta.edits {
                                    let start = edit.start as usize;
                                    let delete = edit.deleteCount as usize;

                                    self.cached_semantic_data
                                        .splice(start..start+delete, edit.data.clone());
                                }
                                self.last_result_id = delta.resultId.clone();
                            }
                        }
                        // now store semantics
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
                method: None,
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
                capabilities: Some(InitializeClientCapabilities {
                    text_document: Some(TextDocumentClientCapabilities {
                        synchronization: Some(TextDocumentSyncClientCapabilities {
                            did_open: true,
                            did_change: true,
                            did_close: true,
                        })
                    })
                }),
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

        //log!("{:?}", abs);

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
        if self.state != LspState::FileOpened && self.state != LspState::RequestingDelta { return; }

        let abs = std::fs::canonicalize(&buffer.path)
            .ok()
            .and_then(|p| Some(format!("file://{}", p.to_string_lossy())))
            .unwrap_or(buffer.path.clone());

        let msg = if false {//self.server_supports_delta && self.last_result_id.is_some() {
            // delta request
            LspMessage {
                jsonrpc: "2.0".into(),
                id: Some(4),
                method: "textDocument/semanticTokens/full/delta".into(),
                params: serde_json::json!({
                    "textDocument": { "uri": abs },
                    "previousResultId": self.last_result_id.clone().unwrap()
                }),
            }
        } else {
            // full request
            LspMessage {
                jsonrpc: "2.0".into(),
                id: Some(4),
                method: "textDocument/semanticTokens/full".into(),
                params: serde_json::json!({
                    "textDocument": { "uri": abs }
                }),
            }
        };

        self.send(msg);
        self.state = LspState::RequestingSemantics;
    }

    pub fn did_change(&mut self, uri: &str, version: u32, new_text: &str) {
        let abs = std::fs::canonicalize(uri)
            .ok()
            .and_then(|p| Some(format!("file://{}", p.to_string_lossy())))
            .unwrap_or(uri.to_string());

        //log!("{:?}", abs);

        let msg = LspMessage {
            jsonrpc: "2.0".into(),
            id: None,
            method: "textDocument/didChange".into(),
            params: serde_json::json!({
                "textDocument": {
                    "uri": abs,
                    "version": version,
                },
                "contentChanges": [
                    { "text": new_text }
                ]
            }),
        };

        self.send(msg);
        self.state = LspState::RequestingDelta;
    }

    pub fn set_tokens(&self, buffer: &Buffer, theme: Theme) -> Vec<Vec<Token>> {
        let colors = theme.to_map();

        let mut current_data: [i32; 5];
        let mut index = 0;
        let mut previousDeltaStart = 0;
        let mut previousDeltaLine = 0;

        let mut tokens: Vec<Vec<Token>> = vec![Vec::new(); buffer.lines.len()];
        let mut currTokens: Vec<Token> = Vec::new();
        let data = &self.cached_semantic_data;

        while index + 4 < data.len() {
            let current_data: [i32; 5] = data[index..index + 5].try_into().unwrap();
                
            let deltaLine = current_data[0];
            let deltaStart = current_data[1];
            let length = current_data[2];
            let tokenIndex = current_data[3];
            let tokenModifier = current_data[4];

            if deltaLine != 0 {
                previousDeltaStart = 0;
                if !currTokens.is_empty() {
                    tokens[previousDeltaLine as usize] = currTokens;
                }
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
                        row: lineIndex as usize,
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
