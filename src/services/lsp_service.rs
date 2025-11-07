use std::collections::HashMap;

use crossterm::style::Color;

use crate::{
    lsp::{
        LspClient::LspClient, 
        LspMessage::{DidOpenParams, InitializeClientCapabilities, InitializeParams, InitializedParams, LspMessage, SemanticTokenParams, SemanticTokenTextDocumentItem, TextDocumentItem}, 
        LspResponse::{LspDidOpenResponseResult, LspResponse, LspResponseResult, LspSemanticResponseResult}
    }, 
    types::Token
};

pub struct LspService {
    client: LspClient,
    data: Option<LspResponseResult>
}

impl LspService {
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

        self.client.send(init);
        let response_result: LspResponse<LspResponseResult> = self.client.read().unwrap();
        self.data = Some(response_result.result);

        let initialized = LspMessage {
            jsonrpc: "2.0".into(),
            id: None,
            method: "initialized".into(),
            params: InitializedParams {},
        };
        self.client.send(initialized);
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

        self.client.send(open);
        let _: Option<LspResponse<LspDidOpenResponseResult>> = self.client.read();
    }
    
    pub fn request_semantic_tokens(&mut self, uri: &str, lines: Vec<String>) -> Vec<Vec<Token>> {
        let mut absolute_path: String = "".to_string();
        match std::fs::canonicalize(uri) {
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

        self.client.send(syntax);
        let response: LspResponse<LspSemanticResponseResult> = self.client.read().unwrap();

        let mut current_data: [i32; 5];
        let mut index = 0;
        let mut previousDeltaStart = 0;
        let mut previousDeltaLine = 0;

        let mut tokens: Vec<Vec<Token>> = vec![Vec::new(); lines.len()];
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
            let line = &lines[lineIndex as usize];
            let start_byte = utf16_to_byte_index(line, charStartIndex as usize);
            let end_byte = utf16_to_byte_index(line, (charStartIndex + length) as usize);
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
