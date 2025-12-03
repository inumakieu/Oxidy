use wgpu::{Device, CommandEncoder, TextureView, Queue};
use wgpu::util::StagingBelt;
use winit::dpi::PhysicalSize;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text, ab_glyph, GlyphBrush, Layout};
use wgpu_glyph::ab_glyph::FontArc;

use super::{Layer, get_font};
use super::gutter::GutterLayer;
use crate::plugins::config::Config;
use crate::editor::Editor;
use crate::ui::ui_manager::UiManager;
use crate::renderer::wgpu::utils::{hex_to_wgpu_color, calculate_gutter_width, status_bar_height};

pub struct UiLayer {
    glyph_brush: GlyphBrush<()>,
    font: ab_glyph::FontArc,
    font_scale: f32,
}

impl Layer for UiLayer {
    fn new(device: &Device, render_format: wgpu::TextureFormat) -> Self where Self: Sized {
        let font = get_font();
        let glyph_brush = GlyphBrushBuilder::using_font(font.clone())
            .build(device, render_format);

        Self {
            glyph_brush,
            font: font,
            font_scale: 26.0,
        }
    }

    fn resize(&mut self, _new_size: PhysicalSize<u32>) {}

    fn update(
        &mut self,
        editor: &Editor,
        ui: &UiManager,
        config: &Config,
        device: &Device,
        queue: &Queue,
        surface_size: PhysicalSize<u32>,
    ) {
        let theme = config.current_theme();
        let fg = hex_to_wgpu_color(&theme.Foreground.unwrap_or_default());
        let layout = Layout::default_single_line();
        
        // TODO: Render ui based on ui parameter
        self.glyph_brush.queue(Section {
            screen_position: (20.0 + 8.0, 20.0 + 8.0),
            bounds: (surface_size.width as f32, surface_size.height as f32),
            layout,
            text: vec![
                Text::new("Oxidy")
                    .with_color([fg.r as f32, fg.g as f32, fg.b as f32, fg.a as f32])
                    .with_scale(self.font_scale),
            ],
            ..Section::default()
        });

    }

    fn draw(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
        staging_belt: &mut StagingBelt,
        surface_size: PhysicalSize<u32>,
    ) {
        self.glyph_brush
            .draw_queued(
                device,
                staging_belt,
                encoder,
                view,
                surface_size.width,
                surface_size.height,
            )
            .expect("Draw queued for ui");

    }
}
