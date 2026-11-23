mod support;

use pikutracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use pikutracker_core::player::Player;
use pikutracker_core::project::{
    ChainEntry, ChainId, Claim, CustomInstrument, InstrumentSound, PhraseId, Project, TargetCart,
    WAVEFORM_SAMPLES, WaveformInstrument,
};
use pikutracker_core::project_file;
use pikutracker_core::synth::SAMPLES_PER_TICK;
use std::path::PathBuf;
use support::{apply, status_line};

const DOWN: Action = Action::Move(Direction::Down);
const UP: Action = Action::Move(Direction::Up);
const NUDGE_UP: Action = Action::Nudge {
    up: true,
    coarse: false,
};
const NUDGE_DOWN: Action = Action::Nudge {
    up: false,
    coarse: false,
};
const NOTHING_TO_UNDO: &str = "NOTHING TO UNDO";
const NOTHING_TO_REDO: &str = "NOTHING TO REDO";

fn song_cell(app: &App, row: usize, track: usize) -> Option<u8> {
    app.project().song_cell(row, track).map(ChainId::number)
}

fn project_with_a_song() -> Project {
    let mut project = Project::new();
    let chain = ChainId::new(0);
    *project.chain_mut(chain).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    project.set_song_cell(0, 0, Some(chain));
    project
}

fn save(app: &mut App) -> String {
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: Vec::new(),
    });
    for ch in "demo.ptrk".chars() {
        app.apply(Action::TypeChar(ch));
    }
    app.apply(Action::Confirm);
    let mut requests = app.take_file_requests();
    let Some(FileRequest::Write { path, contents }) = requests.pop() else {
        panic!("expected a write");
    };
    app.receive_file(FileResponse::Written { path });
    contents
}

fn open(app: &mut App, project: &Project) {
    app.apply(Action::OpenProject);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: "other.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let mut requests = app.take_file_requests();
    let Some(FileRequest::Read { path }) = requests.pop() else {
        panic!("expected a read");
    };
    app.receive_file(FileResponse::Read {
        path,
        contents: project_file::write(project),
    });
}

#[test]
fn undoing_an_edit_keeps_the_claim_and_the_custom_instruments_of_an_opened_project() {
    let mut project = project_with_a_song();
    project.set_target_cart(Some(TargetCart {
        path: "game.p8".to_owned(),
        claim: Claim {
            sfx: vec![1, 2],
            first_and_last_pattern: Some((0, 3)),
        },
        claimed_bytes_hash: Some("aa".to_owned()),
    }));
    project.set_custom_instrument(
        8,
        Some(CustomInstrument {
            name: Some("BASS".to_owned()),
            sound: InstrumentSound::Waveform(WaveformInstrument {
                samples: [0; WAVEFORM_SAMPLES],
                bass: false,
                detune: 0,
                reverb: 0,
                dampen: 0,
            }),
        }),
    );
    let mut app = App::new();
    open(&mut app, &project);

    apply(&mut app, &[Action::HexDigit(1), Action::Undo]);

    assert!(*app.project() == project);
}

#[test]
fn undo_takes_back_a_song_edit_and_redo_puts_it_back() {
    let mut app = App::new();
    apply(&mut app, &[Action::HexDigit(3)]);
    let edited = app.project().clone();

    app.apply(Action::Undo);
    assert!(*app.project() == Project::new());

    app.apply(Action::Redo);
    assert!(*app.project() == edited);
}

#[test]
fn undo_moves_the_cursor_back_to_the_change() {
    let mut app = App::new();
    apply(&mut app, &[DOWN, Action::HexDigit(3), DOWN, DOWN]);

    app.apply(Action::Undo);
    app.apply(Action::HexDigit(7));

    assert_eq!(song_cell(&app, 1, 0), Some(7));
    assert_eq!(song_cell(&app, 3, 0), None);
}

#[test]
fn undo_brings_focus_back_to_the_pane_of_the_change() {
    let mut app = App::with_project(project_with_a_song());
    apply(&mut app, &[Action::HexDigit(5), Action::FocusNext]);

    app.apply(Action::Undo);
    app.apply(Action::HexDigit(6));

    assert_eq!(song_cell(&app, 0, 0), Some(6));
}

#[test]
fn edits_in_every_pane_are_undone_in_order() {
    let mut app = App::with_project(project_with_a_song());
    let start = app.project().clone();
    apply(
        &mut app,
        &[
            Action::ToggleLoopStart,
            Action::FocusNext,
            NUDGE_UP,
            Action::FocusNext,
            Action::PianoKey(0),
            UP,
            UP,
            NUDGE_UP,
        ],
    );

    apply(&mut app, &[Action::Undo; 4]);

    assert!(*app.project() == start);
    app.apply(Action::Undo);
    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}

#[test]
fn repeated_nudges_on_one_cell_are_one_step() {
    let mut app = App::with_project(project_with_a_song());
    let start = app.project().clone();
    apply(&mut app, &[NUDGE_UP, NUDGE_UP, NUDGE_UP]);

    app.apply(Action::Undo);

    assert!(*app.project() == start);
}

#[test]
fn typed_digits_on_one_cell_are_one_step() {
    let mut app = App::new();
    apply(&mut app, &[Action::HexDigit(1), Action::HexDigit(2)]);
    assert_eq!(song_cell(&app, 0, 0), Some(0x12));

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), None);
}

#[test]
fn moving_the_cursor_ends_a_merge() {
    let mut app = App::with_project(project_with_a_song());
    apply(&mut app, &[NUDGE_UP, DOWN, UP, NUDGE_UP]);

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
}

#[test]
fn another_action_ends_a_merge() {
    let mut app = App::with_project(project_with_a_song());
    apply(&mut app, &[NUDGE_UP, Action::OctaveUp, NUDGE_UP]);

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
}

#[test]
fn a_cursor_moved_by_follow_ends_a_merge() {
    let mut project = project_with_a_song();
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_speed(8);
    for index in 0..4 {
        phrase.step_mut(index).set_volume(5);
    }
    let mut app = App::with_project(project);
    let mut player = Player::new();
    apply(
        &mut app,
        &[
            Action::FocusNext,
            Action::FocusNext,
            Action::ToggleFollow,
            Action::PlaySongFromCursor,
            NUDGE_UP,
        ],
    );
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(12 * SAMPLES_PER_TICK as usize);
    app.show_playback(&player.report());
    app.apply(NUDGE_UP);

    app.apply(Action::Undo);

    let phrase = app.project().phrase(PhraseId::new(0));
    assert_eq!(phrase.step(0).pitch(), 25);
    assert_eq!(phrase.step(1).pitch(), 24);
}

#[test]
fn a_nudge_and_a_typed_digit_are_separate_steps() {
    let mut app = App::with_project(project_with_a_song());
    apply(&mut app, &[NUDGE_UP, Action::HexDigit(9)]);

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
}

#[test]
fn nudges_that_cancel_out_leave_no_step() {
    let mut app = App::with_project(project_with_a_song());
    apply(&mut app, &[NUDGE_UP, NUDGE_DOWN]);

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}

#[test]
fn a_new_edit_after_undo_drops_what_could_be_redone() {
    let mut app = App::new();
    apply(&mut app, &[Action::HexDigit(1), Action::Undo, DOWN]);
    app.apply(Action::HexDigit(2));

    app.apply(Action::Redo);

    assert_eq!(status_line(&app), NOTHING_TO_REDO);
    assert_eq!(song_cell(&app, 0, 0), None);
}

#[test]
fn actions_that_change_no_music_are_not_steps() {
    let mut app = App::with_project(project_with_a_song());
    apply(
        &mut app,
        &[
            DOWN,
            Action::FocusNext,
            Action::OctaveUp,
            Action::Mute { track_number: 1 },
            Action::Solo { track_number: 2 },
            Action::ToggleFollow,
        ],
    );

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}

#[test]
fn undo_leaves_mute_and_solo_alone() {
    let mut app = App::with_project(project_with_a_song());
    apply(
        &mut app,
        &[Action::HexDigit(1), Action::Mute { track_number: 1 }],
    );
    let muted = app.grid().text_dump().lines().nth(1).unwrap().to_owned();

    app.apply(Action::Undo);

    let after = app.grid().text_dump().lines().nth(1).unwrap().to_owned();
    assert_eq!(after[..28], muted[..28]);
}

#[test]
fn undo_turns_follow_off_because_it_moves_the_cursor() {
    let mut app = App::with_project(project_with_a_song());
    apply(&mut app, &[Action::HexDigit(1), Action::ToggleFollow]);

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), "FOLLOW OFF");
}

#[test]
fn the_history_keeps_the_last_thousand_steps() {
    let mut app = App::new();
    apply(&mut app, &[Action::ToggleLoopStart; 1001]);
    assert!(app.project().song_row_marks(0).loop_start);

    apply(&mut app, &[Action::Undo; 1000]);
    assert!(app.project().song_row_marks(0).loop_start);

    app.apply(Action::Undo);
    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
    assert!(app.project().song_row_marks(0).loop_start);
}

#[test]
fn the_history_survives_save_and_stays_out_of_the_file() {
    let mut app = App::new();
    app.apply(Action::HexDigit(4));

    let contents = save(&mut app);

    assert_eq!(contents, project_file::write(app.project()));
    app.apply(Action::Undo);
    assert_eq!(song_cell(&app, 0, 0), None);
}

#[test]
fn undoing_back_to_the_saved_music_clears_the_unsaved_mark() {
    let mut app = App::new();
    save(&mut app);
    app.apply(Action::HexDigit(4));
    assert!(app.has_unsaved_changes());

    app.apply(Action::Undo);

    assert!(!app.has_unsaved_changes());
}

#[test]
fn the_history_starts_empty_when_another_project_opens() {
    let mut app = App::new();
    app.apply(Action::HexDigit(4));
    save(&mut app);

    open(&mut app, &project_with_a_song());
    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
    assert!(*app.project() == project_with_a_song());
}

#[test]
fn the_history_starts_empty_for_a_new_project() {
    let mut app = App::new();
    app.apply(Action::HexDigit(4));
    save(&mut app);

    app.apply(Action::NewProject);
    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}
