//! # Testing Example
//!
//! This only exists as a workspace member to satisfy `rust-analyzer`. This is
//! entirely standalone.

extern crate base;

use base::*;



#[unsafe(no_mangle)]
pub extern "Rust" fn view() -> Box<dyn Object> {
    Box::new(
        Flex::column()
            .with(TestingObject {}, 0.0)
            .with(TestingObject {}, 0.0),
    )
}



struct TestingObject {}

impl Object for TestingObject {
    fn render(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {
        renderer.quad(pass.position(), pass.size(), Rgba::BLACK);
        renderer.text("EXAMPLE", pass.position(), 30.0, Rgba::WHITE);
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
            .text_size("EXAMPLE", 30.0)
            .value_for_axis(axis)
    }
}
