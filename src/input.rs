use std::{io, time::Duration};

use crossterm::event::{poll, read, Event, KeyCode, KeyEvent};

use crate::types::EditorMode;

pub enum EditorCommand {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    InsertChar(char),
    InsertCommandChar(char),
    Backspace,
    Tab,
    Enter,
    Save,
    Quit,
    ChangeMode(EditorMode),
    LeaveMode,
    RunCommand,
    None,
}

pub trait InputHandler {
    fn poll(&mut self, mode: &EditorMode) -> io::Result<Option<EditorCommand>>;
}

pub struct CrosstermInput;

impl InputHandler for CrosstermInput {
    fn poll(&mut self, mode: &EditorMode) -> io::Result<Option<EditorCommand>> {
        if poll(Duration::from_millis(0))? {
            match read()? {
                Event::Key(e) => Ok(Some(self.translate_key_event(e, mode))),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
}

impl CrosstermInput {
    pub fn new() -> Self {
        Self
    }

    fn translate_key_event(&mut self, e: KeyEvent, mode: &EditorMode) -> EditorCommand {
        match e.code {
            KeyCode::Char(ch) => {
                match mode {
                    EditorMode::NORMAL => {
                        match ch {
                            'i' => return EditorCommand::ChangeMode(EditorMode::INSERT),
                            ':' => return EditorCommand::ChangeMode(EditorMode::COMMAND),
                            _ => {}
                        }
                    }
                    EditorMode::INSERT => return EditorCommand::InsertChar(ch),
                    EditorMode::COMMAND => return EditorCommand::InsertCommandChar(ch)
                }
            }
            KeyCode::Esc => return EditorCommand::LeaveMode,
            KeyCode::Enter => {
                if *mode == EditorMode::COMMAND {
                    return EditorCommand::RunCommand
                }
                return EditorCommand::Enter
            }
            KeyCode::Tab => return EditorCommand::Tab,
            KeyCode::Backspace => return EditorCommand::Backspace,
            KeyCode::Up => return EditorCommand::MoveUp,
            KeyCode::Down => return EditorCommand::MoveDown,
            KeyCode::Left => return EditorCommand::MoveLeft,
            KeyCode::Right => return EditorCommand::MoveRight,
            _ => {}
        }

        return EditorCommand::None
    }
}
