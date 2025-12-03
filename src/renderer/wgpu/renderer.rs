use wgpu::{Surface, Instance, Device, Queue, TextureFormat};
use winit::window::Window;
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

use crate::renderer::wgpu::layer::{Layer, background::BackgroundLayer, text::TextLayer, gutter::GutterLayer, cursor::CursorLayer, ui::UiLayer};
use crate::renderer::wgpu::utils::{hex_to_wgpu_color, srgb_to_linear};
use crate::renderer::Renderer;

pub struct WgpuRenderer {
    pub surface: Surface<'static>,
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub staging_belt: StagingBelt,
    pub render_format: TextureFormat,

    pub size: PhysicalSize<u32>,

    layers: Vec<Box<dyn Layer>>,
}

impl WgpuRenderer {
    pub fn new(window: &Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let mut render_format: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
        let mut alpha_mode: CompositeAlphaMode = CompositeAlphaMode::PreMultiplied;

        let (device, queue) = futures::executor::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("Request adapter");
            let caps = surface.get_capabilities(&adapter);


            render_format = caps.formats[0];
            alpha_mode = caps.alpha_modes
                .iter()
                .copied()
                .find(|m| *m != CompositeAlphaMode::Opaque)
                .unwrap_or(CompositeAlphaMode::Opaque);
            dbg!(&render_format);
            dbg!(&alpha_mode);

            adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .expect("Request device")
        });

        let staging_belt = wgpu::util::StagingBelt::new(1024);
       
        let mut inner_size = window.inner_size();

        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: render_format,
                width: inner_size.width,
                height: inner_size.height,
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        let mut layers: Vec<Box<dyn Layer>> = Vec::new();
        layers.push(Box::new(BackgroundLayer::new(&device, render_format)));
        layers.push(Box::new(GutterLayer::new(&device, render_format)));
        layers.push(Box::new(TextLayer::new(&device, render_format)));
        layers.push(Box::new(UiLayer::new(&device, render_format)));
        layers.push(Box::new(CursorLayer::new(&device, render_format)));

        for layer in &mut layers {
            layer.resize(inner_size);
        }

        Self {
            surface,
            instance,
            device,
            queue,
            staging_belt,
            render_format,
            size: inner_size,
            layers,
        }
    }
}

impl Renderer for WgpuRenderer {
    fn begin_frame(&mut self) {}

    fn draw_buffer(&mut self, editor: &Editor, ui: &UiManager, config: &Config) {
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Redraw"),
            },
        );

        let frame = self.surface.get_current_texture().expect("Get next frame");
        let view = &frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let theme = config.current_theme();
        let mut bg_color = hex_to_wgpu_color(&theme.Background.unwrap_or_default());
        
        bg_color.a = 0.5;
        {
            let _render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Main Render Pass"),
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(bg_color),
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

        for layer in &mut self.layers {
            layer.update(editor, ui, config, &self.device, &self.queue, self.size);
            layer.draw(&mut encoder, view, &self.device, &self.queue, &mut self.staging_belt, self.size);
        }

        self.staging_belt.finish();
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        
        self.staging_belt.recall();
    }

    fn end_frame(&mut self) {}

    fn resize(&mut self, new_size: Size) {
        if new_size.cols > 0 && new_size.rows > 0 {
            self.size = PhysicalSize::new(new_size.cols as u32, new_size.rows as u32);
            self.surface.configure(
                &self.device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: self.render_format,
                    width: self.size.width,
                    height: self.size.height,
                    present_mode: wgpu::PresentMode::AutoVsync,
                    alpha_mode: CompositeAlphaMode::Auto,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                },
            );

            for layer in &mut self.layers {
                layer.resize(self.size);
            }
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
