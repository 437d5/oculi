//! Digital geometry is painted in shared rows, so labels and traces scroll together.
use crate::{
    trace::Logic,
    viewer::{
        Session,
        format::{format_time, format_value},
        segments,
    },
};
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke};

const ROW_HEIGHT: f32 = 64.0;

#[derive(Clone, Copy)]
struct RowDrag {
    signal: usize,
}

struct Colors {
    wave: Color32,
    cursor: Color32,
    unknown: Color32,
    z: Color32,
    grid: Color32,
    text: Color32,
    muted: Color32,
}
impl Colors {
    fn new(ui: &egui::Ui) -> Self {
        let dark = ui.visuals().dark_mode;
        Self {
            wave: if dark {
                Color32::from_rgb(116, 216, 166)
            } else {
                Color32::from_rgb(25, 118, 71)
            },
            cursor: if dark {
                Color32::from_rgb(233, 191, 113)
            } else {
                Color32::from_rgb(137, 92, 11)
            },
            unknown: if dark {
                Color32::from_rgb(245, 139, 139)
            } else {
                Color32::from_rgb(175, 45, 45)
            },
            z: if dark {
                Color32::from_rgb(150, 184, 248)
            } else {
                Color32::from_rgb(53, 87, 160)
            },
            grid: ui.visuals().widgets.noninteractive.bg_stroke.color,
            text: ui.visuals().text_color(),
            muted: ui.visuals().weak_text_color(),
        }
    }
}

pub fn show(ui: &mut egui::Ui, session: &mut Session) {
    let colors = Colors::new(ui);
    let names_width = (ui.available_width() * 0.30).clamp(160.0, 250.0);
    let available = ui.available_rect_before_wrap();
    let navigation_area =
        Rect::from_min_max(available.min + egui::vec2(names_width, 0.0), available.max);
    // Apply input once before drawing the ruler or any rows. All geometry in
    // this frame then uses the same viewport and cursor.
    navigation(ui, session, navigation_area, "wave_workspace");
    let (header, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), Sense::hover());
    let timeline = Rect::from_min_max(
        Pos2::new(header.left() + names_width, header.top()),
        header.max,
    );
    let painter = ui.painter_at(header);
    painter.text(
        header.left_center() + egui::vec2(8.0, 0.0),
        Align2::LEFT_CENTER,
        "Signal",
        FontId::proportional(12.0),
        colors.muted,
    );
    painter.text(
        Pos2::new(timeline.left() - 12.0, header.center().y),
        Align2::RIGHT_CENTER,
        "Value at A",
        FontId::proportional(11.0),
        colors.muted,
    );
    let ruler_font = FontId::monospace(12.0);
    let longest = format_time(session.viewport.end, session.trace.timescale);
    let label_width = painter
        .layout_no_wrap(longest, ruler_font.clone(), colors.muted)
        .size()
        .x;
    let max_labels = ((timeline.width() / (label_width + 24.0)) as usize).max(2);
    let ticks =
        crate::viewer::ruler::ticks(session.viewport.start, session.viewport.end, max_labels);
    let mut label_right = f32::NEG_INFINITY;
    for &tick in &ticks {
        let x = timeline.left() + timeline.width() * session.viewport.fraction_at(tick) as f32;
        let galley = painter.layout_no_wrap(
            format_time(tick, session.trace.timescale),
            ruler_font.clone(),
            colors.muted,
        );
        let width = galley.size().x;
        if width > timeline.width() {
            continue;
        }
        let left = (x - width / 2.0).clamp(timeline.left(), timeline.right() - width);
        if left < label_right + 12.0 {
            continue;
        }
        painter.galley(
            Pos2::new(left, header.center().y - galley.size().y / 2.0),
            galley,
            colors.muted,
        );
        label_right = left + width;
    }
    if session.selected.is_empty() {
        ui.add_space(35.0);
        ui.weak("Select signals in the left sidebar to display their waveforms.");
    }
    let mut move_row = None;
    let row_count = session.selected.len();
    egui::ScrollArea::vertical()
        .id_salt("wave_rows")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            // Indices are tiny; histories and bit values are always borrowed.
            for (position, index) in session.selected.clone().into_iter().enumerate() {
                let (row, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ROW_HEIGHT),
                    Sense::hover(),
                );
                let area =
                    Rect::from_min_max(Pos2::new(row.left() + names_width, row.top()), row.max);
                let painter = ui.painter_at(row);
                let signal = &session.trace.signals[index];
                let history = &session.trace.histories[signal.history];
                painter.line_segment(
                    [row.left_bottom(), row.right_bottom()],
                    Stroke::new(1.0, colors.grid),
                );
                painter.line_segment(
                    [area.left_top(), area.left_bottom()],
                    Stroke::new(1.0, colors.grid),
                );
                let name_rect =
                    Rect::from_min_max(row.min, Pos2::new(area.left() - 80.0, row.bottom()));
                let label = signal.path.join(".");
                painter.with_clip_rect(name_rect).text(
                    name_rect.left_center() + egui::vec2(34.0, 0.0),
                    Align2::LEFT_CENTER,
                    &label,
                    FontId::monospace(14.0),
                    colors.text,
                );
                let label_response = ui
                    .interact(
                        name_rect,
                        ui.scope_id().with(("label", index)),
                        Sense::click(),
                    )
                    .on_hover_text(&label);
                label_response.context_menu(|ui| {
                    if ui
                        .add_enabled(position > 0, egui::Button::new("Move up"))
                        .clicked()
                    {
                        move_row = Some((position, position - 1));
                        ui.close();
                    }
                    if ui
                        .add_enabled(position + 1 < row_count, egui::Button::new("Move down"))
                        .clicked()
                    {
                        move_row = Some((position, position + 1));
                        ui.close();
                    }
                });
                if let Some(payload) = label_response.dnd_hover_payload::<RowDrag>()
                    && payload.signal != index
                    && let Some(from) = session.selected.iter().position(|&i| i == payload.signal)
                {
                    painter.line_segment(
                        insertion_marker(name_rect, from, position),
                        Stroke::new(2.0, colors.wave),
                    );
                }
                if let Some(payload) = label_response.dnd_release_payload::<RowDrag>()
                    && let Some(from) = session.selected.iter().position(|&i| i == payload.signal)
                {
                    move_row = Some((from, position));
                }
                let handle_rect = Rect::from_center_size(
                    Pos2::new(row.left() + 18.0, row.center().y),
                    egui::vec2(24.0, ROW_HEIGHT - 12.0),
                );
                let handle = ui
                    .interact(
                        handle_rect,
                        ui.scope_id().with(("row_handle", index)),
                        Sense::drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::Grab)
                    .on_hover_text("Drag to reorder signal");
                handle.dnd_set_drag_payload(RowDrag { signal: index });
                for dx in [-2.5, 2.5] {
                    for dy in [-5.0, 0.0, 5.0] {
                        painter.circle_filled(
                            handle_rect.center() + egui::vec2(dx, dy),
                            1.0,
                            colors.muted,
                        );
                    }
                }
                let value = session.cursor.and_then(|time| history.value_at(time));
                let full = value
                    .map(|v| format_value(v, session.radix))
                    .unwrap_or_else(|| "—".into());
                let short = if full.len() > 8 {
                    format!("{}…", &full[..7])
                } else {
                    full.clone()
                };
                painter.text(
                    Pos2::new(area.left() - 12.0, row.center().y),
                    Align2::RIGHT_CENTER,
                    short,
                    FontId::monospace(14.0),
                    colors.cursor,
                );
                let value_rect = Rect::from_min_max(
                    Pos2::new(area.left() - 78.0, row.top()),
                    Pos2::new(area.left(), row.bottom()),
                );
                ui.interact(
                    value_rect,
                    ui.scope_id().with(("value", index)),
                    Sense::hover(),
                )
                .on_hover_text(full);
                let canvas = painter.with_clip_rect(area.shrink2(egui::vec2(0.0, 1.0)));
                for &tick in &ticks {
                    let x = area.left() + area.width() * session.viewport.fraction_at(tick) as f32;
                    canvas.line_segment(
                        [Pos2::new(x, area.top()), Pos2::new(x, area.bottom())],
                        Stroke::new(0.5, colors.grid),
                    );
                }
                let end = session.viewport.end.min(session.trace.end_time);
                for segment in segments::visible(history, session.viewport.start, end) {
                    let left = area.left()
                        + area.width() * session.viewport.fraction_at(segment.start) as f32;
                    let right = area.left()
                        + area.width() * session.viewport.fraction_at(segment.end) as f32;
                    let piece = Rect::from_min_max(
                        Pos2::new(left, area.top() + 17.0),
                        Pos2::new(right, area.bottom() - 17.0),
                    );
                    match segment.value {
                        Some(bits) if bits.len() == 1 => {
                            let previous = segment
                                .start
                                .checked_sub(1)
                                .and_then(|time| history.value_at(time))
                                .and_then(|value| value.first())
                                .copied();
                            scalar(&canvas, piece, bits[0], previous, &colors);
                        }
                        Some(bits) => {
                            let text = format_value(bits, session.radix);
                            bus(&canvas, piece, &text, &colors);
                            ui.interact(
                                piece,
                                ui.scope_id().with(("bus", index, segment.start)),
                                Sense::hover(),
                            )
                            .on_hover_text(text);
                        }
                        None => {
                            canvas.line_segment(
                                [piece.left_center(), piece.right_center()],
                                Stroke::new(1.0, colors.muted),
                            );
                            if piece.width() > 90.0 {
                                canvas.text(
                                    piece.center(),
                                    Align2::CENTER_CENTER,
                                    "unavailable",
                                    FontId::monospace(12.0),
                                    colors.muted,
                                );
                            }
                        }
                    }
                }
                if session.trace.end_time == 0 {
                    canvas.text(
                        area.center(),
                        Align2::CENTER_CENTER,
                        "No recorded duration",
                        FontId::proportional(12.0),
                        colors.muted,
                    );
                }
                if let Some(time) = session.cursor
                    && time >= session.viewport.start
                    && time <= session.viewport.end
                {
                    let x = area.left() + area.width() * session.viewport.fraction_at(time) as f32;
                    canvas.line_segment(
                        [Pos2::new(x, area.top()), Pos2::new(x, area.bottom())],
                        Stroke::new(1.0, colors.cursor),
                    );
                }
            }
        });
    if let Some((from, to)) = move_row {
        session.move_selected(from, to);
    }
}

fn insertion_marker(row: Rect, from: usize, to: usize) -> [Pos2; 2] {
    if from > to {
        [row.left_top(), row.right_top()]
    } else {
        [row.left_bottom(), row.right_bottom()]
    }
}

fn navigation(
    ui: &mut egui::Ui,
    session: &mut Session,
    area: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
) {
    if area.width() <= 0.0 {
        return;
    }
    let response = ui.interact(
        area,
        ui.scope_id().with(("navigation", id)),
        Sense::click_and_drag(),
    );
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        session.cursor = Some(
            session
                .viewport
                .tick_at(f64::from((pos.x - area.left()) / area.width()))
                .min(session.trace.end_time),
        );
    }
    if response.dragged() {
        let origin_id = response.id.with("drag_origin");
        if response.drag_started() {
            ui.data_mut(|data| data.insert_temp(origin_id, session.viewport));
        }
        let origin = ui
            .data(|data| data.get_temp::<crate::viewer::viewport::Viewport>(origin_id))
            .unwrap_or(session.viewport);
        let ticks = -(f64::from(response.total_drag_delta().unwrap_or_default().x)
            / f64::from(area.width())
            * origin.span() as f64)
            .round() as i128;
        session.viewport = origin;
        session.viewport.pan(ticks, session.trace.end_time);
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0
            && let Some(pos) = response.hover_pos()
        {
            let anchor = session
                .viewport
                .tick_at(f64::from((pos.x - area.left()) / area.width()));
            session.viewport.zoom(
                (-f64::from(scroll) * 0.005).exp(),
                anchor,
                session.trace.end_time,
            );
            // Prevent the enclosing row scroll area from reusing this gesture.
            ui.input_mut(|input| {
                input.smooth_scroll_delta.y = 0.0;
            });
        }
    }
}

fn scalar(p: &Painter, r: Rect, bit: Logic, previous: Option<Logic>, c: &Colors) {
    if bit == Logic::One {
        p.rect_filled(r, 0.0, c.wave.gamma_multiply(0.15));
    }
    let y = match bit {
        Logic::Zero => r.bottom(),
        Logic::One => r.top(),
        _ => r.center().y,
    };
    let color = match bit {
        Logic::X => c.unknown,
        Logic::Z => c.z,
        _ => c.wave,
    };
    p.line_segment(
        [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
        Stroke::new(1.5, color),
    );
    // Join real adjacent values, not an imagined level before the initial sample.
    if let Some(previous) = previous.filter(|&value| value != bit) {
        let from = match previous {
            Logic::Zero => r.bottom(),
            Logic::One => r.top(),
            _ => r.center().y,
        };
        p.line_segment(
            [Pos2::new(r.left(), from), Pos2::new(r.left(), y)],
            Stroke::new(1.0, color),
        );
    }
    if matches!(bit, Logic::X | Logic::Z) && r.width() > 14.0 {
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            if bit == Logic::X { "X" } else { "Z" },
            FontId::monospace(13.0),
            color,
        );
    }
}

fn bus(p: &Painter, r: Rect, text: &str, c: &Colors) {
    let cut = 4.0f32.min(r.width() / 3.0);
    let points = vec![
        r.left_center(),
        r.left_top() + egui::vec2(cut, 0.0),
        r.right_top() - egui::vec2(cut, 0.0),
        r.right_center(),
        r.right_bottom() - egui::vec2(cut, 0.0),
        r.left_bottom() + egui::vec2(cut, 0.0),
        r.left_center(),
    ];
    p.add(egui::Shape::line(points, Stroke::new(1.0, c.wave)));
    let max_chars = ((r.width() - 14.0) / 7.0).max(0.0) as usize;
    if max_chars >= 2 {
        let label = if text.len() > max_chars {
            format!("{}…", &text[..max_chars - 1])
        } else {
            text.to_owned()
        };
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::monospace(13.0),
            c.wave,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_marker_matches_the_result_of_reordering() {
        let row = Rect::from_min_max(Pos2::new(0.0, 64.0), Pos2::new(250.0, 128.0));
        assert_eq!(
            insertion_marker(row, 2, 0),
            [row.left_top(), row.right_top()],
            "upward drop inserts before target"
        );
        assert_eq!(
            insertion_marker(row, 0, 2),
            [row.left_bottom(), row.right_bottom()],
            "downward drop inserts after target"
        );
    }

    #[test]
    fn dragging_a_name_handle_reorders_rows_but_cancel_does_not() {
        let context = egui::Context::default();
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        let original = session.selected.clone();
        frame(&context, &mut session, vec![]);
        let from = Pos2::new(18.0, 75.0);
        let target = Pos2::new(18.0, 138.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &context,
            &mut session,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        frame(
            &context,
            &mut session,
            vec![egui::Event::PointerMoved(target)],
        );
        frame(&context, &mut session, vec![button(target, false)]);
        assert_eq!(
            session.selected,
            [original[1], original[0]],
            "dragging a handle onto another row should reorder"
        );
        frame(
            &context,
            &mut session,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        let outside = Pos2::new(-20.0, -20.0);
        frame(
            &context,
            &mut session,
            vec![egui::Event::PointerMoved(outside)],
        );
        frame(&context, &mut session, vec![button(outside, false)]);
        assert_eq!(
            session.selected,
            [original[1], original[0]],
            "release outside rows must cancel"
        );
    }

    #[test]
    fn slow_drag_accumulates_sub_tick_movements() {
        let context = egui::Context::default();
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        session.viewport = crate::viewer::viewport::Viewport { start: 10, end: 20 };
        frame(&context, &mut session, vec![]);
        let start = Pos2::new(750.0, 75.0);
        frame(
            &context,
            &mut session,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        for n in 1..=30 {
            frame(
                &context,
                &mut session,
                vec![egui::Event::PointerMoved(
                    start + egui::vec2(n as f32 * 10.0, 0.0),
                )],
            );
        }
        assert_eq!(
            session.viewport.start, 7,
            "300 px across a 1030 px, ten-tick window should pan three ticks"
        );
    }

    #[test]
    fn waveform_zoom_consumes_the_vertical_wheel_delta() {
        let context = egui::Context::default();
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        session.viewport = crate::viewer::viewport::Viewport { start: 5, end: 15 };
        let point = Pos2::new(500.0, 75.0);
        let area = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let mut remainder = 0.0;
        for wheel in [false, true] {
            let mut events = vec![egui::Event::PointerMoved(point)];
            if wheel {
                events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    delta: egui::vec2(0.0, -80.0),
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut output = context.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    navigation(ui, &mut session, area, "wheel_probe");
                    remainder = ui.input(|input| input.smooth_scroll_delta.y);
                },
            );
            output.textures_delta.clear();
        }
        assert_eq!(
            remainder, 0.0,
            "the enclosing row scroll area must not reuse a handled zoom gesture"
        );
    }

    fn frame(context: &egui::Context, session: &mut Session, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            events,
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| show(ui, session));
        output.textures_delta.clear();
    }

    #[test]
    fn bus_click_sets_cursor_and_drag_does_not_reposition_it() {
        let context = egui::Context::default();
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        frame(&context, &mut session, vec![]);
        let point = Pos2::new(750.0, 138.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &context,
            &mut session,
            vec![egui::Event::PointerMoved(point), button(point, true)],
        );
        frame(&context, &mut session, vec![button(point, false)]);
        assert!(
            session.cursor.is_some(),
            "bus hover tooltip must not intercept clicks"
        );
        let cursor = session.cursor;
        let end = session.trace.end_time;
        session.viewport.zoom(0.5, 12, end);
        let before = session.viewport;
        frame(&context, &mut session, vec![button(point, true)]);
        let moved = point + egui::vec2(100.0, 0.0);
        frame(
            &context,
            &mut session,
            vec![egui::Event::PointerMoved(moved)],
        );
        frame(&context, &mut session, vec![button(moved, false)]);
        assert_eq!(session.cursor, cursor, "drag must preserve cursor");
        assert_ne!(session.viewport, before, "drag should pan a zoomed trace");
    }

    #[test]
    fn an_initial_scalar_sample_does_not_invent_a_transition() {
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            let colors = Colors::new(ui);
            scalar(
                ui.painter(),
                Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 30.0)),
                Logic::Zero,
                None,
                &colors,
            );
        });
        let lines = output
            .shapes
            .iter()
            .filter(|shape| matches!(shape.shape, egui::Shape::LineSegment { .. }))
            .count();
        output.textures_delta.clear();
        assert_eq!(
            lines, 1,
            "initial value should be one horizontal line, with no fake edge"
        );
    }
}
