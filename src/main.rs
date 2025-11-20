#![allow(warnings)]

use std::io;
use std::env;
use std::io::Write;
use std::panic;

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

use crate::input::CrosstermInput;
use crate::renderer::crossterm::CrossTermRenderer;
use crate::types::Size;

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


    let term_size = terminal::size().expect("Size could not be determined.");
    let size = Size { cols: term_size.0, rows: term_size.1 };
    let renderer = CrossTermRenderer::new(size.clone());
    let input = CrosstermInput::new(); 
    
    let mut app = App::new(size, Box::new(renderer), Box::new(input));
    
    if let Some(input_file) = args.next() {
        app.open_file(input_file);
    }
    app.run();

    Ok(())
}
