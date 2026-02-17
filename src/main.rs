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
    std::sync::{Arc, atomic::AtomicBool},
};


const WORKSPACE_DIR: &str = env!("CARGO_MANIFEST_DIR");
const EXAMPLE_SRC: &str = include_str!("../example/src/example.rs");

fn main() -> Result<()> {
    eframe::run_native(
        "Demo",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|_cc| {
            Ok(Box::new(App {
                program: Program::load("example", EXAMPLE_SRC.to_string())?,
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
    known_size: Size,
    known_position: Point,
}

impl Program {
    fn load(name: &'static str, source: String) -> Result<Self> {
        let mut this = Self {
            name,
            handle: None,
            editing: false,
            waiting_on_recompile: false,
            compiling: Arc::new(AtomicBool::new(false)),
            latest_compile_succeeded: Arc::new(AtomicBool::new(true)),
            source,
            known_size: Size::ZERO,
            known_position: Point::ZERO,
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
        let view_fn =
            unsafe { handle.get::<unsafe extern "Rust" fn() -> Box<dyn Object>>(b"view") }?;
        let root_object = unsafe { (&*view_fn)() };

        let tree = ObjectTree::new(root_object);

        self.handle = Some(ProgramHandle {
            tree,
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

        egui::Frame::group(ui.style()).show(ui, |ui| {
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
                ui.add(
                    egui::TextEdit::multiline(&mut self.source)
                        .code_editor()
                        .font(egui::FontId::monospace(20.0))
                        .desired_width(ui.available_width()),
                );
                return;
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
                return;
            }

            let handle = self.handle.as_mut().unwrap();
            let tree = &mut handle.tree;
            let mut renderer = RendererImpl {
                position: self.known_position,
                painter: ui.painter(),
            };

            // TODO: Pass input events to object tree.
            for event in ui.input(|i| {
                i.filtered_events(&egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                })
            }) {
                match event {
                    _ => {}
                }
            }

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

            render_pass(tree, &mut renderer);
        });

        Ok(())
    }
}

struct ProgramHandle {
    tree: ObjectTree,
    _handle: libloading::Library,
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

#[inline(always)]
const fn convert_vec2_to_size(vec2: egui::Vec2) -> Size {
    Size::new(vec2.x, vec2.y)
}

#[inline(always)]
const fn convert_pos2_to_point(pos2: egui::Pos2) -> Point {
    Point::new(pos2.x, pos2.y)
}
