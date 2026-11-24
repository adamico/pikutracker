mod support;

use pikutracker_core::app::{Action, App, LogItem, Report, Severity, layout};
use pikutracker_core::keymap::{Key, KeyPress, Modifiers};
use pikutracker_core::player::AudioRequest;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use std::time::Duration;
use support::{M8_STYLE, apply, launched, pane_text};

const PC_TRACKER: &str = "(version: 1)";
const TO_THE_PHRASE_PANE: [Action; 2] = [Action::FocusPrevious, Action::FocusPrevious];
const MIDDLE_C: u8 = 0;
const NAME_COLUMNS: usize = 14;

fn one_phrase_in_the_song() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    project
}

fn in_the_phrase_pane(config: &str) -> App {
    let mut app = launched(App::with_project(one_phrase_in_the_song()), config);
    apply(&mut app, &TO_THE_PHRASE_PANE);
    app
}

fn in_the_chain_pane(config: &str) -> App {
    let mut app = launched(App::with_project(one_phrase_in_the_song()), config);
    app.apply(Action::FocusPrevious);
    app
}

fn in_the_song_pane(config: &str) -> App {
    launched(App::with_project(one_phrase_in_the_song()), config)
}

fn assert_lists(app: &App, expected: &[(&str, &str)]) {
    let lines = help_lines(app);
    for &(name, keys) in expected {
        let line = format!("{name} {keys}");
        assert!(lines.contains(&line), "{line} in {lines:#?}");
    }
}

fn help_names(app: &App) -> Vec<String> {
    right_slot(app)
        .into_iter()
        .skip(1)
        .map(|line| line.chars().take(NAME_COLUMNS).collect::<String>())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

fn right_slot(app: &App) -> Vec<String> {
    (0..layout::RIGHT_SLOT.rows)
        .map(|line| pane_text(app, layout::RIGHT_SLOT, line))
        .collect()
}

fn help_lines(app: &App) -> Vec<String> {
    let slot = right_slot(app);
    assert_eq!(slot[0], "HELP", "{slot:#?}");
    slot.into_iter()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

fn keys_of<'a>(lines: &'a [String], name: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        line.strip_prefix(name)
            .and_then(|rest| rest.strip_prefix(' '))
            .filter(|keys| !keys.contains(' '))
    })
}

fn press(app: &mut App, key: Key, modifiers: Modifiers, at: Duration) -> Vec<Action> {
    let mut actions: Vec<Action> = app
        .key_down(KeyPress::new(key, modifiers), at)
        .into_iter()
        .collect();
    actions.extend(app.key_up(key));
    for &action in &actions {
        app.apply(action);
    }
    actions
}

fn chord(app: &mut App, held: Key, key: Key, at: Duration) -> Vec<Action> {
    let mut actions = Vec::new();
    actions.extend(app.key_down(KeyPress::new(held, Modifiers::NONE), at));
    actions.extend(app.key_down(KeyPress::new(key, Modifiers::NONE), at));
    actions.extend(app.key_up(key));
    actions.extend(app.key_up(held));
    for &action in &actions {
        app.apply(action);
    }
    actions
}

#[test]
fn ctrl_h_opens_and_closes_help_over_the_right_hand_slot_in_pc_tracker() {
    let mut app = in_the_phrase_pane(PC_TRACKER);
    let multitrack = right_slot(&app);

    let opened = press(&mut app, Key::Char('h'), Modifiers::CTRL, Duration::ZERO);

    assert_eq!(opened, [Action::ToggleHelp]);
    assert_eq!(pane_text(&app, layout::RIGHT_SLOT, 0), "HELP");

    press(
        &mut app,
        Key::Char('h'),
        Modifiers::CTRL,
        Duration::from_secs(1),
    );

    assert_eq!(right_slot(&app), multitrack);
}

#[test]
fn x_plus_h_opens_and_closes_help_over_the_right_hand_slot_in_m8_style() {
    let mut app = in_the_phrase_pane(M8_STYLE);
    let multitrack = right_slot(&app);

    let opened = chord(&mut app, Key::Char('x'), Key::Char('h'), Duration::ZERO);

    assert_eq!(opened, [Action::ToggleHelp]);
    assert_eq!(pane_text(&app, layout::RIGHT_SLOT, 0), "HELP");

    chord(
        &mut app,
        Key::Char('x'),
        Key::Char('h'),
        Duration::from_secs(1),
    );

    assert_eq!(right_slot(&app), multitrack);
}

#[test]
fn help_lists_the_phrase_panes_pc_tracker_keys() {
    let mut app = in_the_phrase_pane(PC_TRACKER);

    app.apply(Action::ToggleHelp);

    let lines = help_lines(&app);
    let expected = [
        ("MOVE", "ARROWS"),
        ("JUMP", "ALT+ARROWS"),
        ("NOTE", "ZSXDCVGBHNJM/Q2W3ER5T6Y7U"),
        ("DIGIT", "0-F"),
        ("NUDGE", "^UP/^DOWN/^SHIFT+UP/^SHIFT+DOWN"),
        ("SELECT", "SHIFT+ARROWS"),
        ("SELECT JUMP", "ALT+SHIFT+ARROWS"),
        ("COPY", "^C"),
        ("MOVE ROWS", "ALT+UP/ALT+DOWN"),
        ("PLAY", "SPACE"),
        ("NEXT PANE", "TAB"),
    ];
    for (name, keys) in expected {
        assert_eq!(keys_of(&lines, name), Some(keys), "{name} in {lines:#?}");
    }
}

#[test]
fn help_lists_the_phrase_panes_m8_style_keys() {
    let mut app = in_the_phrase_pane(M8_STYLE);

    app.apply(Action::ToggleHelp);

    let lines = help_lines(&app);
    let expected = [
        ("JUMP", "X+ARROWS"),
        ("NOTE", "Q2W3ER5T6Y7U/I9O0P"),
        ("NUDGE", "Z+RIGHT/Z+LEFT/Z+UP/Z+DOWN"),
        ("SELECT", "SHIFT+X/ARROWS"),
        ("SELECT JUMP", "X+LEFT/X+RIGHT"),
        ("COPY", "X"),
        ("CUT", "Z+X"),
        ("PASTE", "SHIFT+Z"),
        ("SCALE SPEED", "Z+T"),
        ("NEXT PANE", "TAB/SHIFT+DOWN/SHIFT+RIGHT"),
    ];
    for (name, keys) in expected {
        assert_eq!(keys_of(&lines, name), Some(keys), "{name} in {lines:#?}");
    }
}

#[test]
fn help_shows_a_key_rebound_in_config_ron_instead_of_the_presets_key() {
    let mut app = in_the_phrase_pane(r#"(version: 1, keys: {"Copy": "Ctrl+K"})"#);

    app.apply(Action::ToggleHelp);

    let lines = help_lines(&app);
    assert_eq!(keys_of(&lines, "COPY"), Some("^K"));
    assert!(
        lines.iter().all(|line| !line.ends_with(" ^C")),
        "{lines:#?}"
    );
}

#[test]
fn help_leaves_out_the_keys_that_work_anywhere() {
    let panes: [fn(&str) -> App; 3] = [in_the_phrase_pane, in_the_chain_pane, in_the_song_pane];
    for (config, in_the_pane) in [PC_TRACKER, M8_STYLE]
        .into_iter()
        .flat_map(|config| panes.map(|in_the_pane| (config, in_the_pane)))
    {
        let mut app = in_the_pane(config);

        app.apply(Action::ToggleHelp);

        let lines = help_lines(&app);
        let anywhere = [
            "NEW",
            "OPEN",
            "SAVE",
            "SAVE AS",
            "IMPORT",
            "EXPORT",
            "QUIT",
            "CART",
            "LOG",
            "HELP",
            "MUTE",
            "SOLO",
            "FOLLOW",
            "PLAY SONG",
        ];
        let names = help_names(&app);
        for name in anywhere {
            assert_eq!(keys_of(&lines, name), None, "{name} in {lines:#?}");
            assert!(
                !names.iter().any(|shown| shown == name),
                "{name} in {names:#?}"
            );
        }
    }
}

#[test]
fn help_leaves_focus_alone_and_an_edit_key_still_edits_the_phrase() {
    let mut app = in_the_phrase_pane(PC_TRACKER);
    let context = app.key_context();
    let before = app.project().phrase(PhraseId::new(0)).step(0);

    app.apply(Action::ToggleHelp);
    let entered = press(&mut app, Key::Char('z'), Modifiers::NONE, Duration::ZERO);

    assert_eq!(app.key_context(), context);
    assert_eq!(entered, [Action::PianoKey(MIDDLE_C)]);
    assert_ne!(app.project().phrase(PhraseId::new(0)).step(0), before);
    assert_eq!(pane_text(&app, layout::RIGHT_SLOT, 0), "HELP");
}

#[test]
fn the_player_keeps_playing_across_opening_and_closing_help() {
    let mut app = in_the_phrase_pane(PC_TRACKER);
    apply(
        &mut app,
        &[Action::PianoKey(MIDDLE_C), Action::PlayFocusedPane],
    );
    let requests = app.take_audio_requests();
    assert!(
        matches!(requests.last(), Some(AudioRequest::PlayPhrase { .. })),
        "{requests:?}"
    );

    apply(&mut app, &[Action::ToggleHelp, Action::ToggleHelp]);

    assert_eq!(app.take_audio_requests(), []);
}

#[test]
fn a_log_that_opens_on_its_own_closes_help() {
    let mut app = in_the_phrase_pane(PC_TRACKER);
    app.apply(Action::ToggleHelp);

    app.report(
        Report::failure("EXPORT FAILED")
            .with_items(vec![LogItem::new(Severity::Failure, "TOO MANY SFX")]),
    );

    assert_eq!(pane_text(&app, layout::RIGHT_SLOT, 0), "LOG");
    app.apply(Action::ToggleLog);
    assert_ne!(pane_text(&app, layout::RIGHT_SLOT, 0), "HELP");
}

#[test]
fn help_lists_the_chain_panes_pc_tracker_keys() {
    let mut app = in_the_chain_pane(PC_TRACKER);

    app.apply(Action::ToggleHelp);

    assert_lists(
        &app,
        &[
            ("MOVE", "ARROWS"),
            ("JUMP", "ALT+ARROWS"),
            ("DIGIT", "0-F"),
            ("NUDGE", "^UP/^DOWN/^SHIFT+UP/^SHIFT+DOWN"),
            ("CLEAR", "DEL"),
            ("SELECT", "SHIFT+ARROWS"),
            ("COPY", "^C"),
            ("PASTE", "^V"),
            ("CLONE", "^D"),
            ("NEW UNUSED", "^SHIFT+N"),
            ("MOVE ROWS", "ALT+UP/ALT+DOWN"),
            ("CHANGE SPEED", "^SHIFT+T"),
            ("SCALE SPEED", "ALT+T"),
            ("PLAY", "SPACE"),
            ("NEXT PANE", "TAB"),
        ],
    );
}

#[test]
fn help_lists_the_chain_panes_m8_style_keys() {
    let mut app = in_the_chain_pane(M8_STYLE);

    app.apply(Action::ToggleHelp);

    assert_lists(
        &app,
        &[
            ("JUMP", "X+ARROWS"),
            ("NUDGE", "Z+RIGHT/Z+LEFT/Z+UP/Z+DOWN"),
            ("SELECT", "SHIFT+X/ARROWS"),
            ("COPY", "X"),
            ("CUT", "Z+X"),
            ("PASTE", "SHIFT+Z"),
            ("CLONE", "Z"),
            ("NEW UNUSED", "DOUBLE-TAP Z"),
            ("MOVE ROWS", "X+UP/X+DOWN"),
            ("SCALE SPEED", "Z+T"),
            ("NEXT PANE", "TAB/SHIFT+DOWN/SHIFT+RIGHT"),
        ],
    );
}

#[test]
fn help_lists_the_song_panes_pc_tracker_keys() {
    let mut app = in_the_song_pane(PC_TRACKER);

    app.apply(Action::ToggleHelp);

    assert_lists(
        &app,
        &[
            ("MOVE", "ARROWS"),
            ("JUMP", "ALT+UP/ALT+DOWN"),
            ("DIGIT", "0-F"),
            ("CLONE", "^D"),
            ("DEEP CLONE", "^SHIFT+D"),
            ("NEW UNUSED", "^SHIFT+N"),
            ("LOOP START", "^R"),
            ("LOOP BACK", "^SHIFT+R"),
            ("STOP", "^P"),
            ("CHANGE SPEED", "^SHIFT+T"),
            ("SCALE SPEED", "ALT+T"),
            ("PLAY", "SPACE"),
        ],
    );
}

#[test]
fn help_lists_the_song_panes_m8_style_keys() {
    let mut app = in_the_song_pane(M8_STYLE);

    app.apply(Action::ToggleHelp);

    assert_lists(
        &app,
        &[
            ("JUMP", "X+UP/X+DOWN"),
            ("COPY", "X"),
            ("CLONE", "Z"),
            ("DEEP CLONE", "^SHIFT+D"),
            ("NEW UNUSED", "DOUBLE-TAP Z"),
            ("LOOP START", "X+R"),
            ("LOOP BACK", "X+B"),
            ("STOP", "X+P"),
            ("SCALE SPEED", "Z+T"),
        ],
    );
}

#[test]
fn only_the_song_pane_lists_deep_clone_and_the_song_row_marks() {
    for in_the_pane in [in_the_phrase_pane, in_the_chain_pane] {
        let mut app = in_the_pane(PC_TRACKER);

        app.apply(Action::ToggleHelp);

        let names = help_names(&app);
        for name in ["DEEP CLONE", "LOOP START", "LOOP BACK", "STOP"] {
            assert!(
                !names.iter().any(|shown| shown == name),
                "{name} in {names:#?}"
            );
        }
    }
}

#[test]
fn moving_focus_with_help_open_changes_the_list_to_the_new_panes_keys() {
    for config in [PC_TRACKER, M8_STYLE] {
        let mut app = in_the_phrase_pane(config);
        app.apply(Action::ToggleHelp);
        let phrase = help_lines(&app);
        assert!(help_names(&app).contains(&"NOTE".to_owned()), "{phrase:#?}");

        app.apply(Action::FocusNext);
        let chain = help_lines(&app);
        let names = help_names(&app);
        assert!(!names.contains(&"NOTE".to_owned()), "{names:#?}");
        assert!(names.contains(&"NEW UNUSED".to_owned()), "{names:#?}");
        assert!(!names.contains(&"LOOP START".to_owned()), "{names:#?}");

        app.apply(Action::FocusNext);
        let names = help_names(&app);
        assert!(names.contains(&"LOOP START".to_owned()), "{names:#?}");

        app.apply(Action::FocusPrevious);
        assert_eq!(help_lines(&app), chain);

        app.apply(Action::FocusPrevious);
        assert_eq!(help_lines(&app), phrase);
    }
}

#[test]
fn help_shows_keys_rebound_in_config_ron_in_the_chain_and_song_panes() {
    let config = r#"(version: 1, keys: {"Clone": "Ctrl+K", "ToggleLoopStart": "Ctrl+J"})"#;
    for (in_the_pane, rebound) in [
        (in_the_chain_pane as fn(&str) -> App, vec![("CLONE", "^K")]),
        (
            in_the_song_pane,
            vec![("CLONE", "^K"), ("LOOP START", "^J")],
        ),
    ] {
        let mut app = in_the_pane(config);

        app.apply(Action::ToggleHelp);

        assert_lists(&app, &rebound);
        let lines = help_lines(&app);
        for replaced in [" ^D", " ^R"] {
            assert!(
                lines.iter().all(|line| !line.ends_with(replaced)),
                "{replaced} in {lines:#?}"
            );
        }
    }
}
