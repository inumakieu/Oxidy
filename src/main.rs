use std::io;
use std::env;
use std::io::Write;
use std::panic;

pub mod types;
pub mod highlighter;
pub mod editor;
pub mod plugin_manager;

use crossterm::cursor;
use crossterm::terminal;
use crossterm::terminal::EndSynchronizedUpdate;
use crossterm::ExecutableCommand;
use editor::Editor;

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

        eprintln!("\n\n🛑 Oxidy crashed!\n");
        eprintln!("Reason: {msg}");
        eprintln!("At: {location}");

        // Optional: print backtrace if enabled
        if std::env::var("RUST_BACKTRACE").unwrap_or_default() == "1" {
            eprintln!("\nBacktrace:\n{}", std::backtrace::Backtrace::force_capture());
        }
    }));

    let mut editor = Editor::new();
    
    if let Some(input_file) = args.next() {
        editor.load_file(&input_file)?;
    }
    editor.run()?;
 
    Ok(())
}
