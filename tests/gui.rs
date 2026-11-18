#![cfg(feature = "gui")]
use oculi::gui::{AppState, Startup};
use std::{ffi::OsString, path::PathBuf};

#[test]
fn startup_arguments_preserve_paths_and_reject_extras() {
    assert_eq!(
        Startup::parse(Vec::<OsString>::new()).unwrap(),
        Startup::Open(None)
    );
    assert_eq!(
        Startup::parse([OsString::from("--help")]).unwrap(),
        Startup::Help
    );
    assert_eq!(
        Startup::parse([OsString::from("trace.vcd")]).unwrap(),
        Startup::Open(Some(PathBuf::from("trace.vcd")))
    );
    assert!(Startup::parse(["one".into(), "two".into()]).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let path = OsString::from_vec(b"trace-\xff.vcd".to_vec());
        assert_eq!(
            Startup::parse([path.clone()]).unwrap(),
            Startup::Open(Some(path.into()))
        );
    }
}

#[test]
fn failed_open_preserves_old_session_and_success_resets_it() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut state = AppState::default();
    state.open(&root.join("examples/counter.vcd"));
    assert!(state.error.is_none());
    let session = state.session.as_mut().unwrap();
    session.selected.clear();
    session.cursor = Some(10);
    state.open(&root.join("Cargo.toml"));
    assert!(state.error.is_some());
    assert!(state.session.as_ref().unwrap().selected.is_empty());
    assert_eq!(state.session.as_ref().unwrap().cursor, Some(10));
    state.open(&root.join("examples/counter.vcd"));
    assert!(state.error.is_none());
    assert_eq!(state.session.as_ref().unwrap().cursor, None);
    assert!(!state.session.as_ref().unwrap().selected.is_empty());
}

#[test]
fn zoom_shortcuts_work_but_do_not_intercept_filter_typing() {
    use eframe::egui;
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut app = AppState::default();
    app.open(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/counter.vcd"));
    let frame = |app: &mut AppState, events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ui| app.show(ui),
        );
        output.textures_delta.clear();
    };
    let key = |key| {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]
    };
    frame(&mut app, vec![]);
    frame(&mut app, key(egui::Key::Plus));
    assert_eq!(app.session.as_ref().unwrap().viewport.span(), 13);
    frame(&mut app, key(egui::Key::Minus));
    assert_eq!(app.session.as_ref().unwrap().viewport.span(), 25);
    frame(&mut app, key(egui::Key::Plus));
    frame(&mut app, key(egui::Key::F));
    assert_eq!(app.session.as_ref().unwrap().viewport.span(), 25);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::unique("signal_filter")));
    frame(&mut app, vec![]);
    frame(&mut app, key(egui::Key::Plus));
    assert_eq!(
        app.session.as_ref().unwrap().viewport.span(),
        25,
        "typing in a filter must not zoom"
    );
}

#[test]
fn ui_renders_headless_in_both_themes_with_and_without_a_trace() {
    let context = eframe::egui::Context::default();
    egui_extras::install_image_loaders(&context);
    for theme in [eframe::egui::Theme::Dark, eframe::egui::Theme::Light] {
        context.set_theme(theme);
        for file in [None, Some("examples/counter.vcd")] {
            let mut app = AppState::default();
            if let Some(file) = file {
                app.open(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file));
            }
            let input = eframe::egui::RawInput {
                screen_rect: Some(eframe::egui::Rect::from_min_size(
                    eframe::egui::Pos2::ZERO,
                    eframe::egui::vec2(1280.0, 800.0),
                )),
                ..Default::default()
            };
            let mut output = context.run_ui(input, |ui| app.show(ui));
            assert!(!output.shapes.is_empty());
            // A headless test has no GPU backend to apply uploaded textures.
            output.textures_delta.clear();
        }
    }
}
