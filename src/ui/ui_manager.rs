use crate::{types::{RenderBuffer, RenderCell, Grid}, ui::ui_element::UiElement};

pub struct UiManager {
    elements: Vec<Box<dyn UiElement>>,
}

impl UiManager {
    pub fn new() -> Self {
        Self {
            elements: Vec::new()
        }
    }

    pub fn top_offset(&self) -> usize {
        return 1;
    }

    pub fn add(&mut self, element: impl UiElement + 'static) {
        self.elements.push(Box::new(element));
    }

    pub fn get<T: UiElement + 'static>(&self) -> Option<&T> {
        for element in &self.elements {
            if let Some(found) = element.as_any().downcast_ref::<T>() {
                return Some(found);
            }
        }
        None
    }

    pub fn get_mut<T: UiElement + 'static>(&mut self) -> Option<&mut T> {
        for element in &mut self.elements {
            if let Some(found) = element.as_any_mut().downcast_mut::<T>() {
                return Some(found);
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Grid<RenderCell>) {
        for element in &self.elements {
            element.render(frame);
        }
    }
}
