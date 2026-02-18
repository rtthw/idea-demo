//! # Testing Example
//!
//! This only exists as a workspace member to satisfy `rust-analyzer`. This is
//! entirely standalone.

extern crate base;

use base::*;



#[unsafe(no_mangle)]
pub extern "Rust" fn view(context: &mut dyn ViewContext) -> Box<dyn Object> {
    let texture_id = context.load_texture("res/light.png");
    Box::new(
        Flex::column()
            .gap(5.0)
            .with(
                Flex::row()
                    .gap(10.0)
                    .main_align(AxisAlignment::SpaceEvenly)
                    .with(
                        Label::new("2/17/2026, 7:00 AM")
                            .font_size(12.0)
                            .color(Rgba::rgb(0x73, 0x73, 0x89)),
                        0.0,
                    )
                    .with(
                        Label::new("This is a note, or something like that...")
                            .font_size(18.0)
                            .color(Rgba::rgb(0xaa, 0xaa, 0xad)),
                        1.0,
                    ),
                0.0,
            )
            .with(Label::new("Another").font_size(40.0), 0.0)
            .with(
                Flex::row()
                    .gap(10.0)
                    .main_align(AxisAlignment::SpaceEvenly)
                    .with(
                        Label::new("2/17/2026, 7:01 AM")
                            .font_size(12.0)
                            .color(Rgba::rgb(0x73, 0x73, 0x89)),
                        0.0,
                    )
                    .with(
                        Label::new("And this is another note...")
                            .font_size(18.0)
                            .color(Rgba::rgb(0xaa, 0xaa, 0xad)),
                        1.0,
                    ),
                0.0,
            )
            .with(Label::new("Another").font_size(40.0), 0.0)
            .with(TestingObject { texture_id }, 0.0)
            .with(TestingObject { texture_id }, 0.0),
    )
}



struct TestingObject {
    texture_id: u64,
}

impl Object for TestingObject {
    fn render(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {
        renderer.quad(pass.position(), pass.size(), Rgba::BLACK);
        renderer.image(self.texture_id, pass.position(), pass.size());
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

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::IBeam
    }
}
