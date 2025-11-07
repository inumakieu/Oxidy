use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LspMessage<T> {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub method: String,
    pub params: T,
}
// {"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///home/inumaki/dev/oxidy/src/main.rs","diagnostics":[],"version":1}}
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
