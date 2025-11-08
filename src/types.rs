use crossterm::style::{Color, ContentStyle, Stylize};
use std::fs::File;
use std::io::{Write, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Size {
    pub cols: u16,
    pub rows: u16
}

#[derive(PartialEq)]
pub enum EditorMode {
    INSERT,
    COMMAND,
    NORMAL
}

#[derive(PartialEq)]
pub enum EditorEvent {
    Exit,
    Save,
    ChangeMode(EditorMode),
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

#[derive(Clone, PartialEq)]
pub struct RenderCell {
    pub ch: String,
    pub style: ContentStyle
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

#[derive(Clone, PartialEq)]
pub enum CardType {
    INFO,
    WARNING,
    ERROR
}

#[derive(Clone, PartialEq)]
pub struct Card {
    pub descripiton: String,
    pub card_type: CardType
}

impl Card {
    pub fn get_lines(&self, max_width: usize) -> Vec<String> {
        self.descripiton.chars()
            .collect::<Vec<char>>()
            .chunks(max_width).into_iter()
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
    }
}

impl CardType {
    pub fn style(&self) -> ContentStyle {
        match self {
            Self::INFO => { return ContentStyle::new().on(Color::Reset).white() }
            Self::WARNING => { return ContentStyle::new().on(Color::Reset).yellow() }
            Self::ERROR => { return ContentStyle::new().on(Color::Reset).red() }
        }
    }
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
