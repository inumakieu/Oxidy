use crate::{types::RenderBuffer, ui::ui_element::UiElement};

pub struct UiManager {
    elements: Vec<Box<dyn UiElement>>,
}

impl UiManager {
    pub fn new() -> Self {
        Self {
            elements: Vec::new()
        }
    }

    pub fn add(&mut self, element: impl UiElement + 'static) {

    }

    pub fn render(&self, frame: &mut RenderBuffer) {

    }
}
