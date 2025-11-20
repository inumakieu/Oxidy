use std::{io, time::Duration};

use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

use crate::{buffer::BufferLocation, types::EditorMode};

use crate::types::{Key, Modifiers, Direction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseType {
    Down(MouseButton, u16, u16),
    Up(MouseButton, u16, u16),
    Drag(MouseButton, u16, u16),
    Move(u16, u16),
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Key { key: Key, modifiers: Modifiers },
    Mouse(MouseType),
    Scroll(Direction),
}

pub trait InputHandler {
    fn poll(&mut self) -> io::Result<Option<InputEvent>>;
}

pub struct CrosstermInput;

impl InputHandler for CrosstermInput {
    fn poll(&mut self) -> io::Result<Option<InputEvent>> {
        if poll(Duration::from_millis(0))? {
            match read()? {
                Event::Key(e) => Ok(Some(self.translate_key_event(e))),
                Event::Mouse(e) => {
                    match e.kind {
                        MouseEventKind::ScrollDown => {
                            Ok(Some(InputEvent::Scroll(Direction::Down)))
                        }
                        MouseEventKind::ScrollUp => {
                            Ok(Some(InputEvent::Scroll(Direction::Up)))
                        }
                        _ => { Ok(None) }
                    }
                }
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

    fn translate_key_event(&mut self, event: KeyEvent) -> InputEvent {
        InputEvent::Key {
            key: match event.code {
                KeyCode::Char(c) => Key::Char(c),
                KeyCode::Enter => Key::Enter,
                KeyCode::Backspace => Key::Backspace,
                KeyCode::Tab => Key::Tab,
                KeyCode::Esc => Key::Esc,
                KeyCode::Left => Key::Left,
                KeyCode::Right => Key::Right,
                KeyCode::Up => Key::Up,
                KeyCode::Down => Key::Down,
                _ => Key::Unknown,
            },
            modifiers: Modifiers {
                ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
                alt: event.modifiers.contains(KeyModifiers::ALT),
                shift: event.modifiers.contains(KeyModifiers::SHIFT),
                super_key: false,
            },
        }
    }
}

pub struct WgpuInput;

impl InputHandler for WgpuInput {
    fn poll(&mut self) -> io::Result<Option<InputEvent>> {
        Ok(None)
    }
}

impl WgpuInput {
    pub fn new() -> Self {
        Self
    }
}
