use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LspConfig {
    pub extensions: Vec<String>,
    pub command: String,
    pub args: Vec<String>
}
