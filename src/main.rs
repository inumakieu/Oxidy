use std::io;
use std::env;

pub mod types;
pub mod highlighter;
pub mod editor;
pub mod plugin_manager;

use editor::Editor;

fn main() -> io::Result<()> {
    let mut args = env::args();
    args.next();

    let mut editor = Editor::new();
   
    if let Some(input_file) = args.next() {
        editor.load_file(&input_file)?;
    }
    editor.run()?;
 
    Ok(())
}
