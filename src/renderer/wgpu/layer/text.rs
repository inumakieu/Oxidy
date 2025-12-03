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

pub struct TextLayer {
    font: FontArc,
    glyph_brush: GlyphBrush<()>,
    font_scale: f32,
}

impl Layer for TextLayer {
    fn new(device: &Device, render_format: wgpu::TextureFormat) -> Self {
        let font = get_font();
        let glyph_brush = GlyphBrushBuilder::using_font(font.clone())
            .build(device, render_format);

        Self {
            font,
            glyph_brush,
            font_scale: 26.0,        
        }
    }

    fn update(
        &mut self,
        editor: &Editor,
        _ui: &UiManager,
        config: &Config,
        _device: &Device,
        _queue: &Queue,
        _surface_size: PhysicalSize<u32>,
    ) {
        let buf_view = editor.active_view().unwrap();
        let buffer = editor.active_buffer().unwrap();
        let theme = config.current_theme();
        let fg = hex_to_wgpu_color(&theme.Foreground.unwrap_or_default());

        let layout = Layout::default_single_line();

        let max_line_number_on_screen = buf_view.visible_top() + buf_view.size.rows as usize;
        let start_x = 20.0 + calculate_gutter_width(&self.font, &self.font_scale, max_line_number_on_screen);
        
        for i in 0..(buf_view.size.rows as usize) {
            let line_index = i + buf_view.visible_top();
            if let Some(line) = buffer.lines.get(line_index) {
                self.glyph_brush.queue(Section {
                    screen_position: (start_x, status_bar_height() + (self.font_scale + 2.0) * i as f32),
                    bounds: (_surface_size.width as f32, _surface_size.height as f32),
                    layout,
                    text: vec![
                        Text::new(line)
                            .with_color([fg.r as f32, fg.g as f32, fg.b as f32, fg.a as f32])
                            .with_scale(self.font_scale),
                    ],
                    ..Section::default()
                });
            }
        }
    }

    fn draw(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        _queue: &Queue,
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
            .expect("Draw queued");
    }
}
