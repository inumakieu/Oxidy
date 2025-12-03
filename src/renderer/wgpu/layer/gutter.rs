use wgpu::{Device, CommandEncoder, TextureView, Queue};
use wgpu::util::StagingBelt;
use winit::dpi::PhysicalSize;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text, GlyphBrush, Layout};
use wgpu_glyph::ab_glyph::{self, Font, FontArc, ScaleFont};

use super::{Layer, get_font};
use crate::plugins::config::Config;
use crate::editor::Editor;
use crate::ui::ui_manager::UiManager;
use crate::renderer::wgpu::utils::{hex_to_wgpu_color, calculate_gutter_width, status_bar_height};

pub struct GutterLayer {
    glyph_brush: GlyphBrush<()>,
    font: ab_glyph::FontArc,
    font_scale: f32,
    gutter_width_px: f32,
}


impl Layer for GutterLayer {
    fn new(device: &Device, render_format: wgpu::TextureFormat) -> Self {
        let font = get_font();
        let glyph_brush = GlyphBrushBuilder::using_font(font.clone())
            .build(device, render_format);

        Self {
            glyph_brush,
            font: font,
            font_scale: 26.0,
            gutter_width_px: 30.0,
        }
    }

    fn update(
        &mut self,
        editor: &Editor,
        _ui: &UiManager,
        config: &Config,
        _device: &Device,
        _queue: &Queue,
        surface_size: PhysicalSize<u32>,
    ) {
        let buf_view = editor.active_view().unwrap();
        let buffer = editor.active_buffer().unwrap();
        let theme = config.current_theme();
        let current_line_color = hex_to_wgpu_color(&theme.Foreground.unwrap_or_default()); // Use a muted color for line numbers
        let normal_line_color = hex_to_wgpu_color(&theme.Comment.unwrap_or_default()); // Use a muted color for line numbers


        let layout = Layout::default_single_line().v_align(wgpu_glyph::VerticalAlign::Center);

        // Update gutter width
        let max_line_number_on_screen = buf_view.visible_top() + buf_view.size.rows as usize;
        self.gutter_width_px = calculate_gutter_width(&self.font, &self.font_scale, max_line_number_on_screen.max(buffer.lines.len()));


        // Clear previous queued text
        // self.glyph_brush.queue_unbounded(Section { ..Default::default() });
        
        let use_relative = config.opt.relative_numbers.unwrap();

        for i in 0..(buf_view.size.rows as usize) {
            // let line_number_ = (i + buf_view.visible_top() + 1).to_string(); // Line numbers are 1-based
            let buffer_row = i + buf_view.visible_top();
            let mut color: [f32; 4] = [
                normal_line_color.r as f32,
                normal_line_color.g as f32,
                normal_line_color.b as f32,
                normal_line_color.a as f32,
            ];

            let line_number: i32 = if use_relative {
                let dist = (buf_view.cursor.row as i32 - buffer_row as i32).abs();
                if dist == 0 {
                    color = [
                        current_line_color.r as f32,
                        current_line_color.g as f32,
                        current_line_color.b as f32,
                        current_line_color.a as f32,
                    ];
                    (buffer_row + 1) as i32
                } else {
                    dist
                }
            } else {
                (buffer_row + 1) as i32
            };

            // Align to the right of the gutter
            let x_pos = self.gutter_width_px - 5.0; // 5px padding from right
            let y_pos = status_bar_height() + (self.font_scale + 2.0) * i as f32 + (self.font_scale / 2.0); // Center text vertically in line

            self.glyph_brush.queue(Section {
                screen_position: (x_pos, y_pos),
                bounds: (self.gutter_width_px, surface_size.height as f32),
                layout: layout.h_align(wgpu_glyph::HorizontalAlign::Right),
                text: vec![
                    Text::new(&line_number.to_string())
                        .with_color(color)
                        .with_scale(self.font_scale),
                ],
                ..Section::default()
            });
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
            .expect("Draw queued for gutter");
    }
}
