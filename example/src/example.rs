//! # Testing Example
//!
//! This only exists as a workspace member to satisfy `rust-analyzer`. This is
//! entirely standalone.

extern crate base;

use std::any::TypeId;

use base::*;


#[unsafe(no_mangle)]
pub static __OBJECT_TYPE_ID: TypeId = OBJECT_TYPE_ID;

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
            .with(TestingObject::new(texture_id), 0.0)
            .with(TestingObject::new(texture_id), 0.0),
    )
}



struct TestingObject {
    color: Rgba,
    font_size: f32,
    texture_id: u64,
}

impl TestingObject {
    fn new(texture_id: u64) -> Self {
        Self {
            color: Rgba::rgb(0x73, 0x73, 0x89),
            font_size: 30.0,
            texture_id,
        }
    }
}

impl Object for TestingObject {
    fn render(&self, pass: &mut RenderPass<'_>, renderer: &mut dyn Renderer) {
        // renderer.quad(pass.position(), pass.size(), Rgba::BLACK);
        renderer.image(self.texture_id, pass.position(), pass.size());
        renderer.text("EXAMPLE", pass.position(), self.font_size, self.color);
    }

    fn measure(
        &mut self,
        pass: &mut MeasurePass<'_>,
        axis: Axis,
        _length_request: LengthRequest,
        _cross_length: Option<f32>,
    ) -> f32 {
        pass.measure_context()
            .text_size("EXAMPLE", self.font_size)
            .value_for_axis(axis)
    }

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::IBeam
    }

    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down {
                button: PointerButton::Primary,
            } => {
                pass.capture_pointer();
                pass.set_handled();
            }
            PointerEvent::Up {
                button: PointerButton::Primary,
            } => {
                pass.request_focus();
            }
            _ => {}
        }
    }

    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        if hovered {
            self.color = Rgba::rgb(0xaa, 0xaa, 0xad);
        } else {
            self.color = Rgba::rgb(0x73, 0x73, 0x89);
        }
        pass.set_handled();
    }

    fn on_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {
        if focused {
            self.font_size *= 1.2;
        } else {
            self.font_size /= 1.2;
        }
        pass.set_handled();
        pass.request_layout();
    }
}
