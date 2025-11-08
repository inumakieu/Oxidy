use crate::{types::{RenderBuffer, RenderLine}, ui::ui_element::UiElement};

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
        self.elements.push(Box::new(element));
    }

    pub fn render(&self, frame: &mut Vec<RenderLine>) {
        for element in &self.elements {
            element.render(frame);
        }
    }
}
