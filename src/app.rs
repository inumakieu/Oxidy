use std::sync::mpsc::{Receiver, channel};
use std::fs::File;
use std::io::{self, Read};
use std::sync::Arc;
use std::collections::HashMap;

use std::thread;
use std::time::Duration;

use crate::types::{EditorAction, EditorEvent, EditorMode, Size, Direction};
use crate::editor::Editor;
use crate::command::{self, CommandManager};
use crate::highlighter::Highlighter;
use crate::plugins::plugin_manager::PluginManager;
use crate::services::lsp_service::{LspService, LspServiceEvent, LspState};
use crate::ui::ui_manager::UiManager;
use crate::ui::status_bar::StatusBar;
use crate::ui::command::Command;
use crate::renderer::Renderer;
use crate::input::{InputHandler};
use crate::plugins::config::Config;
use crate::keymap::Keymap;
use crate::log;
use crate::KeyRepeatState;

pub struct App {
    pub size: Size,
    pub editor: Editor,
    pub commands: CommandManager,
    pub keymap: Keymap,
    pub plugins: PluginManager,
    pub lsp: Option<LspService>,
    pub ui: UiManager,
    pub renderer: Box<dyn Renderer>,
    pub input: Box<dyn InputHandler>,
    pub config: Config,
    pub key_repeat: KeyRepeatState,

    pub event_receiver: Receiver<EditorEvent>,
}

impl App {
    pub fn new(size: Size, renderer: Box<dyn Renderer>, input: Box<dyn InputHandler>) -> Self {
        let commands = CommandManager::new();
        let mut plugins = PluginManager::new();
        let lsp = None; //LspService::new();
        let mut ui = UiManager::new();
        let status_bar = StatusBar::new();
        ui.add(status_bar);
        let command = Command::new();
        ui.add(command);

        let mut keymap = Keymap::new();

        keymap
            .normal()
                .map("i", EditorAction::ChangeMode(EditorMode::Insert))
                .map(":", EditorAction::ChangeMode(EditorMode::Command))
                .map("<Up>", EditorAction::MoveCursor(Direction::Up))
                .map("<Down>", EditorAction::MoveCursor(Direction::Down))
                .map("<Left>", EditorAction::MoveCursor(Direction::Left))
                .map("<Right>", EditorAction::MoveCursor(Direction::Right))
                .map("w", EditorAction::SaveCurrentBuffer)
                .map("q", EditorAction::QuitRequested);
        keymap.insert()
                .map("<Backspace>", EditorAction::DeleteChar)
                .map("<Enter>", EditorAction::InsertNewline)
                .map("<Up>", EditorAction::MoveCursor(Direction::Up))
                .map("<Down>", EditorAction::MoveCursor(Direction::Down))
                .map("<Left>", EditorAction::MoveCursor(Direction::Left))
                .map("<Right>", EditorAction::MoveCursor(Direction::Right))
                .map("<Esc>", EditorAction::ChangeMode(EditorMode::Normal));
        keymap.command()
                .map("<Left>", EditorAction::MoveCursor(Direction::Left))
                .map("<Right>", EditorAction::MoveCursor(Direction::Right))
                .map("<Backspace>", EditorAction::DeleteCommandChar)
                .map("<Enter>", EditorAction::ExecuteCommand)
                .map("<Esc>", EditorAction::ChangeMode(EditorMode::Normal));


        let config = Config::default();

        let key_repeat = KeyRepeatState {
            last_movement: None
        };

        let (event_sender, event_receiver) = channel();

        let editor = Editor::new(event_sender);

        plugins.load_config();
        plugins.start_watcher().unwrap();

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
            key_repeat,

            event_receiver
        }
    }

    pub fn run(&mut self) {
        self.register_commands();
        loop {
            if !self.step() { break }
        }
    }

    pub fn step(&mut self) -> bool {
        self.handle_input_event();
            
        self.poll_lsp_events();
        self.poll_plugin_events();

        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                EditorEvent::QuitRequested => { 
                    return false;
                }
                EditorEvent::SaveRequested(_) => {
                    if let Some(lsp) = self.lsp.as_mut() {
                        let buffer = self.editor.active_buffer().unwrap();
                        lsp.did_change(&buffer.path, buffer.version, &buffer.text());
                    }
                }
                EditorEvent::ShowCommand => {
                    let command = self.ui.get_mut::<Command>();

                    if let Some(command) = command {
                        command.shown = true;
                    }
                }
                EditorEvent::HideCommand => {
                    let command = self.ui.get_mut::<Command>();

                    if let Some(command) = command {
                        command.shown = false;
                    }
                }
                EditorEvent::CommandCursorMoved(dir) => {
                    let command = self.ui.get_mut::<Command>();

                    if let Some(command) = command {
                        let mut cursor = command.cursor as isize;
                        command.cursor = (cursor + dir).clamp(0, command.command.len() as isize) as usize;
                    }

                }
                EditorEvent::CommandCharInserted(ch) => {
                    let command = self.ui.get_mut::<Command>();

                    if let Some(command) = command {
                        command.command.insert(command.cursor, ch);
                        command.cursor += 1;
                    }
                }
                EditorEvent::CommandCharDeleted => {
                    let command = self.ui.get_mut::<Command>();

                    if let Some(command) = command {
                        if command.cursor > 0 && command.cursor <= command.command.len() {
                            command.command.remove(command.cursor - 1);
                            command.cursor -= 1;
                        }
                    }
                }
                EditorEvent::StartLsp(name) => {
                    self.lsp = LspService::new(name);
                    if let Some(lsp) = self.lsp.as_mut() {
                        let path = self.editor.active_buffer().unwrap().path.clone();

                        let root_index = path.rfind("/").unwrap();
                        let root_uri = &path[0..root_index];
                        lsp.initialize(&root_uri);
                    }
                }
                EditorEvent::RequestDeltaSemantics => {
                    if let Some(lsp) = self.lsp.as_mut() {
                        let buffer = self.editor.active_buffer().unwrap();
                        lsp.did_change(&buffer.path, buffer.version, &buffer.text());
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        lsp.request_semantic_tokens(&buffer);
                    }
                }
                EditorEvent::ExecuteCommand => {
                    let command = self.ui.get_mut::<Command>();

                    if let Some(command) = command {
                        let mut cmd: Vec<String> = command.command.clone()
                            .split(" ")
                            .map(|s| s.to_string())
                            .collect();
                        
                        let name = cmd.remove(0);
                        self.commands.execute(&name, cmd, &mut self.editor);
                        command.command = "".into();
                        command.cursor = 0;
                        command.shown = false;
                    }
                    self.editor.handle_action(&EditorAction::ChangeMode(EditorMode::Normal));
                }
                _ => {}
            }
        }

        self.renderer.begin_frame();
        self.renderer.draw_buffer(&self.editor, &self.ui, &self.config);
        self.renderer.end_frame();

        true
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
        self.config = self.plugins.config.clone();
    }

    fn poll_lsp_events(&mut self) {
        if let Some(lsp) = self.lsp.as_mut() {
            match lsp.poll() {
                LspServiceEvent::Initialized => {
                    let buffer = self.editor.active_buffer();
                    if let Some(buffer) = buffer {
                        lsp.open_file(&buffer.path, &buffer.text());
                    }
                }
                LspServiceEvent::OpenedFile | LspServiceEvent::ReceivedDelta => {
                    let buffer = self.editor.active_buffer();
                    if let Some(buffer) = buffer {
                        lsp.request_semantic_tokens(&buffer);
                    }
                }
                LspServiceEvent::ReceivedSemantics { semantics: _ } => {
                    let theme = self.config.current_theme();
                    let buffer = self.editor.active_buffer();
                    if let Some(buffer) = buffer {
                        let tokens = lsp.set_tokens(&buffer, theme);
                        self.editor.update_tokens(tokens);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn open_file(&mut self, path: String) {
        let content = std::fs::read_to_string(&path)
            .expect("Failed to open file");

        // TODO: Calculate size based on opened buffers
        let buffer_size = Size {
            cols: self.size.cols.clone(),
            rows: self.size.rows.clone() - self.ui.top_offset() as u16
        };

        self.editor.open_buffer(path.clone(), content, buffer_size);

        let status = self.ui.get_mut::<StatusBar>();

        if let Some(status) = status {
            status.file = path.to_string().clone();
        }
    }

    pub fn register_commands(&mut self) {
        self.commands.register(
            command::Command {
                name: "q".into(),
                description: "Quit Oxidy.".into(),
                execute: (|editor, args| {
                    editor.event_sender.send(EditorEvent::QuitRequested);

                    Ok(())
                })
            }
        );

        self.commands.register(
            command::Command {
                name: "lsp".into(),
                description: "Interface the LSP.".into(),
                execute: (|editor, args| {
                    if let Some(subcommand) = args.first() {
                        match subcommand.as_str() {
                            "start" => {
                                let lsp_name = args[1..].join(" ");
                                editor.event_sender.send(EditorEvent::StartLsp(lsp_name));
                            }
                            "end" => {}
                            _ => {}
                        }
                    }

                    Ok(())
                })
            }
        )
    }
}
