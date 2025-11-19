#![allow(non_snake_case)]

use std::io::{self, Read};
use std::sync::mpsc::Sender;
use std::fs::File;
use std::time::Duration;
use std::collections::HashMap;

use crate::buffer::{Buffer, BufferView};
use crate::input::InputHandler;
use crate::types::{BufferId, ViewId, EditorAction, Direction};

use crate::plugins::plugin_manager::PluginManager;
use crate::renderer::Renderer;
use crate::services::lsp_service::{LspService, LspServiceEvent};
use crate::types::{EditorEvent, EditorMode, Size, Token};
use crate::highlighter::Highlighter;
use crate::ui::command::Command;
use crate::ui::status_bar::StatusBar;
use crate::ui::ui_manager::UiManager;
use crate::ui::card::Card;
use crate::log_manager::LogManager;
use crate::command::{self, CommandManager};

#[macro_export]
macro_rules! elog {
    ($editor:expr, $($arg:tt)*) => {{
        $editor.logs.push_persistent(format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! notify {
    ($editor:expr, $duration:expr, $($arg:tt)*) => {{
        $editor.logs.push_notification(format!($($arg)*), $duration);
    }};
}

pub struct Editor {
    buffers: HashMap<BufferId, Buffer>,
    views: HashMap<ViewId, BufferView>,
    active_view: ViewId,

    pub event_sender: Sender<EditorEvent>
}

impl Editor {
    pub fn new(event_sender: Sender<EditorEvent>) -> Self {
        Self {
            buffers: HashMap::new(),
            views: HashMap::new(),
            active_view: ViewId(0),
            event_sender
        }
    }

    pub fn handle_action(&mut self, action: &EditorAction) {
        match action {
            EditorAction::MoveCursor(dir) => {}
            EditorAction::QuitRequested => {self.event_sender.send(EditorEvent::QuitRequested);},
            _ => {}
        }
    }

    pub fn open_buffer(&mut self, path: String, content: String, size: Size) {
        let lines: Vec<String> = content
            .split("\n")
            .map(|s| s.to_string())
            .collect();

        let buffer_id = self.buffers.len();

        let buffer = Buffer::new(lines, path);
        
        self.buffers.insert(BufferId(buffer_id as u64), buffer);

        let view_id = self.views.len();

        let view = BufferView::new(BufferId(buffer_id as u64), size);

        self.views.insert(ViewId(view_id as u64), view);

        // self.editor.buffer.set(lines.clone(), path.clone());
        
        /*
        let file_type_index = path.to_string().rfind(".");
        if let Some(file_type_index) = file_type_index {
            let file_type = &path[file_type_index + 1..];

            self.highlighter.init(file_type.to_string());
        }
        */
    }

    pub fn update_tokens(&mut self, tokens: Vec<Vec<Token>>) {
        if let Some(view) = self.views.get(&self.active_view) {
            view.highlighter.update_tokens(tokens);
        }
    }

    pub fn active_view(&self) -> Option<&BufferView> {
        return self.views.get(&self.active_view)
    }

    pub fn active_buffer(&self) -> Option<&Buffer> {
        if let Some(view) = self.active_view() {
            return self.buffers.get(&view.buffer);
        }

        None
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        return self.buffers.get(id);
    }

    /*
    pub fn handle_command(&mut self, cmd: EditorCommand) -> io::Result<EditorEvent> {
        match cmd {
            EditorCommand::MoveUp => self.buffer.move_up(),
            EditorCommand::MoveDown => self.buffer.move_down(),
            EditorCommand::MoveLeft => self.buffer.move_left(),
            EditorCommand::MoveRight => self.buffer.move_right(),
            EditorCommand::ScrollDown => self.buffer.scroll_down(),
            EditorCommand::ScrollUp => self.buffer.scroll_up(),
            EditorCommand::JumpTo(loc) => self.buffer.jump_to(loc),
            EditorCommand::InsertChar(c) => self.buffer.insert_char(c),
            EditorCommand::InsertCommandChar(c) => {}
            EditorCommand::Tab => {},
            EditorCommand::Enter => self.buffer.insert_newline(),
            EditorCommand::ChangeMode(mode) => {}
            EditorCommand::Backspace => {
                match self.mode {
                    EditorMode::INSERT => self.buffer.delete_char(),
                    EditorMode::COMMAND => {}
                    _ => {}
                }
            }
            EditorCommand::LeaveMode => {}
            EditorCommand::RunCommand => {}
            EditorCommand::Save => {}
            EditorCommand::Quit => return Ok(EditorEvent::Exit),
            _ => {}
        }
        Ok(EditorEvent::None)
    }
    */
}
