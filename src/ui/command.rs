use std::any::Any;

use crossterm::style::{Color, Stylize};

use crate::{types::{RenderCell, Grid}, ui::ui_element::UiElement};

pub struct Command {
    pub command: String,
    pub shown: bool,
    pub cursor: usize,
}

impl Command {
    pub fn new() -> Self {
        Self {
            command: "".to_string(),
            shown: false,
            cursor: 0
        }
    }
    
    pub fn update_command(&mut self, new_command: String) {
        self.command = new_command;
    }

    pub fn get_position(&self) -> usize {
        return 6 + self.command.len()
    }
}

impl UiElement for Command {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn render(&self, frame: &mut Grid<RenderCell>) {
        let reset_color = Color::Rgb { r: 22, g: 22, b: 23 };
        let fg = Color::Rgb { r: 201, g: 199, b: 205 };
        if !self.shown { return }

        let mut render_line = vec![RenderCell::space_col(reset_color) ;frame.cells[1].len()];
        let text = self.command.clone().on(reset_color.clone()).with(fg.clone());

        render_line[4] = RenderCell { ch: '', style: text.style().clone(), transparent: false };
   
        for (i, ch) in text.content().chars().enumerate() {
            render_line[i + 6] = RenderCell { ch, style: text.style().clone(), transparent: false };
        }

        frame.cells[1] = render_line;
    }
}
