use oculi::{
    trace::Trace,
    viewer::{Session, cursors::Cursors},
};

#[test]
fn placement_moves_only_the_active_cursor_and_creation_preserves_existing_times() {
    let mut cursors = Cursors::default();
    assert!(cursors.all().is_empty());
    assert!(cursors.active().is_none());
    let a = cursors.place(5);
    assert_eq!(a.name(), "A");
    assert_eq!(cursors.place(10), a);
    assert_eq!(cursors.all().len(), 1);
    let b = cursors.add(20);
    let c = cursors.add(30);
    assert_eq!(cursors.active().unwrap().id, c);
    assert_eq!(
        cursors
            .all()
            .iter()
            .map(|cursor| cursor.time)
            .collect::<Vec<_>>(),
        [10, 20, 30]
    );
    assert!(cursors.select(b));
    assert_eq!(cursors.place(25), b);
    assert_eq!(
        cursors
            .all()
            .iter()
            .map(|cursor| cursor.time)
            .collect::<Vec<_>>(),
        [10, 25, 30]
    );
    assert!(cursors.select(a));
    assert_eq!(cursors.active().unwrap().time, 10);
}

#[test]
fn removal_preserves_names_and_keeps_a_valid_active_cursor() {
    let mut cursors = Cursors::default();
    let a = cursors.add(1);
    let b = cursors.add(2);
    let c = cursors.add(3);
    assert!(cursors.select(b));
    assert!(cursors.remove(c));
    assert_eq!(cursors.active().unwrap().id, b);
    assert!(cursors.remove(b));
    assert_eq!(cursors.active().unwrap().id, a);
    assert!(!cursors.select(b));
    assert!(!cursors.remove(b));
    assert_eq!(cursors.active().unwrap().id, a);
    let d = cursors.add(4);
    assert_eq!(d.name(), "D", "deleted identities must not be reused");
    assert!(cursors.select(a));
    assert!(cursors.remove(a));
    assert_eq!(cursors.active().unwrap().id, d);
    assert!(cursors.remove(d));
    assert!(cursors.all().is_empty());
    assert!(cursors.active().is_none());
    assert_eq!(cursors.place(5).name(), "E");
}

#[test]
fn cursor_names_continue_past_z_without_renumbering() {
    let mut cursors = Cursors::default();
    let ids: Vec<_> = (0..53).map(|time| cursors.add(time)).collect();
    for (index, name) in [(0, "A"), (25, "Z"), (26, "AA"), (51, "AZ"), (52, "BA")] {
        assert_eq!(ids[index].name(), name);
    }
    assert!(cursors.remove(ids[0]));
    assert_eq!(cursors.all()[0].id.name(), "B");
    assert_eq!(cursors.active().unwrap().id.name(), "BA");
}

#[test]
fn session_clamps_placements_without_changing_other_cursors_or_the_viewport() {
    let mut session = Session::new(Trace {
        end_time: 100,
        ..Default::default()
    });
    let viewport = session.viewport;
    let a = session.add_cursor(u64::MAX);
    let b = session.add_cursor(10);
    assert_eq!(
        session
            .cursors
            .all()
            .iter()
            .map(|cursor| cursor.time)
            .collect::<Vec<_>>(),
        [100, 10]
    );
    session.place_cursor(200);
    assert_eq!(session.cursors.active().unwrap().id, b);
    assert_eq!(session.cursors.active().unwrap().time, 100);
    assert!(session.cursors.select(a));
    session.place_cursor(0);
    assert_eq!(
        session
            .cursors
            .all()
            .iter()
            .map(|cursor| cursor.time)
            .collect::<Vec<_>>(),
        [0, 100]
    );
    assert_eq!(session.viewport, viewport);
    let mut empty = Session::new(Trace::default());
    empty.add_cursor(500);
    empty.place_cursor(u64::MAX);
    assert_eq!(empty.cursors.all().len(), 1);
    assert_eq!(empty.cursors.active().unwrap().time, 0);
}
