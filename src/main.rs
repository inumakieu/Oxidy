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
use winit::keyboard::{PhysicalKey, KeyCode};
use winit::event::ElementState;
use winit::event::Ime;
use winit::keyboard::Key::Character;

use crate::input::{InputHandler, CrosstermInput, WgpuInput};
use crate::renderer::Renderer;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::renderer::crossterm::CrossTermRenderer;
use crate::types::{Size, EditorAction, Direction, Key};

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

use std::time::{Instant, Duration};
use std::collections::HashMap;

struct KeyRepeatState {
    last_movement: Option<HashMap<crate::types::Key, Instant>>,
}

fn gui_main(file_paths: Vec<String>) -> io::Result<()> {
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
    window.set_ime_allowed(true);

    let mut wgpu_renderer = WgpuRenderer::new(&window);

    window.request_redraw();

    let size = Size { cols: (wgpu_renderer.size.width as f32 / 28f32) as u16, rows: (wgpu_renderer.size.height as f32 / 28f32) as u16 };

    let input = Box::new(WgpuInput::new());
    
    let mut app = App::new(size, Box::new(wgpu_renderer), input);

    if let Some(input_file) = file_paths.first() {
        app.open_file(input_file.clone());
    }

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
                    app.renderer.resize(
                        Size {
                            cols: new_size.width as u16,
                            rows: new_size.height as u16
                        }
                    );

                    if let Some(wgpu_renderer) = app.renderer.as_any_mut().downcast_mut::<WgpuRenderer>() {
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
                    
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::RedrawRequested,
                    ..
                } => {
                    app.step();
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::KeyboardInput { event: input_data, .. },
                    ..
                } => {
                    /*if let Some(text) = &input_data.text {
                        if !text.is_empty() {
                            for ch in text.chars().filter(|c| !c.is_control()) {
                                app.editor.handle_action(&EditorAction::InsertChar(ch));
                                window.request_redraw();
                                return
                            }
                        }
                    }*/

                    let key = match map_winit_key(&input_data.logical_key) {
                        Some(k) => k,
                        None => return, // unmapped key
                    };
                    
                    let now = Instant::now();
                    let mut allow_move = true;

                    const REPEAT_DELAY: Duration = Duration::from_millis(300);
                    const REPEAT_RATE: Duration  = Duration::from_millis(40);

                    // Check repeat map without holding a borrow across handle_input
                    {
                        let last_movement = app.key_repeat.last_movement.get_or_insert_with(HashMap::new);
                        allow_move = match last_movement.get(&key) {
                            Some(last) => now.duration_since(*last) >= REPEAT_RATE,
                            None => true,
                        };
                    }

                    match input_data.state {
                        ElementState::Pressed => {
                            let modifiers = crate::types::Modifiers {
                                shift: false,
                                ctrl: false,
                                alt: false,
                                super_key: false,
                            };

                            let input = crate::input::InputEvent::Key {
                                key,
                                modifiers
                            };

                            if allow_move {
                                app.handle_input(input);

                                let last_movement = app.key_repeat.last_movement.get_or_insert_with(HashMap::new);
                                last_movement.insert(key, now);
                                window.request_redraw();
                            }
                        }

                        ElementState::Released => {
                            let last_movement = app.key_repeat.last_movement.get_or_insert_with(HashMap::new);
                            last_movement.remove(&key);
                        }
                    }                
                }
                _ => {}
            }
        })
        .unwrap();

    Ok(())
}

fn map_winit_key(key: &winit::keyboard::Key) -> Option<Key> {
    use winit::keyboard::{Key as WKey, NamedKey};

    match key {
        WKey::Named(named) => match named {
            NamedKey::ArrowUp => Some(Key::Up),
            NamedKey::ArrowDown => Some(Key::Down),
            NamedKey::ArrowLeft => Some(Key::Left),
            NamedKey::ArrowRight => Some(Key::Right),

            NamedKey::Backspace => Some(Key::Backspace),
            NamedKey::Enter => Some(Key::Enter),
            NamedKey::Escape => Some(Key::Esc),
            NamedKey::Tab => Some(Key::Tab),

            _ => None,
        },

        WKey::Character(s) => {
            // Defensive: ignore multi-char weirdness
            s.chars().next().map(Key::Char)
        }

        _ => None,
    }
}


fn tui_main(file_paths: Vec<String>) -> io::Result<()> {
    let term_size = terminal::size().expect("Size could not be determined.");
    let size = Size { cols: term_size.0, rows: term_size.1 };
        
    let input = Box::new(CrosstermInput::new());

    let renderer = Box::new(CrossTermRenderer::new(size.clone()));

    let mut app = App::new(size, renderer, input);

    if let Some(input_file) = file_paths.first() {
        app.open_file(input_file.clone());
    }
    app.run();

    Ok(())
}

struct CliArgs {
    gui: bool,
    files: Vec<String>,
}

fn parse_args() -> CliArgs {
    let mut gui = false;
    let mut files = Vec::new();

    let mut args = std::env::args().skip(1); // skip program name

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-g" | "--gui" => gui = true,
            _ if arg.starts_with('-') => {
                eprintln!("Unknown option: {}", arg);
            }
            _ => files.push(arg),
        }
    }

    CliArgs { gui, files }
}

// Oxidy comment
fn main() -> io::Result<()> {
    let cli = parse_args();

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

    if cli.gui { gui_main(cli.files)?; }
    else { tui_main(cli.files)?; }

    Ok(())
}
