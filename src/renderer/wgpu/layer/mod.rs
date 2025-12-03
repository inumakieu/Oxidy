pub mod background;
pub mod text;
pub mod gutter;
pub mod ui;
pub mod cursor;

use wgpu::{CommandEncoder, RenderPass, TextureView, Device, Queue};
use wgpu::util::StagingBelt;
use winit::dpi::PhysicalSize;
use wgpu_glyph::ab_glyph::{Font, FontArc};

use crate::plugins::config::Config;
use crate::editor::Editor;
use crate::ui::ui_manager::UiManager;
use crate::types::ViewId;

pub fn get_font() -> FontArc {
    let font = FontArc::try_from_slice(include_bytes!(
        "../../../JetBrainsMono-Regular.ttf"
    )).expect("Could not prepare font glyph_brush.");

    font
}

pub trait Layer {
    fn new(device: &Device, render_format: wgpu::TextureFormat) -> Self where Self: Sized;

    fn resize(&mut self, _new_size: PhysicalSize<u32>) {}

    fn update(
        &mut self,
        editor: &Editor,
        ui: &UiManager,
        config: &Config,
        device: &Device,
        queue: &Queue,
        surface_size: PhysicalSize<u32>,
    );

    fn draw(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
        staging_belt: &mut StagingBelt,
        surface_size: PhysicalSize<u32>,
    );
}
