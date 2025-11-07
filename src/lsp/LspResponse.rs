#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LspResponse<T> {
    pub jsonrpc: String,
    pub id: i32,
    pub result: T
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspSemanticResponseResult {
    pub resultId: String,
    pub data: Vec<i32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspDidOpenResponseResult {}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspResponseResult {
    pub capabilities: LspResponseCapabilities,
    pub serverInfo: LspResponseServerInfo
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspResponseCapabilities {
    pub positionEncoding: String,
    pub textDocumentSync: TextDocumentSync,
    pub selectionRangeProvider: bool,
    pub hoverProvider: bool,
    pub completionProvider: CompletionProvider,
    pub signatureHelpProvider: SignatureHelpProvider,
    pub definitionProvider: bool,
    pub typeDefinitionProvider: bool,
    pub implementationProvider: bool,
    pub referencesProvider: bool,
    pub documentHighlightProvider: bool,
    pub documentSymbolProvider: bool,
    pub workspaceSymbolProvider: bool,
    pub codeActionProvider: bool,
    pub codeLensProvider: CodeLensProvider,
    pub documentFormattingProvider: bool,
    pub documentRangeFormattingProvider: bool,
    pub documentOnTypeFormattingProvider: DocumentOnTypeFormattingProvider,
    pub renameProvider: RenameProvider,
    pub foldingRangeProvider: bool,
    pub declarationProvider: bool,
    pub workspace: LspWorkspace,
    pub callHierarchyProvider: bool,
    pub semanticTokensProvider: SemanticTokensProvider,
    pub inlayHintProvider: InlayHintProvider,
    pub diagnosticProvider: DiagnosticProvider,
    pub experimental: LspExperimental
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspExperimental {
    pub externalDocs: bool,
    pub hoverRange: bool,
    pub joinLines: bool,
    pub matchingBrace: bool,
    pub moveItem: bool,
    pub onEnter: bool,
    pub openCargoToml: bool,
    pub parentModule: bool,
    pub childModules: bool,
    pub runnables: LspExperimentalRunnables,
    pub ssr: bool,
    pub workspaceSymbolScopeKindFiltering: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspExperimentalRunnables {
    pub kinds: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticProvider {
    pub identifier: String,
    pub interFileDependencies: bool,
    pub workspaceDiagnostics: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlayHintProvider {
    pub resolveProvider: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticTokensProvider {
    pub legend: SemanticTokensLegend,
    pub range: bool,
    pub full: SemanticTokensFull
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticTokensLegend {
    pub tokenTypes: Vec<String>,
    pub tokenModifiers: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticTokensFull {
    pub delta: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspWorkspace {
    pub workspaceFolders: LspWorkspaceFolders,
    pub fileOperations: LspWorkspaceFileOperations
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspWorkspaceFileOperations {
    pub willRename: LspWorkspaceWillRename
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspWorkspaceWillRename {
    pub filters: Vec<LspWorkspaceRenameFilter>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspWorkspaceRenameFilter {
    pub scheme: String,
    pub pattern: LspWorkspaceRenamePattern 
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspWorkspaceRenamePattern {
    pub glob: String,
    pub matches: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspWorkspaceFolders {
    pub supported: bool,
    pub changeNotifications: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameProvider {
    pub prepareProvider: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentOnTypeFormattingProvider {
    pub firstTriggerCharacter: String,
    pub moreTriggerCharacter: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeLensProvider {
    pub resolveProvider: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureHelpProvider {
    pub triggerCharacters: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextDocumentSync {
    pub openClose: bool,
    pub change: i32,
    pub save: TextDocumentSyncSave
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextDocumentSyncSave {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionProvider {
    pub resolveProvider: bool,
    pub triggerCharacters: Vec<String>,
    pub completionItem: CompletionItem
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionItem {
    pub labelDetailsSupport: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LspResponseServerInfo {
    pub name: String,
    pub version: String
}
