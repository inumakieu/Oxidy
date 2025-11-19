use std::io::Result;
use std::collections::HashMap;

use crate::editor::Editor;

pub type CommandFn = fn(&mut Editor, Vec<String>) -> Result<()>;

pub struct Command {
    pub name: String,
    pub description: String,
    pub execute: CommandFn,
}

pub struct CommandManager {
    commands: HashMap<String, Command>,
}

impl CommandManager {
    pub fn new() -> Self {
        Self { commands: HashMap::new() }
    }

    pub fn register_commands(&mut self) {}

    pub fn register(&mut self, cmd: Command) {
        self.commands.insert(cmd.name.clone(), cmd);
    }

    pub fn execute(&mut self, name: &str, args: Vec<String>, editor: &mut Editor) -> Result<()> {
        if let Some(cmd) = self.commands.get(name) {
            let _ = (cmd.execute)(editor, args);
        }

        Ok(())
    }
}
