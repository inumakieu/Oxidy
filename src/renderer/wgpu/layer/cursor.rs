use wgpu::{Device, CommandEncoder, TextureView, Queue};
use wgpu::util::{StagingBelt, BufferInitDescriptor};
use winit::dpi::PhysicalSize;
use wgpu_glyph::ab_glyph::{self, Font, FontArc, ScaleFont};

use super::{Layer, get_font};
use crate::plugins::config::Config;
use crate::editor::Editor;
use crate::ui::ui_manager::UiManager;
use crate::types::EditorMode;
use crate::renderer::wgpu::utils::{calculate_gutter_width, status_bar_height};

pub struct CursorLayer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    font: FontArc,
    font_scale: f32,
    cursor_width_px: f32,
    surface_size: PhysicalSize<u32>,
}

impl CursorLayer {
    fn create_cursor_pipeline(device: &Device, surface_format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cursor shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/cursor.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cursor pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cursor pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: (2 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default()
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    /// Calculates the pixel X position for the caret on a given line and column.
    fn caret_x_for_line(&self, line: &str, col: usize, start_x: f32) -> f32 {
        let scaled_font = self.font.as_scaled(self.font_scale);
        let mut x = start_x;
        let mut prev_gid: Option<ab_glyph::GlyphId> = None;

        for (i, ch) in line.chars().enumerate() {
            if i == col {
                break;
            }

            let gid = scaled_font.glyph_id(ch);

            if let Some(prev) = prev_gid {
                x += scaled_font.kern(prev, gid);
            }
            x += scaled_font.h_advance(gid);

            prev_gid = Some(gid);
        }
        x
    }

    /// Update cursor vertex buffer for this frame.
    /// x_px, y_top_px, y_bot_px and width_px are in *pixels* relative to the top-left of the surface.
    /// This function converts to NDC and uploads the 6-vertex quad to the GPU using queue.write_buffer.
    fn update_cursor_buffer(&mut self, queue: &Queue, x_px: f32, y_top_px: f32, y_bot_px: f32, width_px: f32) {
        let w = self.surface_size.width as f32;
        let h = self.surface_size.height as f32;

        // Convert to NDC
        let x1 = (x_px / w) * 2.0 - 1.0;
        let x2 = ((x_px + width_px) / w) * 2.0 - 1.0;

        // y in NDC: 1.0 top -> -1.0 bottom, so invert
        let y1 = 1.0 - (y_top_px / h) * 2.0;
        let y2 = 1.0 - (y_bot_px / h) * 2.0;

        // 6 vertices (triangle list) flattened into f32 vector (x,y pairs)
        let raw: [f32; 12] = [
            x1, y1,
            x2, y1,
            x1, y2,

            x1, y2,
            x2, y1,
            x2, y2,
        ];

        // Write the bytes to the buffer
        let bytes = unsafe {
            std::slice::from_raw_parts(
                raw.as_ptr() as *const u8,
                raw.len() * std::mem::size_of::<f32>(),
            )
        };
        queue.write_buffer(&self.vertex_buffer, 0, bytes);
    }
}

impl Layer for CursorLayer {
    fn new(device: &Device, render_format: wgpu::TextureFormat) -> Self {
        let pipeline = Self::create_cursor_pipeline(device, render_format);

        let vb_size = (6 * 2 * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cursor VB"),
            size: vb_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let font = get_font();

        Self {
            pipeline,
            vertex_buffer,
            font,
            font_scale: 26.0,
            cursor_width_px: 2.0,
            surface_size: PhysicalSize::new(1, 1), // Will be updated on first resize
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_size = new_size;
    }

    fn update(
        &mut self,
        editor: &Editor,
        _ui: &UiManager,
        _config: &Config,
        _device: &Device,
        queue: &Queue,
        _surface_size: PhysicalSize<u32>,
    ) {
        let buf_view = editor.active_view().unwrap();
        let buffer = editor.active_buffer().unwrap();
        
        match buf_view.mode {
            EditorMode::Insert | EditorMode::Command => {
                self.cursor_width_px = 2.0;
            }
            EditorMode::Normal => {
                self.cursor_width_px = 12.0;
            }
        }
        let max_line_number_on_screen = buf_view.visible_top() + buf_view.size.rows as usize;
        let mut cursor_x_px = 20.0 + calculate_gutter_width(&self.font, &self.font_scale, max_line_number_on_screen);

        if let Some(line) = buffer.lines.get(buf_view.cursor.row) {
            cursor_x_px = self.caret_x_for_line(line, buf_view.cursor.col, cursor_x_px);
        }

        // TODO: These Y positions should be calculated dynamically from font metrics and line spacing
        // matching what the TextLayer uses.
        let line_top = status_bar_height() + (self.font_scale + 2.0) * (buf_view.cursor.row - buf_view.scroll.vertical) as f32;
        let line_bottom = line_top + self.font_scale; // approximate line height

        self.update_cursor_buffer(queue, cursor_x_px, line_top, line_bottom, self.cursor_width_px);
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
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Cursor pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,   // IMPORTANT: do NOT clear; keep text
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.draw(0..6, 0..1);
    }
}
