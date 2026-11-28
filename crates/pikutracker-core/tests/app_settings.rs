mod midi_support;
mod support;

use midi_support::{Song, TICKS_PER_STEP};
use pikutracker_core::app::{
    Action, App, Direction, FileRequest, FileResponse, Launch, SettingsContents, layout,
};
use pikutracker_core::keymap::{Key, KeyPress, Modifiers};
use pikutracker_core::pen::Pen;
use pikutracker_core::settings::{AppState, SettingsRead, read_state, write_state};
use std::path::{Path, PathBuf};
use std::time::Duration;
use support::{pane_text, status_line};

const CONFIG_FILE: &str = "/config/pikutracker/config.ron";
const STATE_FILE: &str = "/state/pikutracker/state.ron";
const NONE: Modifiers = Modifiers::NONE;
const MIDDLE_C: u8 = 60;

fn contents(text: Option<&str>) -> SettingsContents {
    text.map_or(SettingsContents::Missing, |text| {
        SettingsContents::Bytes(text.as_bytes().to_vec())
    })
}

fn launch(config: Option<&str>, state: Option<&str>) -> App {
    let mut app = App::new();
    app.launch(Launch {
        config_file: Some(PathBuf::from(CONFIG_FILE)),
        config: contents(config),
        state_file: Some(PathBuf::from(STATE_FILE)),
        state: contents(state),
        directory: PathBuf::from("/home/user"),
    });
    app
}

fn keys(app: &mut App, presses: &[(Key, Modifiers)]) -> Vec<Action> {
    let mut actions = Vec::new();
    for (second, &(key, modifiers)) in (1..).zip(presses) {
        let at = Duration::from_secs(second);
        actions.extend(app.key_down(KeyPress::new(key, modifiers), at));
        actions.extend(app.key_up(key));
    }
    for &action in &actions {
        app.apply(action);
    }
    actions
}

fn chord(app: &mut App, held: Key, key: Key) -> Vec<Action> {
    let mut actions = Vec::new();
    actions.extend(app.key_down(KeyPress::new(held, NONE), Duration::ZERO));
    actions.extend(app.key_down(KeyPress::new(key, NONE), Duration::from_millis(10)));
    actions.extend(app.key_up(key));
    actions.extend(app.key_up(held));
    for &action in &actions {
        app.apply(action);
    }
    actions
}

fn state_written(requests: &[FileRequest]) -> Option<AppState> {
    requests.iter().find_map(|request| match request {
        FileRequest::Write { path, contents } if path == Path::new(STATE_FILE) => {
            match read_state(contents) {
                SettingsRead::Loaded(state) => Some(state),
                other => panic!("the app wrote an unreadable state: {other:?}"),
            }
        }
        _ => None,
    })
}

fn never_writes_config(requests: &[FileRequest]) {
    for request in requests {
        if let FileRequest::Write { path, .. } | FileRequest::Rename { from: path, .. } = request {
            assert_ne!(path, Path::new(CONFIG_FILE), "{request:?}");
        }
    }
}

const M8_STYLE: &str = "(version: 1, preset: M8Style)";

#[test]
fn with_no_config_file_the_pc_tracker_keys_apply() {
    let mut app = launch(None, None);

    assert_eq!(
        keys(&mut app, &[(Key::Char('t'), Modifiers::CTRL)]),
        [Action::ToggleCart]
    );
    assert!(app.take_file_requests().is_empty());
}

#[test]
fn the_m8_style_preset_is_chosen_in_config_ron() {
    let mut app = launch(Some(M8_STYLE), None);

    assert_eq!(keys(&mut app, &[(Key::Char('t'), Modifiers::CTRL)]), []);
    assert_eq!(
        chord(&mut app, Key::Char('x'), Key::Char('t')),
        [Action::ToggleCart]
    );
    assert!(app.grid().text_dump().contains("CART"));
}

#[test]
fn a_changed_key_replaces_the_presets_key_and_the_rest_stay() {
    let mut app = launch(
        Some(r#"(version: 1, preset: PcTracker, keys: {"ToggleCart": "Ctrl+K"})"#),
        None,
    );

    assert_eq!(keys(&mut app, &[(Key::Char('t'), Modifiers::CTRL)]), []);
    assert_eq!(
        keys(&mut app, &[(Key::Char('k'), Modifiers::CTRL)]),
        [Action::ToggleCart]
    );
    assert_eq!(
        keys(&mut app, &[(Key::Char('l'), Modifiers::CTRL)]),
        [Action::ToggleLog]
    );
}

#[test]
fn a_changed_key_can_be_a_chord_or_a_double_tap() {
    let mut app = launch(
        Some(r#"(version: 1, keys: {"ToggleLog": "Double-tap F12", "ToggleCart": "Tab+C"})"#),
        None,
    );

    let tap = |app: &mut App, at| {
        let action = app.key_down(KeyPress::new(Key::F(12), NONE), Duration::from_millis(at));
        app.key_up(Key::F(12));
        action
    };
    assert_eq!(tap(&mut app, 0), None);
    assert_eq!(tap(&mut app, 120), Some(Action::ToggleLog));
    assert_eq!(
        chord(&mut app, Key::Tab, Key::Char('c')),
        [Action::ToggleCart],
        "Tab starts a chord now, so it acts only when let go alone"
    );
}

#[test]
fn keys_that_cannot_be_used_are_ignored_and_the_app_says_which() {
    let mut app = launch(
        Some(
            r#"(version: 1, keys: {"Sav": "Ctrl+W", "Export": "Ctrl+Hyper", "ToggleCart": "Ctrl+K"})"#,
        ),
        None,
    );

    assert!(status_line(&app).starts_with("! CONFIG.RON: IGNORED 2 KEYS"));
    let log = app.grid().text_dump();
    assert!(log.contains("NO ACTION IS"), "{log}");
    assert!(log.contains("CANNOT READ THE"), "{log}");
    assert_eq!(
        keys(&mut app, &[(Key::Char('k'), Modifiers::CTRL)]),
        [Action::ToggleCart]
    );
}

#[test]
fn a_damaged_config_file_is_renamed_and_the_default_keys_apply() {
    for damaged in [
        "(version: 1, preset: Nope)",
        "(version: 1, keys: [",
        "nonsense",
    ] {
        let mut app = launch(Some(damaged), None);

        assert_eq!(
            app.take_file_requests(),
            [FileRequest::Rename {
                from: PathBuf::from(CONFIG_FILE),
                to: PathBuf::from(format!("{CONFIG_FILE}.bad")),
            }],
            "{damaged}"
        );
        assert!(
            status_line(&app)
                .starts_with("! CONFIG.RON WAS DAMAGED: RENAMED TO CONFIG.RON.BAD, USING DEFAULTS"),
            "{damaged}"
        );
        assert_eq!(
            keys(&mut app, &[(Key::Char('t'), Modifiers::CTRL)]),
            [Action::ToggleCart]
        );
    }
}

#[test]
fn a_damaged_config_file_that_cannot_be_renamed_says_so() {
    let mut app = launch(Some("nonsense"), None);
    app.take_file_requests();

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from(CONFIG_FILE),
        reason: "Read-only file system".to_owned(),
    });

    assert!(
        status_line(&app)
            .starts_with("! CANNOT RENAME CONFIG.RON TO CONFIG.RON.BAD: READ-ONLY FILE SYSTEM")
    );
}

#[test]
fn damaged_config_and_state_files_are_each_set_aside_whatever_order_the_answers_come_in() {
    let mut app = launch(Some("nonsense"), Some("nonsense"));
    let renames = app.take_file_requests();
    assert_eq!(renames.len(), 2, "{renames:?}");

    app.receive_file(FileResponse::Renamed {
        path: PathBuf::from(format!("{STATE_FILE}.bad")),
    });
    app.receive_file(FileResponse::Failed {
        path: PathBuf::from(CONFIG_FILE),
        reason: "Read-only file system".to_owned(),
    });

    assert!(
        status_line(&app)
            .starts_with("! CANNOT RENAME CONFIG.RON TO CONFIG.RON.BAD: READ-ONLY FILE SYSTEM")
    );
    app.apply(Action::ToggleFollow);
    assert!(
        state_written(&app.take_file_requests()).is_some(),
        "state.ron was set aside, so a new one is written"
    );
}

#[test]
fn a_config_file_from_a_newer_version_is_left_alone_and_the_defaults_apply() {
    let mut app = launch(Some("(version: 9, preset: M8Style, shiny: true)"), None);

    assert!(app.take_file_requests().is_empty());
    assert!(
        status_line(&app)
            .starts_with("! CONFIG.RON IS FROM A NEWER PIKUTRACKER: USING DEFAULTS THIS SESSION")
    );
    assert_eq!(
        keys(&mut app, &[(Key::Char('t'), Modifiers::CTRL)]),
        [Action::ToggleCart]
    );
}

#[test]
fn a_config_file_that_cannot_be_read_leaves_the_defaults() {
    let mut app = App::new();
    app.launch(Launch {
        config_file: Some(PathBuf::from(CONFIG_FILE)),
        config: SettingsContents::Unreadable {
            reason: "Permission denied".to_owned(),
        },
        state_file: None,
        state: SettingsContents::Missing,
        directory: PathBuf::from("/home/user"),
    });

    assert!(
        status_line(&app).starts_with(
            "! CANNOT READ CONFIG.RON: PERMISSION DENIED. USING DEFAULTS THIS SESSION"
        )
    );
}

#[test]
fn follow_and_the_octave_are_remembered_and_config_ron_is_never_written() {
    let mut app = launch(Some(M8_STYLE), None);

    app.apply(Action::ToggleFollow);
    let follow = state_written(&app.take_file_requests()).expect("Follow is saved");
    app.apply(Action::OctaveUp);
    let octave = state_written(&app.take_file_requests()).expect("the octave is saved");

    assert!(follow.follow);
    assert_eq!((octave.follow, octave.pen.octave()), (true, 3));
    let mut next = launch(
        Some(M8_STYLE),
        Some(&pikutracker_core::settings::write_state(&octave)),
    );
    next.apply(Action::Move(pikutracker_core::app::Direction::Right));
    assert!(
        status_line(&next).starts_with("FOLLOW OFF"),
        "Follow was on"
    );
    never_writes_config(&next.take_file_requests());
}

#[test]
fn a_hand_move_turns_follow_off_for_this_session_only() {
    let mut app = launch(None, Some("(version: 1, follow: true, octave: 3)"));
    app.take_file_requests();

    app.apply(Action::Move(Direction::Right));
    assert!(status_line(&app).starts_with("FOLLOW OFF"), "Follow was on");
    app.apply(Action::OctaveUp);
    let state = state_written(&app.take_file_requests()).expect("the octave is saved");

    assert!(state.follow, "the Follow key last turned Follow on");
}

#[test]
fn a_hand_move_alone_writes_nothing() {
    let mut app = launch(None, Some("(version: 1, follow: true, octave: 3)"));
    app.take_file_requests();

    app.apply(Action::Move(Direction::Right));

    assert_eq!(state_written(&app.take_file_requests()), None);
}

#[test]
fn the_follow_key_after_a_hand_move_sets_what_is_remembered() {
    let mut app = launch(None, Some("(version: 1, follow: true, octave: 3)"));
    app.take_file_requests();
    app.apply(Action::Move(Direction::Right));

    app.apply(Action::ToggleFollow);
    assert!(status_line(&app).starts_with("FOLLOW ON"), "Follow was off");
    app.apply(Action::ToggleFollow);
    let state = state_written(&app.take_file_requests()).expect("Follow is saved");

    assert!(!state.follow, "the Follow key last turned Follow off");
}

#[test]
fn the_whole_pen_is_remembered_and_restored_on_launch() {
    let mut app = launch(None, None);
    for action in [
        Action::FocusPrevious,
        Action::FocusPrevious,
        Action::Move(Direction::Up),
        Action::HexDigit(7),
        Action::Move(Direction::Right),
        Action::HexDigit(3),
        Action::Move(Direction::Up),
    ] {
        app.apply(action);
    }
    app.take_file_requests();
    app.apply(Action::HexDigit(9));
    let state = state_written(&app.take_file_requests()).expect("the Pen is saved");

    assert_eq!(state.pen, Pen::new(2, 9, 7, 3).unwrap());
    let next = launch(None, Some(&write_state(&state)));
    assert_eq!(pane_text(&next, layout::PHRASE, 3), "OCT 2  INS 9");
    assert_eq!(pane_text(&next, layout::PHRASE, 4), "VOL 7  FX  3 DROP");
}

#[test]
fn the_remembered_octave_is_where_piano_keys_play() {
    let app = launch(None, Some("(version: 1, octave: 4)"));

    assert!(app.grid().text_dump().contains("OCT 4"));
}

#[test]
fn actions_that_change_nothing_remembered_write_nothing() {
    let mut app = launch(None, Some("(version: 1, follow: true, octave: 3)"));
    app.take_file_requests();

    app.apply(Action::FocusPrevious);
    app.apply(Action::OctaveUp);
    app.apply(Action::OctaveDown);
    app.take_file_requests();
    app.apply(Action::HexDigit(1));

    assert_eq!(state_written(&app.take_file_requests()), None);
}

#[test]
fn a_state_file_with_a_value_out_of_range_is_damaged() {
    for damaged in [
        "(version: 1, octave: 9)",
        "(version: 1, instrument: 16)",
        "(version: 1, volume: 8)",
        "(version: 1, effect: 8)",
        "(version: 1, midi_programs: {0: 8})",
        "(version: 1, midi_programs: {128: 0})",
        "(version: 1, drum_kit: {5: (instrument: 0, note: 0, effect: 0)})",
        "(version: 1, drum_kit: {0: (instrument: 0, note: 64, effect: 0)})",
    ] {
        assert_eq!(read_state(damaged), SettingsRead::Damaged, "{damaged}");
    }
}

fn melody() -> Song {
    Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127)
}

fn first_step_instrument(app: &App) -> u8 {
    let project = app.project();
    let entry = project.chain_row_entry(0, 0, 0).unwrap();
    project.phrase(entry.phrase()).step(0).instrument()
}

fn to_instruments(app: &mut App) {
    app.start_midi_import("song.mid", &melody().bytes());
    app.apply(Action::Confirm);
    app.apply(Action::Confirm);
}

fn finish_import(app: &mut App) {
    app.apply(Action::Confirm);
    app.apply(Action::Confirm);
}

#[test]
fn midi_table_edits_become_the_defaults_when_the_user_imports() {
    let mut app = launch(None, None);
    to_instruments(&mut app);
    app.apply(Action::Move(pikutracker_core::app::Direction::Right));
    assert_eq!(
        state_written(&app.take_file_requests()),
        None,
        "not before Import"
    );

    finish_import(&mut app);

    let state = state_written(&app.take_file_requests()).expect("the table is saved");
    assert_eq!(state.midi_table.instrument(0), 2);
    assert_eq!(state.midi_table.edited_programs().collect::<Vec<_>>(), [0]);
    to_instruments(&mut app);
    finish_import(&mut app);
    assert_eq!(
        first_step_instrument(&app),
        2,
        "the edit is now the default"
    );
}

#[test]
fn a_cancelled_dialog_leaves_the_defaults_as_they_were() {
    let mut app = launch(None, None);
    to_instruments(&mut app);
    app.apply(Action::Move(pikutracker_core::app::Direction::Right));

    for _ in 0..3 {
        app.apply(Action::Cancel);
    }

    assert_eq!(state_written(&app.take_file_requests()), None);
    to_instruments(&mut app);
    finish_import(&mut app);
    assert_eq!(first_step_instrument(&app), 1);
}

#[test]
fn remembered_midi_table_rows_start_the_dialog_and_shift_r_restores_the_factory_table() {
    let state =
        "(version: 1, midi_programs: {0: 3}, drum_kit: {0: (instrument: 6, note: 20, effect: 0)})";
    let mut app = launch(None, Some(state));
    to_instruments(&mut app);
    assert!(app.grid().text_dump().contains("SQUARE *"));

    keys(&mut app, &[(Key::Char('R'), Modifiers::SHIFT)]);
    finish_import(&mut app);

    assert_eq!(first_step_instrument(&app), 1);
    let state = state_written(&app.take_file_requests()).expect("the table is saved");
    assert!(!state.midi_table.is_edited());
}

#[test]
fn the_m8_style_preset_drives_the_midi_import_dialog() {
    let mut app = launch(Some(M8_STYLE), None);
    to_instruments(&mut app);
    keys(&mut app, &[(Key::Right, NONE)]);
    assert!(app.grid().text_dump().contains("SAW *"));

    assert_eq!(
        chord(&mut app, Key::Char('z'), Key::Char('x')),
        [Action::ResetMidiRow]
    );
    assert!(!app.grid().text_dump().contains('*'));
    keys(&mut app, &[(Key::Tab, NONE)]);
    assert_eq!(
        chord(&mut app, Key::Char('z'), Key::Up),
        [Action::MidiDrumNote { up: true }]
    );
    keys(&mut app, &[(Key::Enter, NONE), (Key::Enter, NONE)]);
    assert!(app.project().chain_row_entry(0, 0, 0).is_some());
}

#[test]
fn the_m8_style_preset_drives_the_cart_view() {
    let mut app = launch(Some(M8_STYLE), None);

    chord(&mut app, Key::Char('x'), Key::Char('t'));
    assert!(app.grid().text_dump().contains("NO TARGET CART"));
    keys(&mut app, &[(Key::Enter, NONE)]);
    assert!(matches!(
        &app.take_file_requests()[..],
        [FileRequest::List { .. }]
    ));
    keys(&mut app, &[(Key::Escape, NONE)]);
    chord(&mut app, Key::Char('x'), Key::Char('t'));

    assert!(!app.grid().text_dump().contains("NO TARGET CART"));
}

#[test]
fn the_m8_style_preset_drives_the_instrument_view() {
    use pikutracker_core::project::Project;
    let mut app = App::with_project(Project::new());
    app.launch(Launch {
        config_file: Some(PathBuf::from(CONFIG_FILE)),
        config: contents(Some(M8_STYLE)),
        state_file: None,
        state: SettingsContents::Missing,
        directory: PathBuf::from("/home/user"),
    });
    keys(&mut app, &[(Key::Char('0'), NONE), (Key::Char('0'), NONE)]);
    keys(&mut app, &[(Key::Left, Modifiers::SHIFT)]);
    keys(&mut app, &[(Key::Char('0'), NONE), (Key::Char('0'), NONE)]);
    keys(&mut app, &[(Key::Left, Modifiers::SHIFT)]);
    keys(&mut app, &[(Key::Char('q'), NONE), (Key::Right, NONE)]);
    keys(&mut app, &[(Key::Char('0'), NONE), (Key::Char('8'), NONE)]);
    keys(&mut app, &[(Key::Enter, NONE)]);
    assert!(app.grid().text_dump().contains("INSTRUMENT"));
    keys(&mut app, &[(Key::Down, NONE)]);

    assert_eq!(
        chord(&mut app, Key::Char('z'), Key::Right),
        [Action::Nudge {
            up: true,
            coarse: false
        }]
    );
    keys(&mut app, &[(Key::Escape, NONE)]);

    assert!(app.project().custom_instrument(8).is_some());
}

#[test]
fn the_m8_style_preset_keeps_a_selection_growing_with_plain_arrows() {
    use pikutracker_core::app::Direction;
    let mut app = launch(Some(M8_STYLE), None);

    let presses = [
        (Key::Char('x'), Modifiers::SHIFT),
        (Key::Down, NONE),
        (Key::Right, NONE),
    ];
    let actions: Vec<Action> = presses
        .iter()
        .flat_map(|&press| keys(&mut app, &[press]))
        .collect();

    assert_eq!(
        actions,
        [
            Action::StartSelection,
            Action::Select(Direction::Down),
            Action::Select(Direction::Right),
        ]
    );
}

#[test]
fn the_pc_tracker_preset_drops_a_selection_on_a_plain_arrow() {
    use pikutracker_core::app::Direction;
    let mut app = launch(None, None);

    let presses = [
        (Key::Down, Modifiers::SHIFT),
        (Key::Down, NONE),
        (Key::Down, NONE),
    ];
    let actions: Vec<Action> = presses
        .iter()
        .flat_map(|&press| keys(&mut app, &[press]))
        .collect();

    assert_eq!(
        actions,
        [
            Action::Select(Direction::Down),
            Action::Move(Direction::Down),
            Action::Move(Direction::Down),
        ]
    );
}

#[test]
fn help_is_not_remembered_and_starts_closed() {
    let mut app = launch(None, None);
    app.apply(Action::ToggleHelp);
    assert_eq!(pane_text(&app, layout::RIGHT_SLOT, 0), "HELP");

    app.apply(Action::ToggleFollow);
    let written = app
        .take_file_requests()
        .into_iter()
        .find_map(|request| match request {
            FileRequest::Write { path, contents } if path == Path::new(STATE_FILE) => {
                Some(contents)
            }
            _ => None,
        });
    let state = written.expect("Follow is saved");

    assert!(!state.to_lowercase().contains("help"), "{state}");
    let next = launch(None, Some(&state));
    assert_ne!(pane_text(&next, layout::RIGHT_SLOT, 0), "HELP");
}
