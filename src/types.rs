use crossterm::style::{Color, ContentStyle, Stylize};
use std::fs::File;
use std::io::{Write, Result};
use std::path::Path;

use crate::plugins::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewId(pub u64);

#[derive(PartialEq, Debug, Clone)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct ScrollOffset {
    pub horizontal: usize,
    pub vertical: usize,
}

#[derive(Debug, Clone)]
pub struct Size {
    pub cols: u16,
    pub rows: u16
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub cols: u16,
    pub rows: u16
}

#[derive(Debug, PartialEq, Clone)]
pub enum EditorMode {
    Insert,
    Command,
    Normal
}

#[derive(PartialEq, Debug, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Tab,
    Esc,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    F(u8), // F1-F12
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool, // ctrl/cmd depending on OS
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            super_key: false
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum EditorAction {
    MoveCursor(Direction),
    InsertChar(char),
    DeleteChar,
    InsertNewline,
    StartCommandLine,
    ExecuteCommand,
    SwitchBuffer(BufferId),
    SaveCurrentBuffer,
    ChangeMode(EditorMode),
    QuitRequested,
    Undo,
    Redo
}

#[derive(PartialEq)]
pub enum EditorEvent {
    CursorMoved(Cursor),
    BufferOpened(BufferId),
    SaveRequested(BufferId),
    QuitRequested,
    CommandRequested(String),
    None
}

#[derive(PartialEq)]
pub struct Location {
    pub col: u16,
    pub row: u16
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub text: String,
    pub offset: usize,
    pub style: Option<Color>
}

pub struct SyntaxRegex {
    pub keywords: String,
    pub types: String,
    pub literals: String,
    pub comments: String,
    pub functions: String,
    pub attributes: String,
    pub punctuations: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Grid<T> {
    pub cells: Vec<Vec<T>>,
}

impl<T: Clone> Grid<T> {
    pub fn new(rows: usize, cols: usize, fill: T) -> Self {
        Self {
            cells: vec![vec![fill; cols]; rows],
        }
    }

    pub fn blit(&mut self, src: &Grid<T>, x: usize, y: usize) {
        for row in 0..src.rows() {
            let dest_row = y + row;
            if dest_row >= self.rows() { break; }

            for col in 0..src.cols() {
                let dest_col = x + col;
                if dest_col >= self.cols() { break; }

                self.cells[dest_row][dest_col] = src.cells[row][col].clone();
            }
        }
    }

    pub fn get(&self, row: usize) -> Option<&Vec<T>> {
        self.cells.get(row)
    }

    pub fn rows(&self) -> usize { self.cells.len() }
    pub fn cols(&self) -> usize { self.cells.first().map(|r| r.len()).unwrap_or(0) }
}


#[derive(Clone, PartialEq, Debug)]
pub struct RenderCell {
    pub ch: char,
    pub style: ContentStyle,
    pub transparent: bool
}

impl RenderCell {
    pub fn from_grapheme(g: &str, style: ContentStyle) -> Self {
        let ch = g.chars().next().unwrap_or(' ');
        Self { ch: ch, style, transparent: false }
    }

    pub fn default_style(config: &Config) -> ContentStyle {
        return ContentStyle::new()
            .on(config.current_theme().background())
            .with(config.current_theme().foreground())
    }

    pub fn blank() -> Self {
        Self {
            ch: ' ',
            style: ContentStyle::new(),
            transparent: true
        }
    }

    pub fn space(config: &Config) -> Self {
        Self {
            ch: ' ',
            style: Self::default_style(config),
            transparent: false
        }
    }

    pub fn tilde(config: &Config) -> Self {
        Self {
            ch: '~',
            style: Self::default_style(config),
            transparent: false
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct RenderLine {
    pub cells: Vec<RenderCell>
}

#[derive(Clone, PartialEq)]
pub struct RenderBuffer {
    pub drawn: Vec<RenderLine>,
    pub current: Vec<RenderLine>
}

impl RenderBuffer {
    pub fn dump_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::create(path)?;

        for (i, line) in self.current.iter().enumerate() {
            for cell in &line.cells {
                write!(file, "{}", cell.ch)?;
            }

            // newline between lines
            if i + 1 != self.current.len() {
                writeln!(file)?;
            }
        }

        Ok(())
    }
}
