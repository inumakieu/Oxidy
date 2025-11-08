pub mod crossterm;

use crate::buffer::Buffer;
use crate::highlighter::Highlighter;
use crate::types::Size;
use crate::ui::ui_manager::UiManager;

pub trait Renderer {
    fn begin_frame(&mut self);
    fn draw_buffer(&mut self, buffer: &Buffer, ui: &UiManager, highlighter: &mut Highlighter);
    fn end_frame(&mut self);
    fn resize(&mut self, new_size: Size);
}
