//! Native application shell. Trace data and viewing calculations live elsewhere.
mod browser;
mod fonts;
mod waves;

use crate::viewer::{
    Session,
    format::{Radix, format_time},
    viewport::Viewport,
};
use eframe::egui::{self, Color32, RichText};
use std::{
    ffi::OsString,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

pub const USAGE: &str = "Usage: oculi-gui [path.vcd]\n\nOpen a completed VCD simulation in the waveform viewer.\n  -h, --help    Show this help";

#[derive(Debug, PartialEq, Eq)]
pub enum Startup {
    Help,
    Open(Option<PathBuf>),
}

impl Startup {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let first = args.next();
        if args.next().is_some() {
            return Err(USAGE.into());
        }
        match first {
            Some(arg) if arg == "--help" || arg == "-h" => Ok(Self::Help),
            Some(path) => Ok(Self::Open(Some(path.into()))),
            None => Ok(Self::Open(None)),
        }
    }
}

pub fn load_path(path: &Path) -> Result<Session, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let trace =
        crate::vcd::load(BufReader::new(file)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Session::new(trace))
}

#[derive(Default)]
pub struct AppState {
    pub session: Option<Session>,
    pub path: Option<PathBuf>,
    pub error: Option<String>,
}

impl AppState {
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused() {
            return;
        }
        let Some(session) = &mut self.session else {
            return;
        };
        let (zoom_in, zoom_out, fit) = ctx.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::NONE, egui::Key::Plus)
                    || input.consume_key(egui::Modifiers::SHIFT, egui::Key::Plus)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::Equals),
                input.consume_key(egui::Modifiers::NONE, egui::Key::Minus),
                input.consume_key(egui::Modifiers::NONE, egui::Key::F),
            )
        });
        if zoom_in {
            zoom(session, 0.5);
        }
        if zoom_out {
            zoom(session, 2.0);
        }
        if fit {
            session.viewport = Viewport::fit(session.trace.end_time);
        }
    }

    /// Commit a replacement only after the complete file has loaded successfully.
    pub fn open(&mut self, path: &Path) {
        match load_path(path) {
            Ok(session) => {
                self.session = Some(session);
                self.path = Some(path.into());
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.shortcuts(ui.ctx());
        let dark = ui.visuals().dark_mode;
        let green = if dark {
            Color32::from_rgb(116, 216, 166)
        } else {
            Color32::from_rgb(25, 118, 71)
        };
        let mut open = false;
        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../../assets/oculi-logo.svg"))
                        .fit_to_exact_size(egui::vec2(29.0, 22.0))
                        .tint(if dark {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(80, 140, 100)
                        }),
                );
                ui.label(RichText::new("oculi").size(22.0).strong());
                ui.separator();
                if ui.button("Open VCD…").clicked() {
                    open = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if dark { "Light theme" } else { "Dark theme" })
                        .clicked()
                    {
                        ui.ctx().set_theme(if dark {
                            egui::Theme::Light
                        } else {
                            egui::Theme::Dark
                        });
                    }
                    ui.weak(
                        self.path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "No trace loaded".into()),
                    );
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if let Some(session) = &mut self.session {
                    if ui.button("Fit trace (F)").clicked() {
                        session.viewport = Viewport::fit(session.trace.end_time);
                    }
                    if ui
                        .button("Zoom out (−)")
                        .on_hover_text("Show more time")
                        .clicked()
                    {
                        zoom(session, 2.0);
                    }
                    if ui
                        .button("Zoom in (+)")
                        .on_hover_text("Inspect a smaller time range")
                        .clicked()
                    {
                        zoom(session, 0.5);
                    }
                    ui.separator();
                    ui.selectable_value(&mut session.radix, Radix::Binary, "Binary");
                    ui.selectable_value(&mut session.radix, Radix::Hex, "Hex");
                    ui.separator();
                    ui.colored_label(
                        green,
                        session
                            .cursor
                            .map(|t| {
                                format!("Cursor A: {}", format_time(t, session.trace.timescale))
                            })
                            .unwrap_or_else(|| "Click a waveform to place cursor A".into()),
                    );
                } else {
                    ui.weak("Completed simulation viewer");
                }
            });
            ui.add_space(4.0);
            if let Some(error) = &self.error {
                let message = error.clone();
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, message);
                    if ui.small_button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
            }
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(session) = &self.session {
                    ui.colored_label(green, "● Trace loaded");
                    ui.separator();
                    ui.weak(format!(
                        "Recorded: 0 – {}",
                        format_time(session.trace.end_time, session.trace.timescale)
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak("Drag to pan · scroll to zoom");
                    });
                } else {
                    ui.weak("Open a VCD file to begin");
                }
            });
        });
        egui::Panel::left("browser")
            .resizable(true)
            .default_size(242.0)
            .min_size(170.0)
            .show(ui, |ui| {
                if let Some(session) = &mut self.session {
                    browser::show(ui, session);
                } else {
                    ui.heading("Design");
                    ui.weak("No signals loaded");
                }
            });
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(session) = &mut self.session {
                ui.horizontal(|ui| {
                    ui.colored_label(green, "●");
                    ui.strong(
                        self.path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    );
                });
                ui.separator();
                waves::show(ui, session);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("Explore your simulation");
                    ui.label("Open a completed VCD trace to inspect signals and values over time.");
                    ui.add_space(14.0);
                    if ui.button("Open VCD…").clicked() {
                        open = true;
                    }
                });
            }
        });
        if open
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("VCD trace", &["vcd"])
                .pick_file()
        {
            self.open(&path);
        }
    }
}

fn zoom(session: &mut Session, factor: f64) {
    let anchor = session
        .cursor
        .filter(|&c| c >= session.viewport.start && c <= session.viewport.end)
        .unwrap_or_else(|| session.viewport.tick_at(0.5));
    session
        .viewport
        .zoom(factor, anchor, session.trace.end_time);
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

fn configure_context(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    fonts::configure(ctx);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
    });
}

pub fn run(path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Oculi",
        options,
        Box::new(move |cc| {
            configure_context(&cc.egui_ctx);
            let mut state = AppState::default();
            if let Some(path) = path {
                state.open(&path);
            }
            Ok(Box::new(state))
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_fields_do_not_expand_the_sidebar_over_successive_frames() {
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            for (screen, sidebar_width) in [
                (egui::vec2(760.0, 480.0), 170.0),
                (egui::vec2(1280.0, 800.0), 242.0),
                (egui::vec2(1280.0, 800.0), 360.0),
            ] {
                let ctx = egui::Context::default();
                configure_context(&ctx);
                ctx.set_theme(theme);
                let mut app = AppState::default();
                app.open(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build/counter.vcd"));
                assert!(app.error.is_none());
                let session = app.session.as_mut().unwrap();
                for index in session.selected.clone() {
                    session.set_bus_expanded(index, true);
                }
                for frame in 0..60 {
                    let session = app.session.as_mut().unwrap();
                    session.hierarchy_query =
                        if (20..40).contains(&frame) { "td" } else { "" }.into();
                    session.filter = if (20..40).contains(&frame) { "ct" } else { "" }.into();
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen)),
                            ..Default::default()
                        },
                        |ui| {
                            let id = ui.make_persistent_id("browser");
                            if frame == 0 {
                                ctx.data_mut(|data| {
                                    data.insert_persisted(
                                        id,
                                        egui::containers::panel::PanelState {
                                            outer_rect: egui::Rect::from_min_size(
                                                egui::Pos2::ZERO,
                                                egui::vec2(sidebar_width, screen.y),
                                            ),
                                        },
                                    );
                                });
                            }
                            app.show(ui);
                            let width = egui::containers::panel::PanelState::load(&ctx, id)
                                .expect("browser panel state")
                                .size()
                                .x;
                            assert!(
                                (width - sidebar_width).abs() <= 1.0,
                                "sidebar grew from {sidebar_width} to {width} on frame {frame} ({theme:?})"
                            );
                        },
                    );
                    output.textures_delta.clear();
                }
            }
        }
    }
}
