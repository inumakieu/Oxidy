use wgpu::{Surface, Instance, Device, Queue, TextureFormat};
use winit::window::Window;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text, ab_glyph, GlyphBrush};
use wgpu::CompositeAlphaMode;
use wgpu::util::StagingBelt;
use winit::dpi::PhysicalSize;
use wgpu::util::BufferInitDescriptor;
use wgpu::BufferUsages;
use wgpu_glyph::ab_glyph::Font;
use wgpu_glyph::ab_glyph::ScaleFont;

use std::sync::Arc;

use crate::buffer::{Buffer, BufferView};
use crate::highlighter::Highlighter;
use crate::plugins::config::Config;
use crate::types::{EditorMode, Size, RenderCell, Grid, Rect, ViewId};
use crate::ui::ui_manager::UiManager;
use crate::editor::Editor;
use crate::renderer::Renderer;

pub struct WgpuRenderer {
    pub surface: Surface<'static>,
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub staging_belt: StagingBelt,
    pub glyph_brush: GlyphBrush<()>,
    font: ab_glyph::FontArc,
    font_scale: f32,
    pub render_format: TextureFormat,

    pub size: PhysicalSize<u32>,

    // Cursor resources
    pub cursor_pipeline: wgpu::RenderPipeline,
    pub cursor_vertex_buffer: wgpu::Buffer,
}

struct CursorVertex {
    pos: [f32; 2],
}

const CURSOR_WGSL: &str = r#"
@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // cursor color (RGBA linear)
    return vec4<f32>(0.95, 0.95, 0.95, 1.0);
}
"#;

impl WgpuRenderer {
    pub fn new(window: &Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        
        let (device, queue) = futures::executor::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("Request adapter");

            adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .expect("Request device")
        });

        // Create staging belt
        let mut staging_belt = wgpu::util::StagingBelt::new(1024);

        // Prepare swap chain
        let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let mut inner_size = window.inner_size();

        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: render_format,
                width: inner_size.width,
                height: inner_size.height,
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        // Prepare glyph_brush
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!(
            "../JetBrainsMono-Regular.ttf"
        )).expect("Could not prepare font glyph_brush.");


        let mut glyph_brush = GlyphBrushBuilder::using_font(font.clone())
            .build(&device, render_format);

        // Prepare cursor
        let cursor_pipeline = Self::create_cursor_pipeline(&device, render_format);

        // create an initially-empty vertex buffer sized for 6 vertices (2 floats each)
        let vb_size = (6 * 2 * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let cursor_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cursor VB"),
            size: vb_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            instance,
            device,
            queue,
            staging_belt,
            glyph_brush,
            font,
            font_scale: 26.0,
            render_format,
            size: inner_size,

            cursor_pipeline,
            cursor_vertex_buffer
        }
    }

    fn create_cursor_pipeline(device: &Device, surface_format: TextureFormat) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cursor shader"),
            source: wgpu::ShaderSource::Wgsl(CURSOR_WGSL.into()),
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
            cache: None
        })
    }

    /// Update cursor vertex buffer for this frame.
    /// x_px, y_top_px, y_bot_px and width_px are in *pixels* relative to the top-left of the surface.
    /// This function converts to NDC and uploads the 6-vertex quad to the GPU using queue.write_buffer.
    fn update_cursor_buffer(&mut self, x_px: f32, y_top_px: f32, y_bot_px: f32, width_px: f32) {
        let w = self.size.width as f32;
        let h = self.size.height as f32;

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
        self.queue.write_buffer(&self.cursor_vertex_buffer, 0, bytes);
    }

    pub fn caret_x_for_line(&self, line: &str, col: usize, start_x: f32) -> f32 {
        // Make a scaled view of the font at the pixel size you use for glyph_brush.
        // font_scale should be the same value you used when creating Sections (.with_scale(...)).
        let scaled_font = self.font.as_scaled(self.font_scale);

        let mut x = start_x;
        let mut prev_gid: Option<ab_glyph::GlyphId> = None;

        for (i, ch) in line.chars().enumerate() {
            if i == col {
                break;
            }

            let gid = scaled_font.glyph_id(ch);

            // Apply kerning between previous glyph and this glyph (if any)
            if let Some(prev) = prev_gid {
                x += scaled_font.kern(prev, gid);
            }

            // Advance for this glyph (already scaled to pixels)
            x += scaled_font.h_advance(gid);

            prev_gid = Some(gid);
        }

        x
    }
}

impl Renderer for WgpuRenderer {
    fn begin_frame(&mut self) {}

    fn draw_buffer(&mut self, editor: &Editor, ui: &UiManager, config: &Config) {
        // Get a command encoder for the current frame
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Redraw"),
            },
        );

        // Get the next frame
        let frame =
            self.surface.get_current_texture().expect("Get next frame");
        let view = &frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());


        let theme = config.current_theme();

        let bg = hex_to_wgpu_color(&theme.Background.unwrap_or_default());
        let fg = hex_to_wgpu_color(&theme.Foreground.unwrap_or_default());
        // Clear frame
        {
            let _ = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Render pass"),
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(
                                    bg,
                                ),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        },
                    )],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                },
            );
        }

        let buf_view = editor.active_view().unwrap().clone();

        for i in 0..(buf_view.size.rows as usize) {
            if let Some(line) = editor.active_buffer().unwrap().lines.get(i + buf_view.visible_top()).clone() {
                self.glyph_brush.queue(Section {
                    screen_position: (30.0, 30.0 + (28 * i) as f32),
                    bounds: (self.size.width as f32, self.size.height as f32),
                    text: vec![
                        Text::new(line)
                            .with_color([fg.r as f32, fg.g as f32, fg.b as f32, fg.a as f32])
                            .with_scale(26.0),
                    ],
                    ..Section::default()
                });
            }
        }

        // Draw the text!
        self.glyph_brush
            .draw_queued(
                &self.device,
                &mut self.staging_belt,
                &mut encoder,
                view,
                self.size.width,
                self.size.height,
            )
            .expect("Draw queued");

        {
            let cursor_width_px = 2.0_f32;
            let mut cursor_x_px = 30.0;

            let font = &self.font;       // the same FontArc you gave to glyph_brush
            let scale = self.font_scale; // the same scale as your glyph sections

            if let Some(buffer) = editor.active_buffer() {
                if let Some(line) = buffer.lines.get(buf_view.cursor.row) {
                    cursor_x_px = self.caret_x_for_line(line, buf_view.cursor.col, 30.0);
                }
            }
            let line_top = 30.0 + (28.0 * (buf_view.cursor.row - buf_view.scroll.vertical) as f32); // Replace: compute Y for the caret's line
            let line_bottom = line_top + 26.0; // approximate line height (scale 26.0 in glyph_brush)

            // Update GPU vertex buffer for this frame
            self.update_cursor_buffer(cursor_x_px, line_top, line_bottom, cursor_width_px);

            // Begin cursor render pass (draw the updated buffer)
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

            rpass.set_pipeline(&self.cursor_pipeline);
            rpass.set_vertex_buffer(0, self.cursor_vertex_buffer.slice(..));
            rpass.draw(0..6, 0..1);
        }

        // Submit the work!
        self.staging_belt.finish();
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        // Recall unused staging buffers
        self.staging_belt.recall();
    }
    
    fn end_frame(&mut self) {}
    
    fn resize(&mut self, new_size: Size) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        return self
    }
}

pub fn hex_to_wgpu_color(hex: &str) -> wgpu::Color {
    let (r8, g8, b8) = parse_hex(hex);

    let r = srgb_to_linear(r8 as f32 / 255.0);
    let g = srgb_to_linear(g8 as f32 / 255.0);
    let b = srgb_to_linear(b8 as f32 / 255.0);

    wgpu::Color {
        r: r as f64,
        g: g as f64,
        b: b as f64,
        a: 1.0,
    }
}

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    (r, g, b)
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
