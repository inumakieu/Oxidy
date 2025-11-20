use wgpu::{Surface, Instance, Device, Queue, TextureFormat};
use winit::window::Window;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text, ab_glyph, GlyphBrush};
use wgpu::CompositeAlphaMode;
use wgpu::util::StagingBelt;
use winit::dpi::PhysicalSize;

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
    pub render_format: TextureFormat,

    pub size: PhysicalSize<u32>
}

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
        let inconsolata = ab_glyph::FontArc::try_from_slice(include_bytes!(
            "../JetBrainsMono-Regular.ttf"
        )).expect("Could not prepare font glyph_brush.");

        let mut glyph_brush = GlyphBrushBuilder::using_font(inconsolata)
            .build(&device, render_format);


        Self {
            surface,
            instance,
            device,
            queue,
            staging_belt,
            glyph_brush,
            render_format,
            size: inner_size
        }
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

        self.glyph_brush.queue(Section {
            screen_position: (30.0, 30.0),
            bounds: (self.size.width as f32, self.size.height as f32),
            text: vec![
                Text::new("let variable = String(\"Oxidy!!!\")")
                    .with_color([fg.r as f32, fg.g as f32, fg.b as f32, fg.a as f32])
                    .with_scale(26.0),
            ],
            ..Section::default()
        });

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

        // Submit the work!
        self.staging_belt.finish();
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        // Recall unused staging buffers
        self.staging_belt.recall();
    }
    
    fn end_frame(&mut self) {}
    
    fn resize(&mut self, new_size: Size) {}
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
