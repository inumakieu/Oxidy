use crate::types::RenderLine;

pub trait UiElement {
    fn render(&self, frame: &mut Vec<RenderLine>);
}
