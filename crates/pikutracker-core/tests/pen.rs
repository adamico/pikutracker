mod support;

use pikutracker_core::app::{Action, App, Direction, layout};
use pikutracker_core::project::{Phrase, Project, Step};
use pikutracker_core::screen::palette::Colour;
use support::pane_text;

const OCT_INS_LINE: usize = 4;
const VOL_FX_LINE: usize = 5;
const STEP_HEADING_LINE: usize = 6;
const FIRST_STEP_LINE: usize = 7;
const SETTINGS_LINE: usize = 1;

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

fn phrase_line(app: &App, line: usize) -> String {
    pane_text(app, layout::PHRASE, line)
}

fn label_colour(app: &App, line: usize) -> Colour {
    app.grid()
        .cell(layout::PHRASE.column, layout::PHRASE.row + line)
        .style
        .fg
}

#[test]
fn the_pen_shows_in_two_rows_under_the_phrase_settings() {
    let app = phrase_app(&[]);

    assert_eq!(phrase_line(&app, 0), "PHRASE 00");
    assert_eq!(phrase_line(&app, OCT_INS_LINE), "OCT 2  INS 2 SAW");
    assert_eq!(phrase_line(&app, VOL_FX_LINE), "VOL 5  FX  0 NONE");
    assert_eq!(phrase_line(&app, STEP_HEADING_LINE), "   NOTE INST VOL FX");
    assert_eq!(phrase_line(&app, FIRST_STEP_LINE), "00[...] .    .   .");
}

#[test]
fn the_pen_labels_are_drawn_in_their_own_colour() {
    let app = phrase_app(&[]);

    let settings = label_colour(&app, SETTINGS_LINE);
    assert_ne!(label_colour(&app, OCT_INS_LINE), settings);
    assert_eq!(
        label_colour(&app, OCT_INS_LINE),
        label_colour(&app, VOL_FX_LINE)
    );
}

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const C: Action = Action::PianoKey(0);

fn step(app: &App, index: usize) -> Step {
    phrase(app).step(index)
}

fn phrase(app: &App) -> &Phrase {
    app.project().phrase(app.shown_phrase().unwrap())
}

#[test]
fn typing_an_effect_into_a_step_sets_the_pen_and_later_notes_take_it() {
    let app = phrase_app(&[
        C,
        UP,
        RIGHT,
        RIGHT,
        RIGHT,
        Action::HexDigit(3),
        LEFT,
        LEFT,
        LEFT,
        DOWN,
        C,
    ]);

    assert_eq!(phrase_line(&app, VOL_FX_LINE), "VOL 5  FX  3 DROP");
    assert_eq!(step(&app, 1), Step::new(24, 2, 5, 3).unwrap());
}

#[test]
fn entering_a_note_overwrites_all_four_values_of_its_step() {
    let app = phrase_app(&[
        C,
        UP,
        RIGHT,
        Action::HexDigit(6),
        RIGHT,
        Action::HexDigit(7),
        RIGHT,
        Action::HexDigit(4),
        DOWN,
        Action::HexDigit(1),
        UP,
        LEFT,
        LEFT,
        LEFT,
        Action::OctaveUp,
        C,
    ]);

    assert_eq!(step(&app, 0), Step::new(36, 6, 7, 1).unwrap());
}

#[test]
fn each_pen_field_can_be_typed_directly() {
    let app = phrase_app(&[
        UP,
        Action::HexDigit(7),
        RIGHT,
        Action::HexDigit(6),
        UP,
        Action::HexDigit(4),
        LEFT,
        Action::HexDigit(3),
        DOWN,
        DOWN,
        LEFT,
        C,
    ]);

    assert_eq!(phrase_line(&app, OCT_INS_LINE), "OCT 3  INS 4 PULSE");
    assert_eq!(phrase_line(&app, VOL_FX_LINE), "VOL 7  FX  6 ARP FAST");
    assert_eq!(step(&app, 0), Step::new(36, 4, 7, 6).unwrap());
}

#[test]
fn the_pen_can_be_set_with_no_phrase_shown() {
    let mut app = App::new();
    for action in [
        Action::FocusNext,
        Action::FocusNext,
        UP,
        Action::Nudge {
            up: true,
            coarse: false,
        },
    ] {
        app.apply(action);
    }

    assert_eq!(phrase_line(&app, VOL_FX_LINE), "VOL[6] FX  0 NONE");
    assert!(!support::status_line(&app).contains("NO PHRASE HERE"));
}

#[test]
fn setting_the_pen_leaves_the_project_untouched_and_adds_no_undo_entry() {
    let mut app = App::new();
    for action in [
        Action::FocusNext,
        Action::FocusNext,
        UP,
        Action::HexDigit(3),
        Action::OctaveUp,
        Action::Undo,
    ] {
        app.apply(action);
    }

    assert_eq!(app.project(), &Project::default());
    assert_eq!(phrase_line(&app, VOL_FX_LINE), "VOL[3] FX  0 NONE");
}

#[test]
fn the_octave_keys_change_the_pen_octave() {
    let app = phrase_app(&[Action::OctaveUp]);

    assert_eq!(phrase_line(&app, OCT_INS_LINE), "OCT 3  INS 2 SAW");
}
