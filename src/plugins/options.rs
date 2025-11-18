use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Options {
    pub relative_numbers: Option<bool>,
    pub natural_scroll: Option<bool>,
    pub tab_size: Option<usize>
}

impl Options {
    pub fn merge(&self, base: &Options) -> Options {
        Options {
            relative_numbers: self.relative_numbers.or(base.relative_numbers),
            natural_scroll: self.natural_scroll.or(base.natural_scroll),
            tab_size: self.tab_size.or(base.tab_size),
        }
    }
}
