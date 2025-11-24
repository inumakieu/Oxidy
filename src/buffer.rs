use std::collections::HashMap;

use crate::types::{Size, EditorMode, BufferId, Cursor, ScrollOffset, ViewId};
use crate::highlighter::Highlighter;


#[derive(Debug, Clone)]
pub struct Selection {} // TODO: Support selections

#[derive(Debug, Clone)]
pub struct BufferView {
    pub id: ViewId,
    pub buffer: BufferId,
    pub cursor: Cursor,
    pub scroll: ScrollOffset,
    pub selection: Option<Selection>,
    pub size: Size,
    pub mode: EditorMode,
    pub highlighter: Highlighter
}

pub enum BufferLocation {
    Top,
    Bottom,
    StartLine,
    EndLine,
    PreviousWord,
    NextWord
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub path: String,
    pub version: u32,
}

impl Buffer {
    pub fn new(lines: Vec<String>, path: String) -> Self {
        Self {
            lines,
            path,
            version: 1
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_at_scroll(&self, scroll: &ScrollOffset, row: usize) -> Option<&str> {
        let absolute = row + scroll.vertical;
        self.lines.get(absolute).map(|s| s.as_str())
    }

    pub fn set(&mut self, lines: Vec<String>, path: String) {
        self.lines = lines;
        self.path = path;
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, row: usize) -> Option<&str> {
        self.lines.get(row).map(|s| s.as_str())
    }
}

impl BufferView {
    pub fn new(id: ViewId, buffer: BufferId, size: Size) -> Self {
        let highlighter = Highlighter::new(HashMap::new());

        Self {
            id,
            buffer,
            size,

            cursor: Cursor { row: 0, col: 0 },
            scroll: ScrollOffset { horizontal: 0, vertical: 0 },
            selection: None,
            mode: EditorMode::Normal,
            highlighter
        }
    }

    pub fn visible_top(&self) -> usize {
        self.scroll.vertical
    }

    pub fn visible_bottom(&self) -> usize {
        self.scroll.vertical + (self.size.rows as usize).saturating_sub(1)
    }
}
