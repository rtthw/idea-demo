//! # Label
//!
//! See [`Label`] for details.

use crate::{
    Axis, CursorIcon, LayoutPass, LengthRequest, MeasurePass, Object, RenderPass, Renderer, Rgba,
};



/// An [object](Object) that will render a piece of text.
pub struct Label {
    pub content: String,
    pub font_size: f32,
    pub color: Rgba,
}

impl Label {
    /// Create a new label with the provided content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            font_size: 16.0,
            color: Rgba::WHITE,
        }
    }

    /// Defines the font size of this label.
    pub const fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Defines the foreground [color](Rgba) of this label.
    pub const fn color(mut self, color: Rgba) -> Self {
        self.color = color;
        self
    }

    /// Set this label's content to the given value.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    /// Set this label's font size to the given value.
    pub const fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
    }

    /// Set this label's color to the given value.
    pub const fn set_color(&mut self, color: Rgba) {
        self.color = color;
    }
}

impl Object for Label {
    fn render(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {
        renderer.text(&self.content, pass.position(), self.font_size, self.color);
    }

    fn layout(&mut self, _pass: &mut LayoutPass<'_>) {}

    fn measure(
        &mut self,
        pass: &mut MeasurePass<'_>,
        axis: Axis,
        _length_request: LengthRequest,
        _cross_length: Option<f32>,
    ) -> f32 {
        pass.measure_context()
            .text_size(&self.content, self.font_size)
            .value_for_axis(axis)
    }

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::IBeam
    }
}
