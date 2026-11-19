use crate::viewer::Session;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, session: &mut Session) {
    ui.add_space(6.0);
    hierarchy(ui, session);
    ui.separator();
    ui.strong("Signals");
    ui.weak(if session.scope.is_empty() {
        "(root)".into()
    } else {
        session.scope.join(".")
    });
    let finder = search_field(ui, &mut session.filter, "signal_filter", "Find signal…");
    let visible = session.visible_signals();
    finder.navigate(&mut session.signal_match, visible.len());
    if finder.enter
        && let Some(&index) = visible.get(session.signal_match)
    {
        session.set_selected(index, !session.selected.contains(&index));
    }
    ui.add_space(5.0);
    let mut scroll = egui::ScrollArea::vertical().id_salt(("signals", &session.scope));
    if finder.changed {
        scroll = scroll.vertical_scroll_offset(0.0);
    }
    scroll.show(ui, |ui| {
        if visible.is_empty() {
            if session.filter.trim().is_empty() {
                ui.weak("No signals declared directly in this scope. Select a child scope.");
            } else {
                ui.weak("No signals match the filter.");
            }
        }
        for (position, index) in visible.into_iter().enumerate() {
            let signal = &session.trace.signals[index];
            let name = signal.path.last().cloned().unwrap_or_default();
            let width = signal.width;
            let mut selected = session.selected.contains(&index);
            let mut changed = false;
            let active = position == session.signal_match;
            let fill = if active && finder.response.has_focus() {
                ui.visuals().selection.bg_fill.gamma_multiply(0.35)
            } else {
                egui::Color32::TRANSPARENT
            };
            let row = egui::Frame::new().fill(fill).show(ui, |ui| {
                ui.horizontal(|ui| {
                    changed = ui.checkbox(&mut selected, name).changed();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak(format!("{width}b"));
                    });
                });
            });
            if active && (finder.up || finder.down || finder.changed) {
                row.response.scroll_to_me(Some(egui::Align::Center));
            }
            if changed {
                session.signal_match = position;
                session.set_selected(index, selected);
                finder.response.request_focus();
            }
        }
    });
}

struct Finder {
    response: egui::Response,
    changed: bool,
    up: bool,
    down: bool,
    enter: bool,
}

impl Finder {
    fn navigate(&self, active: &mut usize, count: usize) {
        if self.changed {
            *active = 0;
        }
        let last = count.saturating_sub(1);
        *active = (*active).min(last);
        if self.up {
            *active = active.saturating_sub(1);
        }
        if self.down {
            *active = (*active + 1).min(last);
        }
    }
}

fn search_field(ui: &mut egui::Ui, query: &mut String, id: &str, hint: &str) -> Finder {
    let search_id = egui::Id::unique(id);
    // Consume navigation before the single-line editor handles arrows or Enter.
    let focused = ui.memory(|memory| memory.has_focus(search_id));
    let (up, down, enter, escape) = ui.input_mut(|input| {
        let mut key = |key| focused && input.consume_key(egui::Modifiers::NONE, key);
        (
            key(egui::Key::ArrowUp),
            key(egui::Key::ArrowDown),
            key(egui::Key::Enter),
            key(egui::Key::Escape),
        )
    });
    let mut cleared = false;
    let search = ui
        .horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Reserve the actual button width before filling the remaining space.
                let clear = ui.small_button("Clear").clicked();
                let response = ui.add(
                    egui::TextEdit::singleline(query)
                        .id(search_id)
                        .event_filter(egui::EventFilter {
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                            ..Default::default()
                        })
                        .hint_text(hint)
                        .align(egui::Align2::LEFT_CENTER)
                        .desired_width(f32::INFINITY),
                );
                if clear {
                    query.clear();
                    cleared = true;
                    response.request_focus();
                }
                response
            })
            .inner
        })
        .inner;
    if escape {
        query.clear();
        search.request_focus();
    }
    Finder {
        changed: search.changed() || cleared || escape,
        response: search,
        up,
        down,
        enter,
    }
}

fn hierarchy(ui: &mut egui::Ui, session: &mut Session) {
    ui.strong("Design hierarchy");
    let finder = search_field(
        ui,
        &mut session.hierarchy_query,
        "hierarchy_search",
        "Find scope…",
    );
    if finder.changed {
        session.hierarchy_match = 0;
    }
    let scopes = session.scopes();
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt("hierarchy")
        .max_height(ui.available_height() * 0.4);
    if finder.changed {
        scroll = scroll.vertical_scroll_offset(0.0);
    }
    if session.hierarchy_query.trim().is_empty() {
        scroll.show(ui, |ui| {
            if ui
                .selectable_label(session.scope.is_empty(), "(root)")
                .clicked()
            {
                session.select_scope(Vec::new());
            }
            children(ui, &scopes, &[], session);
        });
        return;
    }
    let matches = crate::viewer::hierarchy::search(&scopes, &session.hierarchy_query);
    finder.navigate(&mut session.hierarchy_match, matches.len());
    if finder.enter
        && let Some(scope) = matches.get(session.hierarchy_match)
    {
        session.select_scope(scope.to_vec());
    }
    scroll.show(ui, |ui| {
        if matches.is_empty() {
            ui.weak("No matching scopes.");
        }
        for (index, scope) in matches.into_iter().enumerate() {
            let response = ui
                .selectable_label(index == session.hierarchy_match, scope.join("."))
                .on_hover_text(scope.join("."));
            if index == session.hierarchy_match && (finder.up || finder.down || finder.changed) {
                response.scroll_to_me(Some(egui::Align::Center));
            }
            if response.clicked() {
                session.hierarchy_match = index;
                session.select_scope(scope.to_vec());
                finder.response.request_focus();
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
        frame_match(ctx, session, events, label, 0)
    }

    fn frame_match(
        ctx: &egui::Context,
        session: &mut Session,
        events: Vec<egui::Event>,
        label: &str,
        occurrence: usize,
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
        let point = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + egui::vec2(6.0, 6.0))
                }
                _ => None,
            })
            .nth(occurrence);
        output.textures_delta.clear();
        point
    }

    fn press(key: egui::Key) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect()
    }

    fn finder_session() -> Session {
        Session::new(
            crate::vcd::load(
                &b"$scope module tb $end
$scope module a $end
$scope module uart $end
$var wire 1 ! clk $end
$upscope $end
$upscope $end
$scope module b $end
$scope module uart $end
$var wire 1 ! clk $end
$upscope $end
$upscope $end
$scope module c $end
$scope module uart $end
$var wire 1 ! clk $end
$upscope $end
$upscope $end
$upscope $end
$enddefinitions $end
#0
0!
#100
1!
"[..],
            )
            .unwrap(),
        )
    }

    fn focus_finder(ctx: &egui::Context, session: &mut Session) {
        frame(ctx, session, vec![], "");
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::unique("hierarchy_search")));
        frame(ctx, session, vec![], "");
    }

    #[test]
    fn signal_finder_handles_no_matches_and_clear_preserves_hierarchy_and_waveforms() {
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        session.hierarchy_query = "TD".into();
        let selected = session.selected.clone();
        let ctx = egui::Context::default();
        frame(&ctx, &mut session, vec![], "");
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::unique("signal_filter")));
        frame(&ctx, &mut session, vec![], "");
        assert!(
            frame(
                &ctx,
                &mut session,
                vec![egui::Event::Text("missing".into())],
                "No signals match the filter.",
            )
            .is_some()
        );
        for key in [egui::Key::ArrowDown, egui::Key::ArrowUp, egui::Key::Enter] {
            frame(&ctx, &mut session, press(key), "");
        }
        assert_eq!(session.signal_match, 0);
        assert_eq!(session.selected, selected);
        let point = frame_match(&ctx, &mut session, vec![], "Clear", 1)
            .expect("the Signals finder needs its own Clear button");
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut session,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                "",
            );
        }
        assert!(session.filter.is_empty());
        assert_eq!(session.visible_signals(), [0, 1]);
        assert_eq!(session.hierarchy_query, "TD");
        assert_eq!(session.selected, selected);
    }

    #[test]
    fn signal_finder_accepts_fuzzy_queries_and_toggles_with_keyboard() {
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        session.selected.clear();
        session.hierarchy_query = "TD".into();
        let ctx = egui::Context::default();
        assert!(frame(&ctx, &mut session, vec![], "Find signal…").is_some());
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::unique("signal_filter")));
        frame(&ctx, &mut session, vec![], "");
        frame(&ctx, &mut session, vec![egui::Event::Text("c".into())], "");
        assert!(session.selected.is_empty(), "typing must not add waveforms");
        frame(&ctx, &mut session, press(egui::Key::ArrowDown), "");
        frame(&ctx, &mut session, press(egui::Key::Enter), "");
        assert_eq!(session.selected, [1], "Enter should select count");
        frame(&ctx, &mut session, press(egui::Key::Enter), "");
        assert!(
            session.selected.is_empty(),
            "Enter again should remove count"
        );
        frame(&ctx, &mut session, press(egui::Key::ArrowUp), "");
        frame(&ctx, &mut session, press(egui::Key::Enter), "");
        assert_eq!(session.selected, [0], "Up should return to clk");
        frame(&ctx, &mut session, vec![egui::Event::Text("t".into())], "");
        assert_eq!(session.filter, "ct");
        assert_eq!(session.visible_signals(), [1]);
        frame(&ctx, &mut session, press(egui::Key::Enter), "");
        assert_eq!(session.selected, [0, 1]);
        frame(&ctx, &mut session, press(egui::Key::Escape), "");
        assert!(session.filter.is_empty());
        assert_eq!(session.visible_signals(), [0, 1]);
        assert_eq!(session.selected, [0, 1], "clearing must preserve waveforms");
        assert_eq!(
            session.hierarchy_query, "TD",
            "search fields stay independent"
        );
    }

    #[test]
    fn arrow_keys_navigate_ranked_scopes_and_stop_at_list_boundaries() {
        let mut session = finder_session();
        let ctx = egui::Context::default();
        focus_finder(&ctx, &mut session);
        frame(
            &ctx,
            &mut session,
            vec![egui::Event::Text("uart".into())],
            "",
        );
        frame(&ctx, &mut session, press(egui::Key::ArrowUp), "");
        assert_eq!(session.hierarchy_match, 0);
        frame(&ctx, &mut session, press(egui::Key::ArrowDown), "");
        assert_eq!(session.hierarchy_match, 1);
        assert_eq!(session.scope, ["tb", "a", "uart"]);
        frame(&ctx, &mut session, press(egui::Key::Enter), "");
        assert_eq!(session.scope, ["tb", "b", "uart"]);
        for _ in 0..4 {
            frame(&ctx, &mut session, press(egui::Key::ArrowDown), "");
        }
        assert_eq!(session.hierarchy_match, 2);
        frame(&ctx, &mut session, press(egui::Key::Enter), "");
        assert_eq!(session.scope, ["tb", "c", "uart"]);
        frame(&ctx, &mut session, press(egui::Key::ArrowUp), "");
        frame(&ctx, &mut session, press(egui::Key::Enter), "");
        assert_eq!(session.scope, ["tb", "b", "uart"]);
    }

    #[test]
    fn edited_queries_reset_highlight_and_empty_results_cannot_select_a_scope() {
        let mut session = finder_session();
        let ctx = egui::Context::default();
        focus_finder(&ctx, &mut session);
        frame(
            &ctx,
            &mut session,
            vec![egui::Event::Text("uar".into())],
            "",
        );
        frame(&ctx, &mut session, press(egui::Key::ArrowDown), "");
        assert_eq!(session.hierarchy_match, 1);
        frame(&ctx, &mut session, vec![egui::Event::Text("t".into())], "");
        assert_eq!(session.hierarchy_match, 0);
        assert!(
            frame(
                &ctx,
                &mut session,
                vec![egui::Event::Text("missing".into())],
                "No matching scopes.",
            )
            .is_some()
        );
        for key in [egui::Key::ArrowDown, egui::Key::ArrowUp, egui::Key::Enter] {
            frame(&ctx, &mut session, press(key), "");
        }
        assert_eq!(session.hierarchy_match, 0);
        assert_eq!(session.scope, ["tb", "a", "uart"]);
    }

    #[test]
    fn clicking_a_result_selects_its_full_path_and_clears_the_signal_filter() {
        let mut session = finder_session();
        session.filter = "missing".into();
        let ctx = egui::Context::default();
        focus_finder(&ctx, &mut session);
        let point = frame(
            &ctx,
            &mut session,
            vec![egui::Event::Text("uart".into())],
            "tb.c.uart",
        )
        .unwrap();
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut session,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                "",
            );
        }
        assert_eq!(session.scope, ["tb", "c", "uart"]);
        assert!(session.filter.is_empty());
        assert_eq!(session.hierarchy_match, 2);
    }

    #[test]
    fn clear_and_escape_restore_the_tree_without_changing_the_selected_scope() {
        let mut session = finder_session();
        let ctx = egui::Context::default();
        focus_finder(&ctx, &mut session);
        let point = frame(
            &ctx,
            &mut session,
            vec![egui::Event::Text("uart".into())],
            "Clear",
        )
        .unwrap();
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut session,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                "",
            );
        }
        assert!(session.hierarchy_query.is_empty());
        assert!(frame(&ctx, &mut session, vec![], "tb").is_some());
        frame(
            &ctx,
            &mut session,
            vec![egui::Event::Text("uart".into())],
            "",
        );
        assert_eq!(session.hierarchy_query, "uart");
        frame(&ctx, &mut session, press(egui::Key::Escape), "");
        assert!(session.hierarchy_query.is_empty());
        assert!(frame(&ctx, &mut session, vec![], "tb").is_some());
        assert_eq!(session.scope, ["tb", "a", "uart"]);
    }

    #[test]
    fn fuzzy_scope_search_selects_with_enter_without_changing_waveforms() {
        let mut session = Session::new(
            crate::vcd::load(&include_bytes!("../../examples/counter.vcd")[..]).unwrap(),
        );
        session.filter = "count".into();
        session.place_cursor(10);
        let selected = session.selected.clone();
        let viewport = session.viewport;
        let ctx = egui::Context::default();
        let point = frame(&ctx, &mut session, vec![], "Find scope…")
            .expect("hierarchy needs a fuzzy search field");
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
        assert!(
            frame(
                &ctx,
                &mut session,
                vec![egui::Event::Text("TD".into())],
                "tb.dut",
            )
            .is_some(),
            "skipped, case-insensitive characters should find full scope paths"
        );
        assert_eq!(session.scope, ["tb"], "typing must not change scope");
        assert_eq!(session.filter, "count");
        frame(
            &ctx,
            &mut session,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            "tb.dut",
        );
        assert_eq!(session.scope, ["tb", "dut"]);
        assert!(session.filter.is_empty());
        assert_eq!(session.selected, selected);
        assert_eq!(session.cursors.active().map(|cursor| cursor.time), Some(10));
        assert_eq!(session.viewport, viewport);
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
