use std::any::Any;

use crossterm::style::{ContentStyle, Stylize, Color};

use crate::{types::{RenderCell, Grid}, ui::ui_element::UiElement};

#[derive(Clone, PartialEq)]
pub enum CardType {
    INFO,
    WARNING,
    ERROR
}

#[derive(Clone, PartialEq)]
pub struct Card {
    pub description: String,
    pub card_type: CardType
}

impl Card {
    pub fn new(description: String) -> Self {
        Self {
            description,
            card_type: CardType::INFO
        }
    }

    pub fn update(&mut self, description: String) {
        self.description = description;
    }

    pub fn get_lines(&self, max_width: usize) -> Vec<String> {
        self.description.chars()
            .collect::<Vec<char>>()
            .chunks(max_width).into_iter()
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
    }
}

impl CardType {
    pub fn style(&self) -> ContentStyle {
        let reset_color = Color::Rgb { r: 22, g: 22, b: 23 };
        let fg = Color::Rgb { r: 201, g: 199, b: 205 };

        match self {
            Self::INFO => { return ContentStyle::new().on(reset_color.clone()).with(fg.clone()) }
            Self::WARNING => { return ContentStyle::new().on(reset_color.clone()).yellow() }
            Self::ERROR => { return ContentStyle::new().on(reset_color.clone()).red() }
        }
    }
}

impl UiElement for Card {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn render(&self, frame: &mut Grid<RenderCell>) {
        /*
        if self.description.is_empty() { return }
        let top_left = '╭';
        let top_right = '╮';
        let bottom_left = '╰';
        let bottom_right = '╯';

        let horizontal = '─';
        let vertical = '│';

        let max_width = 63;
        let max_height = 12;
        let padding = 1;
        
        let lines = self.get_lines(max_width - 2 - (padding * 2));
        let width = lines[0].len().min(max_width) + (padding * 2) + 2;
        let height = (lines.len() + 2).clamp(3, max_height);
        let offset = frame[0].cells.len() - width - 1;
        let style = self.card_type.style();

        let frame_height = frame.len().clone();

        for y in 0..height {
            let mut render_line = frame[frame.len() - 1 - (height - y)].clone();            
            let mut char: char;
            for x in 0..width {
                if y == 0 {
                    if x == 0 {
                        char = top_left.clone();
                    } else if x == width - 1 {
                        char = top_right.clone();
                    } else {
                        char = horizontal.clone();
                    }
                } else if y == height - 1 {
                    if x == 0 {
                        char = bottom_left.clone();
                    } else if x == width - 1 {
                        char = bottom_right.clone();
                    } else {
                        char = horizontal.clone();
                    }
                } else {
                    if x == 0 || x == width - 1 {
                        char = vertical.clone();
                    } else if x <= padding || x >= width - 1 - padding {
                        char = ' ';
                    } else { 
                        let mut chars = lines[y - 1].chars();
                        char = chars.nth(x - 1 - padding).unwrap_or(' ');
                    }
                }
                render_line.cells[x + offset] = RenderCell { ch: char.to_string(), style: style };

            }
            frame[frame_height - 1 - (height - y)] = render_line
        }
        */
    }
}
