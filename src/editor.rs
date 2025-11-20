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
            EditorAction::MoveCursor(dir) => {
                if let Some(view) = self.views.get_mut(&self.active_view) {
                    match dir {
                        Direction::Up => self.move_cursor_up(),
                        Direction::Down => self.move_cursor_down(),
                        Direction::Left => self.move_cursor_left(),
                        Direction::Right => self.move_cursor_right(),
                    }
                }
            }
            EditorAction::InsertChar(ch) => {
                let view = self.views.get(&self.active_view).unwrap();
                if let Some(buffer) = self.buffers.get_mut(&view.buffer) {
                    if let Some(line) = buffer.lines.get_mut(view.cursor.row) {
                        // check if cursor is inside char (unicode)
                        let byte_idx = line.char_indices()
                            .nth(view.cursor.col)
                            .map(|(i, _)| i)
                            .unwrap_or_else(|| line.len());
                        line.insert(byte_idx, *ch);

                        view.highlighter.shift_line_tokens(view.cursor.row, view.cursor.col, 1);

                        self.move_cursor_right();
                    }
                }
            }
            EditorAction::DeleteChar => {
                let view = self.views.get_mut(&self.active_view).unwrap();
                if let Some(buffer) = self.buffers.get_mut(&view.buffer) {
                    let line_index = view.cursor.row;
                    let mut new_col = view.cursor.col;
                    let mut move_up = false;

                    if view.cursor.col == 0 {
                        if line_index > 0 {
                            // split the slice to borrow both lines safely
                            let (before, after) = buffer.lines.split_at_mut(line_index);
                            let prev = &mut before[line_index - 1];
                            let curr = &mut after[0];
                            new_col = prev.clone().len();
                            prev.push_str(curr);
                            buffer.lines.remove(line_index);
                            move_up = true;
                        }
                    } else if let Some(line) = buffer.lines.get_mut(line_index) {
                        if view.cursor.col <= line.len() {
                            let byte_idx = line.char_indices()
                                .nth(view.cursor.col - 1)
                                .map(|(i, _)| i)
                                .unwrap_or_else(|| line.len());
                            line.remove(byte_idx);
                            new_col -= 1;
                        }
                    }

                    view.highlighter.shift_line_tokens(view.cursor.row, view.cursor.col, -1);
                    
                    view.cursor.col = new_col;
                    if move_up { self.move_cursor_up(); }
                }
            }
            EditorAction::InsertNewline => {
                let view = self.views.get_mut(&self.active_view).unwrap();
                if let Some(buffer) = self.buffers.get_mut(&view.buffer) {
                    if view.cursor.row >= buffer.lines.len() {
                        return;
                    }

                    // Take ownership of the current line (no borrow remains)
                    let line = buffer.lines.remove(view.cursor.row);

                    if view.cursor.col < line.len() {
                        let (first, second) = line.split_at(view.cursor.col);

                        buffer.lines.insert(view.cursor.row, first.to_string());
                        buffer.lines.insert(view.cursor.row + 1, second.to_string());
                    } else {
                        // cursor at end → insert empty line
                        buffer.lines.insert(view.cursor.row, line);
                        buffer.lines.insert(view.cursor.row + 1, String::new());
                    }

                    view.cursor.row += 1;
                    view.cursor.col = 0;
                }
            }
            EditorAction::ChangeMode(mode) => {
                if let Some(view) = self.views.get_mut(&self.active_view) {
                    view.mode = mode.clone();
                }
            }
            EditorAction::SaveCurrentBuffer => {
                if let Some(view) = self.views.get_mut(&self.active_view) {
                    self.event_sender.send(EditorEvent::SaveRequested(view.buffer));
                }
            }
            EditorAction::QuitRequested => {self.event_sender.send(EditorEvent::QuitRequested);},
            _ => {}
        }
    }

    pub fn open_buffer(&mut self, path: String, content: String, size: Size) {
        let lines: Vec<String> = content
            .replace("\r\n", "\n")
            .replace("\r", "\n")
            .split("\n")
            .map(|s| s.to_string())
            .collect();

        let buffer_id = self.buffers.len();
        let buffer = Buffer::new(lines, path);
        
        self.buffers.insert(BufferId(buffer_id as u64), buffer);

        let view_id = ViewId(self.views.len() as u64);
        let view = BufferView::new(view_id.clone(), BufferId(buffer_id as u64), size.clone());
        
        self.views.insert(view_id.clone(), view.clone());

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
        
        eprintln!("NONE");
        None
    }

    pub fn views(&self) -> HashMap<ViewId, BufferView> {
        return self.views.clone()
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        return self.buffers.get(id);
    }

    fn move_cursor_up(&mut self) {
        if let Some(view) = self.views.get_mut(&self.active_view) {
            if view.cursor.row > 0 {
                view.cursor.row -= 1;
            }

            if view.scroll.vertical == 0 { return }

            if view.cursor.row < view.scroll.vertical {
                view.scroll.vertical -= 1
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if let Some(view) = self.views.get_mut(&self.active_view) {
            if view.cursor.row < self.buffers.get(&view.buffer).unwrap().lines.len() - 1 {
                view.cursor.row += 1;
            }

            if view.cursor.row >= view.size.rows as usize + view.scroll.vertical {
                view.scroll.vertical += 1;
            }
        }
    }

    fn move_cursor_left(&mut self) {
        if let Some(view) = self.views.get_mut(&self.active_view) {
            let line = self.buffers.get(&view.buffer).unwrap().line(view.cursor.row).unwrap();
            if view.cursor.col >= line.len() {
                view.cursor.col = line.len();
            }

            if view.cursor.col > 0 {
                view.cursor.col -= 1;
            }
        }
    }

    fn move_cursor_right(&mut self) {
        if let Some(view) = self.views.get_mut(&self.active_view) {
            if let Some(line) = self.buffers.get(&view.buffer).unwrap().line(view.cursor.row) {
                if view.cursor.col < line.len() {
                    view.cursor.col += 1;
                }
            }
        }
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
