pub mod crossterm;
pub mod wgpu_renderer;

use crate::buffer::{Buffer, BufferView};
use crate::highlighter::Highlighter;
use crate::plugins::config::Config;
use crate::types::{EditorMode, Size, RenderCell, Grid, Rect, ViewId};
use crate::ui::ui_manager::UiManager;
use crate::editor::Editor;

pub trait Renderer {
    fn begin_frame(&mut self);
    fn draw_buffer(&mut self, editor: &Editor, ui: &UiManager, config: &Config);
    fn end_frame(&mut self);
    fn resize(&mut self, new_size: Size);

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub trait Layer {
    fn render(editor: &Editor, view: &BufferView, ui: &UiManager, config: &Config, rect: Rect) -> Grid<RenderCell>;
}
