pub mod crossterm;

use crate::buffer::Buffer;
use crate::highlighter::Highlighter;
use crate::plugins::config::Config;
use crate::types::{EditorMode, Size, RenderCell, Grid};
use crate::ui::ui_manager::UiManager;
use crate::editor::Editor;

pub trait Renderer {
    fn begin_frame(&mut self);
    fn draw_buffer(&mut self, editor: &Editor, ui: &UiManager, config: &Config);
    fn end_frame(&mut self);
    fn resize(&mut self, new_size: Size);
}

pub trait Layer {
    fn render(editor: &Editor, ui: &UiManager, config: &Config, size: Size) -> Grid<RenderCell>;
}
