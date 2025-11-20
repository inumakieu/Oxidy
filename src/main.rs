#![allow(warnings)]

use std::io;
use std::env;
use std::io::Write;
use std::panic;
use std::sync::Arc;

pub mod app;
pub mod types;
pub mod highlighter;
pub mod editor;
pub mod plugins;
pub mod lsp;
pub mod buffer;
pub mod renderer;
pub mod input;
pub mod services;
pub mod ui;
pub mod log_manager;
pub mod command;
pub mod keymap;
pub mod logger;

use crossterm::cursor;
use crossterm::terminal;
use crossterm::terminal::EndSynchronizedUpdate;
use crossterm::ExecutableCommand;
use app::App;

use wgpu::CompositeAlphaMode;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text, ab_glyph};

use crate::input::{InputHandler, CrosstermInput, WgpuInput};
use crate::renderer::Renderer;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::renderer::crossterm::CrossTermRenderer;
use crate::types::Size;

use crate::editor::Editor;
use crate::plugins::config::Config;
use crate::ui::ui_manager::UiManager;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        $crate::logger::LOGGER
            .get_or_init(|| $crate::logger::Logger::new())
            .log(format!($($arg)*));
    }};
}

// Oxidy comment
fn main() -> io::Result<()> {
    let mut args = env::args();
    args.next();

    panic::set_hook(Box::new(|info| {
        let _ = std::io::stdout().execute(EndSynchronizedUpdate);
        let _ = std::io::stdout().flush();
        let _ = terminal::disable_raw_mode();
        let _ = std::io::stdout().execute(cursor::Show);
        let _ = std::io::stdout().execute(terminal::LeaveAlternateScreen);

        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "unknown panic"
        };

        let location = info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown location".into());

        eprintln!("\n\nOxidy crashed!\n");
        eprintln!("Reason: {msg}");
        eprintln!("At: {location}");

        // Optional: print backtrace if enabled
        if std::env::var("RUST_BACKTRACE").unwrap_or_default() == "1" {
            eprintln!("\nBacktrace:\n{}", std::backtrace::Backtrace::force_capture());
        }
    }));

    // GUI Rendering
    let gui_mode = false;

    let renderer: Box<dyn Renderer>;
    let input: Box<dyn InputHandler>;
    let size: Size;

    if gui_mode {
        env_logger::init();

        let event_loop = winit::event_loop::EventLoop::new().unwrap();

        let window = Arc::new(
            winit::window::WindowBuilder::new()
                .with_title("Oxidy")
                .with_resizable(true)
                .with_blur(true)
                .build(&event_loop)
                .unwrap(),
        );

        let mut wgpu_renderer = WgpuRenderer::new(&window);

        window.request_redraw();

        size = Size { cols: wgpu_renderer.size.width as u16, rows: wgpu_renderer.size.height as u16 };

        input = Box::new(WgpuInput::new());
        // renderer = Box::new(wgpu_renderer);

        event_loop
        .run(move |event, elwt| {
            match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::Resized(new_size),
                    ..
                } => {
                    wgpu_renderer.size = new_size;

                    wgpu_renderer.surface.configure(
                        &wgpu_renderer.device,
                        &wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: wgpu_renderer.render_format,
                            width: wgpu_renderer.size.width,
                            height: wgpu_renderer.size.height,
                            present_mode: wgpu::PresentMode::AutoVsync,
                            alpha_mode: CompositeAlphaMode::Auto,
                            view_formats: vec![wgpu_renderer.render_format],
                            desired_maximum_frame_latency: 2,
                        },
                    );
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::RedrawRequested,
                    ..
                } => {
                    let config = Config::default();
                    let (event_sender, event_receiver) = std::sync::mpsc::channel();
                    let ui = UiManager::new();
                    let editor = Editor::new(event_sender);
                    wgpu_renderer.draw_buffer(&editor, &ui, &config);
                }
                _ => {}
            }
        })
        .unwrap()
    } else {
        let term_size = terminal::size().expect("Size could not be determined.");
        size = Size { cols: term_size.0, rows: term_size.1 };
        
        input = Box::new(CrosstermInput::new());

        renderer = Box::new(CrossTermRenderer::new(size.clone()));

        let mut app = App::new(size, renderer, input);
    
        if let Some(input_file) = args.next() {
            app.open_file(input_file);
        }
        app.run();
    }
    
    Ok(())
}
