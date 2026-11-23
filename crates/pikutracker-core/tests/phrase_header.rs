mod support;

use pikutracker_core::app::{Action, App, Direction, layout};
use pikutracker_core::project::Phrase;
use support::pane_text;

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const NUDGE_UP: Action = Action::Nudge {
    up: true,
    coarse: false,
};
const NUDGE_DOWN: Action = Action::Nudge {
    up: false,
    coarse: false,
};

const SPEED_LINE: usize = 1;
const NOIZ_LINE: usize = 2;
const FILTER_LINE: usize = 3;
const STEP_HEADING_LINE: usize = 4;
const FIRST_STEP_LINE: usize = 5;

fn phrase_app(actions: &[Action]) -> App {
    let mut app = App::new();
    let chain_00_holding_phrase_00 = [
        Action::HexDigit(0),
        Action::FocusNext,
        Action::HexDigit(0),
        Action::FocusNext,
    ];
    for action in chain_00_holding_phrase_00 {
        app.apply(action);
    }
    for action in actions {
        app.apply(*action);
    }
    app
}

fn phrase(app: &App) -> &Phrase {
    app.project().phrase(app.shown_phrase().unwrap())
}

fn phrase_line(app: &App, line: usize) -> String {
    pane_text(app, layout::PHRASE, line)
}

const LOOP_START: [Action; 4] = [UP, UP, UP, RIGHT];
const LOOP_END: [Action; 5] = [UP, UP, UP, RIGHT, RIGHT];

fn typed(cell: &[Action], value: u8) -> Vec<Action> {
    [
        cell,
        &[Action::HexDigit(value >> 4), Action::HexDigit(value & 0xF)],
    ]
    .concat()
}

fn with_loop(start: u8, end: u8) -> App {
    let mut actions = typed(&LOOP_END, end);
    actions.extend([
        LEFT,
        Action::HexDigit(start >> 4),
        Action::HexDigit(start & 0xF),
    ]);
    phrase_app(&actions)
}

#[test]
fn a_new_phrase_has_16_steps_at_speed_16_and_loop_10_00() {
    let app = phrase_app(&[]);

    assert_eq!(phrase(&app).length(), 16);
    assert_eq!(phrase(&app).speed(), 16);
    assert_eq!(
        (phrase(&app).loop_start(), phrase(&app).loop_end()),
        (16, 0)
    );
    assert_eq!(phrase(&app).phrase_loop(), None);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD 10  LOOP 10 00  BPM 113");
    assert_eq!(phrase_line(&app, NOIZ_LINE), "NOIZ 0  BUZZ 0");
    assert_eq!(
        phrase_line(&app, FILTER_LINE),
        "DETUNE 0  REVERB 0  DAMPEN 0"
    );
    assert_eq!(phrase_line(&app, STEP_HEADING_LINE), "   NOTE INST VOL FX");
}

#[test]
fn the_cursor_moves_up_from_the_first_step_into_the_header() {
    let app = phrase_app(&[UP]);
    assert_eq!(
        phrase_line(&app, FILTER_LINE),
        "DETUNE[0] REVERB 0  DAMPEN 0"
    );

    let app = phrase_app(&[UP, UP]);
    assert_eq!(phrase_line(&app, NOIZ_LINE), "NOIZ[0] BUZZ 0");

    let app = phrase_app(&[UP, UP, UP]);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD[10] LOOP 10 00  BPM 113");
}

#[test]
fn the_loop_pair_sits_beside_the_speed() {
    let app = phrase_app(&LOOP_START);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD 10  LOOP[10]00  BPM 113");

    let app = phrase_app(&LOOP_END);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD 10  LOOP 10[00] BPM 113");
}

#[test]
fn the_cursor_moves_back_down_from_the_header_to_the_first_step() {
    let app = phrase_app(&[UP, DOWN]);

    assert_eq!(phrase_line(&app, FIRST_STEP_LINE), "00[...] .    .   .");
}

#[test]
fn two_typed_digits_set_the_speed_and_update_the_bpm_readout() {
    let app = phrase_app(&[UP, UP, UP, Action::HexDigit(0x2), Action::HexDigit(0x0)]);

    assert_eq!(phrase(&app).speed(), 32);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD[20] LOOP 10 00  BPM 56");
}

#[test]
fn the_speed_stops_at_1_and_255() {
    let app = phrase_app(&[UP, UP, UP, Action::HexDigit(0), Action::HexDigit(0)]);
    assert_eq!(phrase(&app).speed(), 1);
    assert_eq!(
        phrase_line(&app, SPEED_LINE),
        "SPD[01] LOOP 10 00  BPM 1807"
    );

    let app = phrase_app(&[
        UP,
        UP,
        UP,
        Action::HexDigit(0xF),
        Action::HexDigit(0xF),
        NUDGE_UP,
    ]);
    assert_eq!(phrase(&app).speed(), 255);
}

#[test]
fn moving_the_cursor_starts_a_new_typed_value() {
    let app = phrase_app(&[
        UP,
        UP,
        UP,
        Action::HexDigit(0x2),
        DOWN,
        UP,
        Action::HexDigit(0x3),
    ]);

    assert_eq!(phrase(&app).speed(), 3);
}

#[test]
fn with_end_00_the_start_is_the_length_and_nothing_loops() {
    let app = with_loop(4, 0);
    assert_eq!(phrase(&app).length(), 4);
    assert_eq!(phrase(&app).phrase_loop(), None);
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 3), "03 ...  .    .   .");
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 4), "04 ...  .    .   .");

    let app = with_loop(1, 0);
    assert_eq!(phrase(&app).length(), 1);
}

#[test]
fn a_start_below_the_end_loops_start_to_end_minus_1_and_the_end_is_the_length() {
    let app = with_loop(4, 0x0C);

    assert_eq!(phrase(&app).length(), 12);
    assert_eq!(phrase(&app).phrase_loop(), Some(4..12));
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD 10  LOOP[04]0C  BPM 113");

    let app = with_loop(0, 0x20);
    assert_eq!(phrase(&app).length(), 32);
    assert_eq!(phrase(&app).phrase_loop(), Some(0..32));
}

#[test]
fn any_other_pair_plays_all_32_steps_and_nothing_loops() {
    for (start, end) in [(0, 0), (8, 8), (0x0C, 4)] {
        let app = with_loop(start, end);
        assert_eq!(phrase(&app).length(), 32, "LOOP {start:02X} {end:02X}");
        assert_eq!(
            phrase(&app).phrase_loop(),
            None,
            "LOOP {start:02X} {end:02X}"
        );
        assert_eq!(
            phrase_line(&app, FIRST_STEP_LINE + 31),
            "1F ...  .    .   ."
        );
    }
}

#[test]
fn the_loop_pair_runs_from_00_to_20() {
    let app = phrase_app(&typed(&LOOP_START, 0xFF));
    assert_eq!(phrase(&app).loop_start(), 32);

    let app = phrase_app(&[&typed(&LOOP_END, 0x01)[..], &[NUDGE_DOWN, NUDGE_DOWN]].concat());
    assert_eq!(phrase(&app).loop_end(), 0);
}

#[test]
fn the_phrase_loop_start_is_marked_beside_its_row_label() {
    let app = with_loop(4, 0x0C);

    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 4), "04<...  .    .   .");
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 3), "03 ...  .    .   .");
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 5), "05 ...  .    .   .");

    let app = with_loop(4, 0);
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 3), "03 ...  .    .   .");
}

#[test]
fn each_loop_edit_is_one_undo_step() {
    let mut app = with_loop(4, 0x0C);
    app.apply(NUDGE_UP);
    assert_eq!(phrase(&app).loop_start(), 5);

    app.apply(Action::Undo);
    assert_eq!(phrase(&app).loop_start(), 4);
    assert_eq!(phrase(&app).loop_end(), 0x0C);
}

#[test]
fn after_shortening_len_the_cursor_still_reaches_the_steps_past_it() {
    let app = phrase_app(
        &[
            &[DOWN; 10][..],
            &[UP; 10],
            &typed(&LOOP_START, 0x02),
            &[LEFT],
            &[DOWN; 10],
        ]
        .concat(),
    );

    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 7), "07[...] .    .   .");
}

#[test]
fn shortening_and_lengthening_the_phrase_keeps_the_steps_past_the_end() {
    let app = phrase_app(
        &[
            &[DOWN; 8][..],
            &[Action::PianoKey(0)],
            &[UP; 12],
            &[RIGHT, Action::HexDigit(0), Action::HexDigit(4)],
            &[Action::HexDigit(1), Action::HexDigit(0)],
        ]
        .concat(),
    );

    assert_eq!(phrase(&app).length(), 16);
    assert_eq!(phrase(&app).step(8).volume(), 5);
}

#[test]
fn shortening_and_lengthening_keeps_the_steps_past_the_end() {
    let app = phrase_app(
        &[
            &[DOWN; 8][..],
            &[Action::PianoKey(0)],
            &[UP; 9],
            &typed(&LOOP_START, 0x04),
            &[Action::HexDigit(1), Action::HexDigit(0)],
        ]
        .concat(),
    );

    assert_eq!(phrase(&app).length(), 16);
    assert_eq!(phrase(&app).step(8).volume(), 5);
}

#[test]
fn noiz_and_buzz_switch_on_and_off() {
    let app = phrase_app(&[UP, UP, Action::HexDigit(1), RIGHT, NUDGE_UP]);
    assert!(phrase(&app).filters().noiz());
    assert!(phrase(&app).filters().buzz());
    assert_eq!(phrase_line(&app, NOIZ_LINE), "NOIZ 1  BUZZ[1]");

    let app = phrase_app(&[
        UP,
        UP,
        Action::HexDigit(1),
        Action::HexDigit(0),
        RIGHT,
        NUDGE_UP,
        NUDGE_DOWN,
    ]);
    assert!(!phrase(&app).filters().noiz());
    assert!(!phrase(&app).filters().buzz());
}

#[test]
fn detune_reverb_and_dampen_run_from_0_to_2() {
    let app = phrase_app(&[
        UP,
        Action::HexDigit(1),
        RIGHT,
        Action::HexDigit(2),
        RIGHT,
        Action::HexDigit(9),
    ]);

    let filters = phrase(&app).filters();
    assert_eq!(filters.detune(), 1);
    assert_eq!(filters.reverb(), 2);
    assert_eq!(filters.dampen(), 2);
    assert_eq!(
        phrase_line(&app, FILTER_LINE),
        "DETUNE 1  REVERB 2  DAMPEN[2]"
    );
}

#[test]
fn the_cursor_stops_at_the_ends_of_a_header_row() {
    let app = phrase_app(&[UP, LEFT, LEFT, RIGHT, RIGHT, RIGHT, RIGHT]);

    assert_eq!(
        phrase_line(&app, FILTER_LINE),
        "DETUNE 0  REVERB 0  DAMPEN[0]"
    );
}
