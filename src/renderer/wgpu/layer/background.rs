use wgpu::{Device, CommandEncoder, TextureView, Queue};
use wgpu::util::StagingBelt;
use winit::dpi::PhysicalSize;

use super::Layer;
use crate::plugins::config::Config;
use crate::editor::Editor;
use crate::ui::ui_manager::UiManager;
use crate::renderer::wgpu::utils::hex_to_wgpu_color;

pub struct BackgroundLayer;

impl Layer for BackgroundLayer {
    fn new(_device: &Device, _render_format: wgpu::TextureFormat) -> Self {
        Self
    }

    fn update(
        &mut self,
        _editor: &Editor,
        _ui: &UiManager,
        _config: &Config,
        _device: &Device,
        _queue: &Queue,
        _surface_size: PhysicalSize<u32>,
    ) {
        // No updates needed for a static background color
    }

    fn draw(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        _device: &Device,
        _queue: &Queue,
        _staging_belt: &mut StagingBelt,
        _surface_size: PhysicalSize<u32>,
    ) {
        // The background clear color will be handled by the renderer's initial render pass
        // when creating the `RenderPassDescriptor`. This layer mostly acts as a placeholder
        // if more complex background drawing were needed in the future.
    }
}
