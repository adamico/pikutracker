use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::project::Phrase;

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
const LENGTH_LINE: usize = 2;
const FILTER_LINE: usize = 3;
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
    app.project().phrase(app.phrase_under_cursor().unwrap())
}

fn phrase_line(app: &App, line: usize) -> String {
    let text: String = app
        .grid()
        .text_dump()
        .lines()
        .nth(line)
        .unwrap()
        .chars()
        .skip(63)
        .take(19)
        .collect();
    text.trim_end().to_owned()
}

#[test]
fn a_new_phrase_has_16_steps_at_speed_16() {
    let app = phrase_app(&[]);

    assert_eq!(phrase(&app).length(), 16);
    assert_eq!(phrase(&app).speed(), 16);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD 10 BPM 113");
    assert_eq!(phrase_line(&app, LENGTH_LINE), "LEN 10 NZ 0 BZ 0");
    assert_eq!(phrase_line(&app, FILTER_LINE), "DT 0 RV 0 DP 0");
}

#[test]
fn the_cursor_moves_up_from_the_first_step_into_the_header() {
    let app = phrase_app(&[UP]);
    assert_eq!(phrase_line(&app, FILTER_LINE), "DT[0]RV 0 DP 0");

    let app = phrase_app(&[UP, UP]);
    assert_eq!(phrase_line(&app, LENGTH_LINE), "LEN[10]NZ 0 BZ 0");

    let app = phrase_app(&[UP, UP, UP, UP]);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD[10]BPM 113");
}

#[test]
fn the_cursor_moves_back_down_from_the_header_to_the_first_step() {
    let app = phrase_app(&[UP, DOWN]);

    assert_eq!(phrase_line(&app, FIRST_STEP_LINE), "00[...].. . .");
}

#[test]
fn two_typed_digits_set_the_speed_and_update_the_bpm_readout() {
    let app = phrase_app(&[UP, UP, UP, Action::HexDigit(0x2), Action::HexDigit(0x0)]);

    assert_eq!(phrase(&app).speed(), 32);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD[20]BPM 56");
}

#[test]
fn the_speed_stops_at_1_and_255() {
    let app = phrase_app(&[UP, UP, UP, Action::HexDigit(0), Action::HexDigit(0)]);
    assert_eq!(phrase(&app).speed(), 1);
    assert_eq!(phrase_line(&app, SPEED_LINE), "SPD[01]BPM 1807");

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
fn len_runs_from_1_to_32_and_sets_the_visible_steps() {
    let app = phrase_app(&[UP, UP, Action::HexDigit(0), Action::HexDigit(4)]);
    assert_eq!(phrase(&app).length(), 4);
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 3), "03 ... .. . .");
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 4), "04 ... .. . .");

    let app = phrase_app(&[UP, UP, Action::HexDigit(0), Action::HexDigit(0)]);
    assert_eq!(phrase(&app).length(), 1);

    let app = phrase_app(&[UP, UP, Action::HexDigit(0xF), Action::HexDigit(0xF)]);
    assert_eq!(phrase(&app).length(), 32);
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 31), "1F ... .. . .");
}

#[test]
fn after_shortening_len_the_cursor_still_reaches_the_steps_past_it() {
    let app = phrase_app(
        &[
            &[DOWN; 10][..],
            &[UP; 12],
            &[Action::HexDigit(0), Action::HexDigit(2)],
            &[DOWN; 9],
        ]
        .concat(),
    );

    assert_eq!(phrase_line(&app, FIRST_STEP_LINE + 7), "07[...].. . .");
}

#[test]
fn shortening_and_lengthening_len_keeps_the_steps_past_the_end() {
    let app = phrase_app(
        &[
            &[DOWN; 8][..],
            &[Action::PianoKey(0)],
            &[UP; 11],
            &[Action::HexDigit(0), Action::HexDigit(4)],
            &[Action::HexDigit(1), Action::HexDigit(0)],
        ]
        .concat(),
    );

    assert_eq!(phrase(&app).length(), 16);
    assert_eq!(phrase(&app).step(8).volume(), 5);
}

#[test]
fn noiz_and_buzz_switch_on_and_off() {
    let app = phrase_app(&[UP, UP, RIGHT, Action::HexDigit(1), RIGHT, NUDGE_UP]);
    assert!(phrase(&app).filters().noiz());
    assert!(phrase(&app).filters().buzz());
    assert_eq!(phrase_line(&app, LENGTH_LINE), "LEN 10 NZ 1 BZ[1]");

    let app = phrase_app(&[
        UP,
        UP,
        RIGHT,
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
    assert_eq!(phrase_line(&app, FILTER_LINE), "DT 1 RV 2 DP[2]");
}

#[test]
fn the_cursor_stops_at_the_ends_of_a_header_row() {
    let app = phrase_app(&[UP, LEFT, LEFT, RIGHT, RIGHT, RIGHT, RIGHT]);

    assert_eq!(phrase_line(&app, FILTER_LINE), "DT 0 RV 0 DP[0]");
}
