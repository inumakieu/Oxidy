use std::any::Any;

use crate::types::{RenderCell, Grid};

pub trait UiElement {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn render(&self, frame: &mut Grid<RenderCell>);
}
