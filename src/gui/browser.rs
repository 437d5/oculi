use crate::viewer::Session;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, session: &mut Session) {
    ui.add_space(6.0);
    ui.strong("Design hierarchy");
    let scopes = session.scopes();
    egui::ScrollArea::vertical()
        .id_salt("hierarchy")
        .max_height(ui.available_height() * 0.4)
        .show(ui, |ui| {
            if ui
                .selectable_label(session.scope.is_empty(), "(root)")
                .clicked()
            {
                session.select_scope(Vec::new());
            }
            children(ui, &scopes, &[], session);
        });
    ui.separator();
    ui.strong("Signals");
    ui.weak(if session.scope.is_empty() {
        "(root)".into()
    } else {
        session.scope.join(".")
    });
    ui.add(
        egui::TextEdit::singleline(&mut session.filter)
            .id(egui::Id::unique("signal_filter"))
            .hint_text("Filter signals…")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(5.0);
    egui::ScrollArea::vertical()
        .id_salt(("signals", &session.scope))
        .show(ui, |ui| {
            let visible = session.visible_signals();
            if visible.is_empty() {
                if session.filter.is_empty() {
                    ui.weak("No signals declared directly in this scope. Select a child scope.");
                } else {
                    ui.weak("No signals match the filter.");
                    if ui.button("Clear filter").clicked() {
                        session.filter.clear();
                    }
                }
            }
            for index in visible {
                let signal = &session.trace.signals[index];
                let name = signal.path.last().cloned().unwrap_or_default();
                let width = signal.width;
                let mut selected = session.selected.contains(&index);
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed = ui.checkbox(&mut selected, name).changed();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak(format!("{width}b"));
                    });
                });
                if changed {
                    session.set_selected(index, selected);
                }
            }
        });
}

fn children(ui: &mut egui::Ui, scopes: &[Vec<String>], parent: &[String], session: &mut Session) {
    for scope in scopes
        .iter()
        .filter(|s| s.len() == parent.len() + 1 && s.starts_with(parent))
    {
        let name = scope.last().map(String::as_str).unwrap_or_default();
        let has_children = scopes
            .iter()
            .any(|s| s.len() == scope.len() + 1 && s.starts_with(scope));
        if has_children {
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.scope_id().with(("scope", scope)),
                true,
            )
            .show_header(ui, |ui| {
                if ui
                    .selectable_label(session.scope == *scope, name)
                    .on_hover_text(scope.join("."))
                    .clicked()
                {
                    session.select_scope(scope.clone());
                }
            })
            .body(|ui| children(ui, scopes, scope, session));
        } else if ui
            .selectable_label(session.scope == *scope, name)
            .on_hover_text(scope.join("."))
            .clicked()
        {
            session.select_scope(scope.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        ctx: &egui::Context,
        session: &mut Session,
        events: Vec<egui::Event>,
        label: &str,
    ) -> Option<egui::Pos2> {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(242.0, 650.0),
                )),
                events,
                ..Default::default()
            },
            |ui| show(ui, session),
        );
        let point = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + egui::vec2(6.0, 6.0))
            }
            _ => None,
        });
        output.textures_delta.clear();
        point
    }

    #[test]
    fn selecting_dut_shows_its_signals_even_after_filtering_parent_scope() {
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        session.filter = "count".into();
        let ctx = egui::Context::default();
        let point = frame(&ctx, &mut session, vec![], "dut").expect("DUT must be visible");
        let button = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &ctx,
            &mut session,
            vec![egui::Event::PointerMoved(point), button(true)],
            "dut",
        );
        frame(&ctx, &mut session, vec![button(false)], "dut");
        assert_eq!(session.scope, ["tb", "dut"]);
        assert_eq!(
            session.visible_signals(),
            [2],
            "parent's filter must not hide the DUT's clock"
        );
    }

    #[test]
    fn clicking_scope_name_selects_without_collapsing_its_children() {
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        let ctx = egui::Context::default();
        let point = frame(&ctx, &mut session, vec![], "tb").unwrap();
        let button = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &ctx,
            &mut session,
            vec![egui::Event::PointerMoved(point), button(true)],
            "dut",
        );
        frame(&ctx, &mut session, vec![button(false)], "dut");
        // Allow the collapse animation to finish before checking visibility.
        for _ in 0..40 {
            frame(&ctx, &mut session, vec![], "dut");
        }
        assert!(
            frame(&ctx, &mut session, vec![], "dut").is_some(),
            "click the arrow, not the name, to collapse a scope"
        );
    }
}
