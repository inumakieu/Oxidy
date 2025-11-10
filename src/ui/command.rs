use std::any::Any;

use crossterm::style::{Color, Stylize};

use crate::{types::{RenderCell, RenderLine}, ui::ui_element::UiElement};



pub struct Command {
    pub command: String,
    pub shown: bool,
}

impl Command {
    pub fn new() -> Self {
        Self {
            command: "".to_string(),
            shown: false,
        }
    }
    pub fn update_command(&mut self, new_command: String) {
        self.command = new_command;
    }
}

impl UiElement for Command {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn render(&self, frame: &mut Vec<RenderLine>) {
        if !self.shown { return }

        let mut render_line = RenderLine {
            cells: Vec::new()
        };

        let text = self.command.clone().on(Color::Reset).white();

        render_line.cells.push(
            RenderCell { ch: "     ".to_string(), style: text.style().clone() }
        );
   
        for ch in text.content().chars() {
            render_line.cells.push(
                RenderCell { ch: ch.to_string(), style: text.style().clone() }
            );
        }

        frame[1] = render_line;
    }
}
