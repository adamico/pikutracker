mod support;

use pikutracker_core::app::{Action, App, Direction, KeyContext, layout};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::project::{ChainId, PhraseId};
use support::{apply, pane_text, status_line};

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const SHIFT_TAB: Action = Action::FocusPrevious;
const TAB: Action = Action::FocusNext;
const NUDGE_UP: Action = Action::Nudge {
    up: true,
    coarse: false,
};
const NUDGE_DOWN: Action = Action::Nudge {
    up: false,
    coarse: false,
};
const COARSE_DOWN: Action = Action::Nudge {
    up: false,
    coarse: true,
};

const FIRST_ROW_LINE: usize = 2;

fn app_after(actions: &[Action]) -> App {
    let mut app = App::new();
    apply(&mut app, actions);
    app
}

fn id(value: u8) -> [Action; 2] {
    [Action::HexDigit(value >> 4), Action::HexDigit(value & 0xF)]
}

fn song_text(app: &App, line: usize) -> String {
    pane_text(app, layout::SONG, line)
}

fn chain_text(app: &App, line: usize) -> String {
    pane_text(app, layout::CHAIN, line)
}

fn phrase_text(app: &App, line: usize) -> String {
    pane_text(app, layout::PHRASE, line)
}

fn chain_app() -> App {
    app_after(&[&id(3)[..], &[SHIFT_TAB]].concat())
}

#[test]
fn two_hex_digits_place_a_chain_on_a_song_row() {
    let app = app_after(&[&[DOWN, RIGHT][..], &id(0x3A)].concat());

    assert_eq!(app.project().song_cell(1, 1), Some(ChainId::new(0x3A)));
    assert_eq!(song_text(&app, FIRST_ROW_LINE + 1), "01  --[3A]-- --");
}

#[test]
fn all_four_tracks_take_chains() {
    let mut app = App::new();
    for track in 0..4 {
        apply(&mut app, &id(track as u8));
        app.apply(RIGHT);
    }

    for track in 0..4 {
        assert_eq!(
            app.project().song_cell(0, track),
            Some(ChainId::new(track as u8))
        );
    }
}

#[test]
fn clear_removes_a_chain_from_the_song_row_and_keeps_the_chain() {
    let mut app = chain_app();
    apply(&mut app, &id(5));
    apply(&mut app, &[TAB, Action::Clear]);

    assert_eq!(app.project().song_cell(0, 0), None);
    assert!(app.project().chain(ChainId::new(3)).entry(0).is_some());
}

#[test]
fn nudging_a_song_cell_steps_through_the_chains() {
    let app = app_after(&[&id(3)[..], &[NUDGE_UP, NUDGE_UP, NUDGE_DOWN]].concat());

    assert_eq!(app.project().song_cell(0, 0), Some(ChainId::new(4)));
}

#[test]
fn hex_digits_type_directly_in_the_song_and_chain_panes() {
    let keymap = Keymap::pc_tracker();
    let press = KeyPress::new(Key::Char('a'), Modifiers::NONE);

    assert_eq!(App::new().key_context(), KeyContext::ReferenceValue);
    assert_eq!(
        keymap.action_in(press, KeyContext::ReferenceValue),
        Some(Action::HexDigit(0xA))
    );
    assert_eq!(chain_app().key_context(), KeyContext::ReferenceValue);
}

#[test]
fn delete_is_bound_to_clear() {
    let keymap = Keymap::pc_tracker();

    assert_eq!(
        keymap.action_for(KeyPress::new(Key::Delete, Modifiers::NONE)),
        Some(Action::Clear)
    );
}

#[test]
fn a_chain_entry_holds_a_phrase_with_no_transpose_at_first() {
    let mut app = chain_app();
    apply(&mut app, &id(0x1F));

    let entry = app.project().chain(ChainId::new(3)).entry(0).unwrap();
    assert_eq!(entry.phrase(), PhraseId::new(0x1F));
    assert_eq!(entry.transpose(), 0);
    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00[1F]    +00");
}

#[test]
fn an_untouched_chain_entry_stays_empty() {
    let mut app = chain_app();
    apply(&mut app, &id(1));

    assert!(app.project().chain(ChainId::new(3)).entry(1).is_none());
    assert_eq!(chain_text(&app, FIRST_ROW_LINE + 1), "01 --     ---");
}

#[test]
fn the_transpose_column_nudges_by_semitones_and_octaves() {
    let mut app = chain_app();
    apply(&mut app, &id(1));
    apply(&mut app, &[RIGHT, NUDGE_UP, NUDGE_UP, COARSE_DOWN]);

    let entry = app.project().chain(ChainId::new(3)).entry(0).unwrap();
    assert_eq!(entry.transpose(), -10);
    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00 01    [-0A]");
}

#[test]
fn hex_digits_set_the_size_of_a_transpose_and_keep_its_sign() {
    let mut app = chain_app();
    apply(&mut app, &id(1));
    apply(&mut app, &[RIGHT, NUDGE_DOWN]);
    apply(&mut app, &id(0x0C));

    let entry = app.project().chain(ChainId::new(3)).entry(0).unwrap();
    assert_eq!(entry.transpose(), -12);
}

#[test]
fn a_transpose_stops_at_its_limits() {
    let mut app = chain_app();
    apply(&mut app, &id(1));
    app.apply(RIGHT);
    apply(&mut app, &[COARSE_DOWN; 20]);

    let entry = app.project().chain(ChainId::new(3)).entry(0).unwrap();
    assert_eq!(entry.transpose(), -128);
}

#[test]
fn clear_on_the_phrase_column_empties_the_whole_entry() {
    let mut app = chain_app();
    apply(&mut app, &id(1));
    apply(&mut app, &[Action::Clear]);

    assert!(app.project().chain(ChainId::new(3)).entry(0).is_none());
    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00[--]    ---");
}

#[test]
fn clear_on_the_transpose_column_resets_only_the_transpose() {
    let mut app = chain_app();
    apply(&mut app, &id(1));
    apply(&mut app, &[RIGHT, NUDGE_UP, Action::Clear]);

    let entry = app.project().chain(ChainId::new(3)).entry(0).unwrap();
    assert_eq!(entry.transpose(), 0);
    assert_eq!(entry.phrase(), PhraseId::new(1));
}

#[test]
fn the_chain_pane_has_no_fill_column() {
    let mut app = chain_app();
    apply(&mut app, &id(1));
    apply(&mut app, &[RIGHT, RIGHT, RIGHT]);

    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00 01    [+00]");
}

#[test]
fn editing_a_chain_entry_with_no_chain_under_the_cursor_says_so() {
    let app = app_after(&[&[SHIFT_TAB][..], &id(1)].concat());

    assert_eq!(status_line(&app), "NO CHAIN HERE");
    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00[--]    ---");
}

#[test]
fn editing_with_no_phrase_under_the_cursor_says_so() {
    let app = app_after(&[SHIFT_TAB, SHIFT_TAB, Action::PianoKey(0)]);

    assert_eq!(status_line(&app), "NO PHRASE HERE");
    assert_eq!(app.shown_phrase_id(), None);
}

#[test]
fn the_chain_pane_follows_the_song_cursor() {
    let mut app = chain_app();
    apply(&mut app, &id(7));
    apply(&mut app, &[TAB, DOWN]);
    assert_eq!(chain_text(&app, 0), "CHAIN --");
    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00 --     ---");

    app.apply(UP);
    assert_eq!(chain_text(&app, 0), "CHAIN 03");
    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00 07     +00");
}

#[test]
fn the_phrase_pane_follows_the_chain_cursor() {
    let mut app = chain_app();
    apply(&mut app, &id(7));
    apply(&mut app, &[DOWN]);
    apply(&mut app, &id(9));

    assert_eq!(app.shown_phrase_id(), Some(PhraseId::new(9)));
    assert_eq!(phrase_text(&app, 0), "PHRASE 09");

    app.apply(UP);
    assert_eq!(app.shown_phrase_id(), Some(PhraseId::new(7)));
    assert_eq!(phrase_text(&app, 0), "PHRASE 07");
}

#[test]
fn the_phrase_pane_follows_the_song_cursor_through_the_chain() {
    let mut app = chain_app();
    apply(&mut app, &id(7));
    app.apply(TAB);
    apply(&mut app, &[DOWN]);
    assert_eq!(phrase_text(&app, 0), "PHRASE --");

    app.apply(UP);
    assert_eq!(phrase_text(&app, 0), "PHRASE 07");
}

#[test]
fn a_note_goes_into_the_phrase_under_the_chain_cursor() {
    let mut app = chain_app();
    apply(&mut app, &id(7));
    apply(&mut app, &[SHIFT_TAB, Action::PianoKey(0)]);

    let written = app.project().phrase(PhraseId::new(7)).step(0);
    assert_eq!(written.pitch(), 24);
    assert!(app.project().phrase(PhraseId::new(0)).is_silent());
}

#[test]
fn two_entries_sharing_a_phrase_show_the_same_notes() {
    let mut app = chain_app();
    apply(&mut app, &id(7));
    app.apply(DOWN);
    apply(&mut app, &id(7));
    apply(&mut app, &[SHIFT_TAB, Action::PianoKey(0), TAB, UP]);

    assert_eq!(phrase_text(&app, 7), "00 C-2  2    5   0");
}

#[test]
fn the_phrase_cursor_keeps_its_step_past_the_length_of_a_shorter_phrase_it_follows_to() {
    let mut app = chain_app();
    apply(&mut app, &id(1));
    app.apply(DOWN);
    apply(&mut app, &id(2));
    apply(
        &mut app,
        &[
            SHIFT_TAB,
            UP,
            UP,
            UP,
            UP,
            UP,
            RIGHT,
            Action::HexDigit(0),
            Action::HexDigit(2),
        ],
    );
    apply(
        &mut app,
        &[LEFT, DOWN, DOWN, DOWN, DOWN, DOWN, TAB, UP, SHIFT_TAB],
    );
    apply(&mut app, &[DOWN; 15]);
    apply(&mut app, &[TAB, DOWN, SHIFT_TAB]);

    assert_eq!(phrase_text(&app, 22), "0F[...] .    .   .");
}

#[test]
fn the_song_editing_keys_are_bound_in_the_pc_tracker_preset() {
    let keymap = Keymap::pc_tracker();

    assert_eq!(
        keymap.action_for(KeyPress::new(Key::Char('f'), Modifiers::CTRL)),
        None
    );
    assert_eq!(
        keymap.action_for(KeyPress::new(Key::Char('r'), Modifiers::CTRL)),
        Some(Action::ToggleLoopStart)
    );
}

#[test]
fn left_moves_back_to_the_phrase_column_of_a_chain_entry() {
    let mut app = chain_app();
    apply(&mut app, &[RIGHT, RIGHT, LEFT, LEFT]);

    assert_eq!(chain_text(&app, FIRST_ROW_LINE), "00[--]    ---");
}
