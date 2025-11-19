use std::collections::HashMap;

use crate::types::{Key, Modifiers, EditorAction, EditorMode};
use crate::input::InputEvent;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct KeyCombo {
    pub key: Key,
    pub mods: Modifiers,
}

impl KeyCombo {
    pub fn from_str(s: &str) -> Self {
        // Case 1: Single character without brackets
        if !s.starts_with('<') && s.len() == 1 {
            return KeyCombo {
                key: Key::Char(s.chars().next().unwrap()),
                mods: Modifiers::default(),
            };
        }

        // Case 2: Bracketed key combo like "<C-s>"
        if s.starts_with('<') && s.ends_with('>') {
            let inner = &s[1..s.len()-1]; // remove < >

            // Split by '-' to get tokens
            let parts: Vec<&str> = inner.split('-').collect();

            let mut mods = Modifiers::default();
            let mut key = Key::Unknown;

            for (i, part) in parts.iter().enumerate() {
                let p = part.to_lowercase();

                // If this is NOT the last token, it's modifier
                let is_last = i == parts.len() - 1;

                if !is_last {
                    match p.as_str() {
                        "c" | "ctrl" => mods.ctrl = true,
                        "a" | "alt"  => mods.alt = true,
                        "s" | "shift" => mods.shift = true,
                        "super" | "cmd" | "meta" => mods.super_key = true,
                        _ => {}
                    }
                    continue;
                }

                // Last token = key
                key = match p.as_str() {
                    "esc" => Key::Esc,
                    "enter" => Key::Enter,
                    "ret" | "return" => Key::Enter,
                    "tab" => Key::Tab,
                    "backspace" => Key::Backspace,
                    "bs" => Key::Backspace,
                    "left" => Key::Left,
                    "right" => Key::Right,
                    "up" => Key::Up,
                    "down" => Key::Down,
                    "home" => Key::Home,
                    "end" => Key::End,
                    "pageup" => Key::PageUp,
                    "pagedown" => Key::PageDown,
                    "delete" | "del" => Key::Delete,
                    "insert" | "ins" => Key::Insert,

                    // Single-character key: <C-x>
                    c if c.len() == 1 => {
                        let ch = c.chars().next().unwrap();
                        Key::Char(ch)
                    }

                    _ => Key::Unknown
                };
            }

            return KeyCombo { key, mods };
        }

        // Fallback: unknown
        KeyCombo {
            key: Key::Unknown,
            mods: Modifiers::default(),
        }
    }

    pub fn from_input_event(event: &InputEvent) -> Option<Self> {
        match event {
            InputEvent::Key { key, modifiers } => {
                Some(KeyCombo {
                    key: *key,
                    mods: *modifiers,
                })
            }
            _ => None,
        }
    }
}

pub struct Keymap {
    normal: HashMap<KeyCombo, EditorAction>,
    insert: HashMap<KeyCombo, EditorAction>,
    command: HashMap<KeyCombo, EditorAction>,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            normal: HashMap::new(),
            insert: HashMap::new(),
            command: HashMap::new(),
        }
    }

    pub fn resolve(&self, input: InputEvent, mode: &EditorMode) -> Option<EditorAction> {
        let combo = KeyCombo::from_input_event(&input);

        let table = match mode {
            EditorMode::Normal => &self.normal,
            EditorMode::Insert => &self.insert,
            EditorMode::Command => &self.command,
        };

        if let Some(ref c) = combo {
            if let Some(action) = table.get(c) {
                return Some(action.clone());
            }
        }

        if let EditorMode::Insert = mode {
            if let InputEvent::Key { key: Key::Char(ch), modifiers } = input {
                if !modifiers.ctrl && !modifiers.alt {
                    return Some(EditorAction::InsertChar(ch));
                }
            }
        }

        if let EditorMode::Command = mode {
            if let InputEvent::Key { key: Key::Char(ch), modifiers } = input {
                if !modifiers.ctrl && !modifiers.alt {
                    // return Some(EditorAction::InsertCommandChar(ch));
                }
            }
        }

        None
    }

    pub fn normal(&mut self) -> KeymapBuilder {
        KeymapBuilder { map: &mut self.normal }
    }

    pub fn insert(&mut self) -> KeymapBuilder {
        KeymapBuilder { map: &mut self.insert }
    }

    pub fn command(&mut self) -> KeymapBuilder {
        KeymapBuilder { map: &mut self.command }
    }
}

pub struct KeymapBuilder<'a> {
    map: &'a mut HashMap<KeyCombo, EditorAction>,
}

impl<'a> KeymapBuilder<'a> {
    pub fn map(mut self, key: &str, action: EditorAction) -> Self {
        let combo = KeyCombo::from_str(key);
        self.map.insert(combo, action);
        self
    }
}
