use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use crate::plugins::options::Options;
use crate::plugins::statusbar::StatusBarConfig;
use crate::plugins::theme::Theme;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub opt: Options,
    pub theme: Option<String>,
    pub themes: HashMap<String, Theme>,
    pub keymap: HashMap<String, String>,
    pub statusbar: Option<StatusBarConfig>,
    // pub syntax: HashMap<String, SyntaxConfig>,
}

impl Config {
    pub fn merge(&self, base: &Config) -> Self {
        Self {
            opt: self.opt.merge(&base.opt),
            theme: Some(self.theme.clone().unwrap_or(base.theme.clone().unwrap())),
            themes: self.themes.clone(),
            keymap: self.keymap.clone(),
            statusbar: self.statusbar.clone()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            opt: Options {
                relative_numbers: Some(false),
                natural_scroll: Some(false),
                tab_size: Some(2)
            },
            theme: Some("".to_string()),
            themes: HashMap::new(),
            keymap: HashMap::new(),
            statusbar: Some(StatusBarConfig::default())
        }
    }
}
