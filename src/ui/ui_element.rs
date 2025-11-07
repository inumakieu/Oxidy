use crate::types::RenderBuffer;

pub trait UiElement {
    fn render(&self, frame: &mut RenderBuffer);
}
