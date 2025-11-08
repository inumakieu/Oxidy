use crossterm::style::{Color, StyledContent, Stylize};

use crate::{types::{RenderCell, RenderLine}, ui::ui_element::UiElement};

pub struct StatusBar {
    pub name: String,
    pub file: String,
    pub bg: Color,
    pub fg: Color,
    pub left_symbol: String,
    pub right_symbol: String
}

impl UiElement for StatusBar {
    fn render(&self, frame: &mut Vec<RenderLine>) {
        let mut items = vec![];
        let title = self.item(&self.name);
        let file_path = self.item(&self.file);
        let state = format!("{:02}:{:02}{}", 1, 1, " INS");
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
            bg: Color::Rgb { r: 32, g: 31, b: 37 },
            fg: Color::Rgb { r: 230, g: 225, b: 233 },
            left_symbol: "".to_string(),
            right_symbol: "".to_string()
        }
    }

    fn item(&self, title: &str) -> Vec<StyledContent<String>> { 
        let item = vec![
            self.left_symbol.clone().on(Color::Reset).with(self.bg.clone()),
            format!(" {} ", title).on(self.bg.clone()).with(self.fg.clone()),
            self.right_symbol.clone().on(Color::Reset).with(self.bg.clone()),
        ];

        item
    }

    fn spacer(&self, amount: usize) -> StyledContent<String> {
        format!("{}", " ".repeat(amount)).reset()
    }
}
