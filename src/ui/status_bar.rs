use std::any::Any;

use crossterm::style::{Color, StyledContent, Stylize};

use crate::{types::{RenderCell, RenderLine}, ui::ui_element::UiElement};
use crate::types::{Cursor, EditorMode};

pub struct StatusBar {
    pub name: String,
    pub file: String,
    pub pos: Cursor,
    pub mode: EditorMode,
    pub bg: Color,
    pub fg: Color,
    pub left_symbol: String,
    pub right_symbol: String
}

impl UiElement for StatusBar {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn render(&self, frame: &mut Vec<RenderLine>) {
        let mut items = vec![];
        let title = self.item(&self.name);
        let file_path = self.item(&self.file);

        let mode = match self.mode {
            EditorMode::Insert => " INS",
            EditorMode::Command => " CMD",
            _ => ""
        };

        let state = format!("{:02}:{:02}{}", self.pos.col + 1, self.pos.row + 1, mode);
        let state_item = self.item(&state);

        items.extend(title);
        items.push(self.spacer(1));
        items.extend(file_path);

        let gap = self.spacer(
            frame[0].cells.len() - (
                (self.left_symbol.len() * 3) +
                (self.right_symbol.len() * 3) + 
                self.name.len() + self.file.len() + state.len()
            ) + 5
        );
        items.push(gap);
        items.extend(state_item);

        let mut render_line = RenderLine { cells: Vec::new() };
        
        for item in items {
            for char in item.content().chars() {
                render_line.cells.push(
                    RenderCell { ch: char.to_string(), style: item.style().clone() }
                );
            }
        }

        frame[0] = render_line;
    }
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            name: "Oxidy".to_string(),
            file: "file.rs".to_string(),
            pos: Cursor { col: 0, row: 0 },
            mode: EditorMode::Normal,
            bg: Color::Rgb { r: 68, g: 68, b: 72 },
            fg: Color::Rgb { r: 201, g: 199, b: 205 },
            left_symbol: "".to_string(),
            right_symbol: "".to_string()
        }
    }

    fn item(&self, title: &str) -> Vec<StyledContent<String>> {
        let reset_color = Color::Rgb { r: 22, g: 22, b: 23 };

        let item = vec![
            self.left_symbol.clone().on(reset_color.clone()).with(self.bg.clone()),
            format!(" {} ", title).on(self.bg.clone()).with(self.fg.clone()),
            self.right_symbol.clone().on(reset_color.clone()).with(self.bg.clone()),
        ];

        item
    }

    fn spacer(&self, amount: usize) -> StyledContent<String> {
        let reset_color = Color::Rgb { r: 22, g: 22, b: 23 };
        format!("{}", " ".repeat(amount)).on(reset_color.clone())
    }
}
