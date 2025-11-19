use std::sync::mpsc::{Receiver, channel};
use std::fs::File;
use std::io::{self, Read};
use std::sync::Arc;

use crate::types::{EditorAction, EditorEvent, EditorMode, Size, Direction};
use crate::editor::Editor;
use crate::command::CommandManager;
use crate::highlighter::Highlighter;
use crate::plugins::plugin_manager::PluginManager;
use crate::services::lsp_service::{LspService, LspServiceEvent};
use crate::ui::ui_manager::UiManager;
use crate::ui::status_bar::StatusBar;
use crate::renderer::Renderer;
use crate::input::{InputHandler};
use crate::plugins::config::Config;
use crate::keymap::Keymap;

pub struct App {
    size: Size,
    editor: Editor,
    commands: CommandManager,
    keymap: Keymap,
    plugins: PluginManager,
    lsp: LspService,
    ui: UiManager,
    renderer: Box<dyn Renderer>,
    input: Box<dyn InputHandler>,
    config: Config,

    event_receiver: Receiver<EditorEvent>,
}

impl App {
    pub fn new(size: Size, renderer: Box<dyn Renderer>, input: Box<dyn InputHandler>) -> Self {
        let commands = CommandManager::new();
        let plugins = PluginManager::new();
        let lsp = LspService::new().unwrap();
        let ui = UiManager::new();

        let mut keymap = Keymap::new();

        keymap
            .normal()
                .map("i", EditorAction::ChangeMode(EditorMode::Insert))
                .map(":", EditorAction::ChangeMode(EditorMode::Command))
                .map("k", EditorAction::MoveCursor(Direction::Up))
                .map("j", EditorAction::MoveCursor(Direction::Down))
                .map("q", EditorAction::QuitRequested);
        keymap.insert()
                .map("<Backspace>", EditorAction::DeleteChar)
                .map("<Enter>", EditorAction::InsertNewline)
                .map("<Esc>", EditorAction::ChangeMode(EditorMode::Normal));
        keymap.command()
                .map("<Enter>", EditorAction::ExecuteCommand)
                .map("<Esc>", EditorAction::ChangeMode(EditorMode::Normal));


        let config = Config::default();

        let (event_sender, event_receiver) = channel();

        let editor = Editor::new(event_sender);

        Self {
            size,
            editor,
            commands,
            keymap,
            plugins,
            lsp,
            ui,
            renderer,
            input,
            config,

            event_receiver
        }
    }

    pub fn run(&mut self) {
        loop {
            self.handle_input_event();
            
            self.poll_lsp_events();
            self.poll_plugin_events();

            while let Ok(event) = self.event_receiver.try_recv() {
                match event {
                    EditorEvent::QuitRequested => { 
                        println!("Quit Requested.");
                        return;
                    }
                    _ => {}
                }
            }

            self.renderer.begin_frame();
            self.renderer.draw_buffer(&self.editor, &self.ui, &self.config);
            self.renderer.end_frame();
        }
    }

    fn handle_input_event(&mut self) {
        let input = match self.input.poll() {
            Ok(Some(ev)) => ev,
            _ => return,
        };
        let mode = match self.editor.active_view() {
            Some(view) => &view.mode,
            None => &EditorMode::Normal
        };
        
        let action = match self.keymap.resolve(input, mode) {
            Some(a) => a,
            None => return,
        };
        self.editor.handle_action(&action);
    }

    fn poll_plugin_events(&mut self) {
        self.plugins.poll_reload();
    }

    fn poll_lsp_events(&mut self) {
        match self.lsp.poll() {
            LspServiceEvent::Initialized => {
                let buffer = self.editor.active_buffer();
                if let Some(buffer) = buffer {
                    self.lsp.open_file(&buffer.path, &buffer.text());
                }
            }
            LspServiceEvent::OpenedFile => {
                let buffer = self.editor.active_buffer();
                if let Some(buffer) = buffer {
                    self.lsp.request_semantic_tokens(&buffer);
                }
            }
            LspServiceEvent::ReceivedSemantics { semantics: _ } => {
                let theme = self.config.current_theme();
                let buffer = self.editor.active_buffer();
                if let Some(buffer) = buffer {
                    let tokens = self.lsp.set_tokens(&buffer, theme);
                    self.editor.update_tokens(tokens);
                }
            }
            _ => {}
        }

    }

    pub fn open_file(&mut self, path: String) {
        let content = std::fs::read_to_string(&path)
            .expect("Failed to open file");

        self.editor.open_buffer(path.clone(), content, self.size.clone()); 

        let status = self.ui.get_mut::<StatusBar>();

        if let Some(status) = status {
            status.file = path.to_string().clone();
        }
                
        let root_index = path.rfind("/").unwrap();
        let root_uri = &path[0..root_index];
        self.lsp.initialize(&root_uri);
    }
}
