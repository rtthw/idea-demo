//! # Demo Binary

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_feature;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

mod compiler;

use {
    anyhow::Result,
    base::*,
    eframe::egui,
    std::{
        any::TypeId,
        collections::HashMap,
        sync::{Arc, atomic::AtomicBool},
    },
};


const WORKSPACE_DIR: &str = env!("CARGO_MANIFEST_DIR");
const EXAMPLE_SRC: &str = include_str!("../example/src/example.rs");

fn main() -> Result<()> {
    eframe::run_native(
        "Demo",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder {
                inner_size: Some(egui::vec2(1200.0, 800.0)),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| {
            Ok(Box::new(App {
                program: Program::load("example", EXAMPLE_SRC.to_string(), cc.egui_ctx.clone())?,
            }))
        }),
    )
    .map_err(|err| anyhow::anyhow!("{err}"))?;

    Ok(())
}



struct App {
    program: Program,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.program.update(ui).expect("failed to update program");
                });
        });
    }
}



struct Program {
    name: &'static str,
    handle: Option<ProgramHandle>,
    editing: bool,
    waiting_on_recompile: bool,
    compiling: Arc<AtomicBool>,
    latest_compile_succeeded: Arc<AtomicBool>,
    source: String,
    egui_context: egui::Context,
    known_size: Size,
    known_position: Point,
    known_pointer_position: Option<Point>,
}

impl Program {
    fn load(name: &'static str, source: String, egui_context: egui::Context) -> Result<Self> {
        let mut this = Self {
            name,
            handle: None,
            editing: false,
            waiting_on_recompile: false,
            compiling: Arc::new(AtomicBool::new(false)),
            latest_compile_succeeded: Arc::new(AtomicBool::new(true)),
            source,
            egui_context,
            known_size: Size::ZERO,
            known_position: Point::ZERO,
            known_pointer_position: None,
        };

        this.start_compiling();

        Ok(this)
    }

    fn start_compiling(&mut self) {
        self.waiting_on_recompile = true;
        self.compiling
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let compiling = self.compiling.clone();
        let latest_compile_succeeded = self.latest_compile_succeeded.clone();
        let content = self.source.clone();
        let input_filename = format!("{}.rs", self.name);
        let output_filename = format!("{}.so", self.name);

        std::thread::spawn(move || {
            let result = compiler::run(&content, &input_filename, &output_filename);
            if let Err(error) = &result {
                println!("ERROR: {error}");
            }
            latest_compile_succeeded.swap(result.is_ok(), std::sync::atomic::Ordering::SeqCst);
            compiling.swap(false, std::sync::atomic::Ordering::SeqCst);
        });
    }

    fn reload(&mut self) -> Result<()> {
        // We need to drop the previous shared object before reloading because `dlopen`
        // won't load the new version if there are existing references to the old one.
        drop(self.handle.take());

        let handle = unsafe {
            libloading::Library::new(
                format!("{WORKSPACE_DIR}/target/debug/{}.so", self.name).as_str(),
            )?
        };

        let object_type_id = unsafe { handle.get::<*const TypeId>(b"__OBJECT_TYPE_ID")? };
        assert_eq!(unsafe { **object_type_id }, TypeId::of::<dyn Object>());

        let mut textures = HashMap::new();
        let view_fn = unsafe {
            handle.get::<unsafe extern "Rust" fn(&mut dyn ViewContext) -> Box<dyn Object>>(b"view")
        }?;
        let root_object = unsafe {
            (&*view_fn)(&mut ViewContextImpl {
                egui_context: &self.egui_context,
                textures: &mut textures,
            })
        };

        let tree = ObjectTree::new(root_object);

        self.handle = Some(ProgramHandle {
            tree,
            _textures: textures,
            _handle: handle,
        });

        Ok(())
    }

    fn update(&mut self, ui: &mut egui::Ui) -> Result<()> {
        if self.compiling.load(std::sync::atomic::Ordering::Relaxed) {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
            return Ok(());
        }

        let compile_success = self
            .latest_compile_succeeded
            .load(std::sync::atomic::Ordering::Relaxed);

        let recompiled = self.waiting_on_recompile && compile_success;

        if self.waiting_on_recompile && compile_success {
            self.waiting_on_recompile = false;
            self.reload()?;
        }

        ui.set_width(ui.available_width());
        ui.set_height(ui.available_height());

        if self.editing {
            ui.allocate_ui_with_layout(
                egui::vec2(
                    ui.available_size_before_wrap().x,
                    ui.spacing().interact_size.y,
                ),
                egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    if ui.button("Confirm").clicked() {
                        self.editing = false;
                        self.start_compiling();
                    }
                    if ui.button("Cancel").clicked() {
                        self.editing = false;
                    }
                },
            );
            ui.separator();

            let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                let mut layout_job: egui::text::LayoutJob =
                    egui_extras::syntax_highlighting::highlight(
                        ui.ctx(),
                        ui.style(),
                        &egui_extras::syntax_highlighting::CodeTheme::dark(16.0),
                        text.as_str(),
                        "rs",
                    );
                layout_job.wrap.max_width = wrap_width;

                ui.fonts_mut(|f| f.layout_job(layout_job))
            };
            ui.add(
                egui::TextEdit::multiline(&mut self.source)
                    .layouter(&mut layouter)
                    .desired_width(ui.available_width()),
            );

            return Ok(());
        }

        ui.allocate_ui_with_layout(
            egui::vec2(
                ui.available_size_before_wrap().x,
                ui.spacing().interact_size.y,
            ),
            egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true),
            |ui| {
                if ui.button("Edit").clicked() {
                    self.editing = true;
                }
            },
        );
        ui.separator();
        if !compile_success {
            ui.centered_and_justified(|ui| {
                ui.heading("Compilation failed, see logs");
            });
            return Ok(());
        }

        let handle = self.handle.as_mut().unwrap();
        let tree = &mut handle.tree;
        let mut renderer = RendererImpl {
            position: self.known_position,
            painter: ui.painter(),
        };

        let window_rect = ui.available_rect_before_wrap();
        let window_size = convert_vec2_to_size(window_rect.size());
        self.known_position = convert_pos2_to_point(window_rect.min);
        if self.known_size != window_size || recompiled {
            self.known_size = window_size;
            tree.resize(
                window_size,
                &mut MeasureContextImpl {
                    egui_context: ui.ctx(),
                },
            );
        }

        for event in ui.input(|i| {
            i.filtered_events(&egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            })
        }) {
            match event {
                egui::Event::PointerMoved(pos) => {
                    let position = if window_rect.contains(pos) {
                        Some(convert_pos2_to_point(pos) - self.known_position)
                    } else {
                        None
                    };
                    if self.known_pointer_position == position {
                        continue;
                    }
                    self.known_pointer_position = position;
                    tree.handle_pointer_event(
                        PointerEvent::Move { position },
                        &mut MeasureContextImpl {
                            egui_context: ui.ctx(),
                        },
                    );
                }
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    ..
                } => {
                    if !window_rect.contains(pos) {
                        continue;
                    }
                    let button = match button {
                        egui::PointerButton::Primary => PointerButton::Primary,
                        egui::PointerButton::Secondary => PointerButton::Secondary,
                        egui::PointerButton::Middle => PointerButton::Auxiliary,
                        egui::PointerButton::Extra1 => PointerButton::Back,
                        egui::PointerButton::Extra2 => PointerButton::Forward,
                    };
                    let event = if pressed {
                        PointerEvent::Down { button }
                    } else {
                        PointerEvent::Up { button }
                    };
                    tree.handle_pointer_event(
                        event,
                        &mut MeasureContextImpl {
                            egui_context: ui.ctx(),
                        },
                    );
                }
                _ => {}
            }
        }

        if ui.ui_contains_pointer() {
            ui.ctx()
                .set_cursor_icon(convert_cursor_icon(tree.cursor_icon()));
        }

        render_pass(tree, &mut renderer);

        Ok(())
    }
}

struct ProgramHandle {
    tree: ObjectTree,
    _textures: HashMap<String, egui::TextureHandle>,
    _handle: libloading::Library,
}



struct ViewContextImpl<'pass> {
    textures: &'pass mut HashMap<String, egui::TextureHandle>,
    egui_context: &'pass egui::Context,
}

impl ViewContext for ViewContextImpl<'_> {
    fn load_texture(&mut self, path: &str) -> u64 {
        let egui::TextureId::Managed(id) = self
            .textures
            .entry(path.to_string())
            .or_insert_with(|| {
                let image = image::ImageReader::open(path).unwrap().decode().unwrap();
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();

                self.egui_context.load_texture(
                    path,
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id()
        else {
            unreachable!("load_texture should only produce managed IDs")
        };

        id
    }
}

struct RendererImpl<'pass> {
    position: Point,
    painter: &'pass egui::Painter,
}

impl Renderer for RendererImpl<'_> {
    fn text(&mut self, content: &str, position: Point, font_size: f32, color: Rgba) {
        self.painter.text(
            convert_point(self.position + position),
            egui::Align2::LEFT_TOP,
            content,
            egui::FontId::proportional(font_size),
            convert_color(color),
        );
    }

    fn quad(&mut self, position: Point, size: Size, color: Rgba) {
        self.painter.rect(
            egui::Rect::from_min_size(convert_point(self.position + position), convert_size(size)),
            0,
            convert_color(color),
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }

    fn image(&mut self, texture_id: u64, position: Point, size: Size) {
        self.painter.image(
            egui::TextureId::Managed(texture_id),
            egui::Rect::from_min_size(convert_point(self.position + position), convert_size(size)),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}

struct MeasureContextImpl<'pass> {
    egui_context: &'pass egui::Context,
}

impl MeasureContext for MeasureContextImpl<'_> {
    fn text_size(&mut self, content: &str, font_size: f32) -> Size {
        convert_vec2_to_size(
            self.egui_context
                .fonts_mut(|f| {
                    f.layout(
                        content.to_string(),
                        egui::FontId::proportional(font_size),
                        egui::Color32::WHITE,
                        f32::INFINITY,
                    )
                })
                .rect
                .size(),
        )
    }
}



#[inline(always)]
const fn convert_point(point: Point) -> egui::Pos2 {
    egui::Pos2 {
        x: point.x,
        y: point.y,
    }
}

#[inline(always)]
const fn convert_size(size: Size) -> egui::Vec2 {
    egui::Vec2 {
        x: size.width,
        y: size.height,
    }
}

#[inline(always)]
const fn convert_color(rgba: Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(rgba.r, rgba.g, rgba.b, rgba.a)
}

const fn convert_cursor_icon(icon: CursorIcon) -> egui::CursorIcon {
    match icon {
        CursorIcon::PointingHand => egui::CursorIcon::PointingHand,
        CursorIcon::IBeam => egui::CursorIcon::Text,
        _ => egui::CursorIcon::Default,
    }
}

#[inline(always)]
const fn convert_vec2_to_size(vec2: egui::Vec2) -> Size {
    Size::new(vec2.x, vec2.y)
}

#[inline(always)]
const fn convert_pos2_to_point(pos2: egui::Pos2) -> Point {
    Point::new(pos2.x, pos2.y)
}
