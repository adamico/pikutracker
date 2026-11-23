use pikutracker_core::app::{Action, App, Direction, KeyContext};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::project::{Phrase, Step};
use pikutracker_core::screen::palette::Colour;

const FIRST_STEP_LINE: usize = 5;
const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const C: Action = Action::PianoKey(0);

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

fn step(app: &App, index: usize) -> Step {
    phrase(app).step(index)
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

fn step_line(app: &App, index: usize) -> String {
    phrase_line(app, FIRST_STEP_LINE + index)
}

#[test]
fn a_new_phrase_shows_empty_steps() {
    let app = phrase_app(&[]);

    assert_eq!(step_line(&app, 0), "00[...]. . .");
    assert_eq!(step_line(&app, 15), "0F ... . . .");
    assert_eq!(step_line(&app, 31), "1F ... . . .");
}

#[test]
fn a_piano_key_enters_a_note_with_the_current_instrument_volume_and_octave() {
    let app = phrase_app(&[C]);

    let entered = step(&app, 0);
    assert_eq!(entered.pitch(), 24);
    assert_eq!(entered.instrument(), 2);
    assert_eq!(entered.volume(), 5);
    assert_eq!(entered.effect(), 0);
    assert_eq!(step_line(&app, 0), "00 C-2 2 5 0");
}

#[test]
fn entering_a_note_moves_the_cursor_down_one_step() {
    let app = phrase_app(&[C]);

    assert_eq!(step_line(&app, 1), "01[...]. . .");
}

#[test]
fn the_cursor_stays_on_step_31_after_entering_a_note_there() {
    let app = phrase_app(&[&[DOWN; 31][..], &[C]].concat());

    assert_eq!(step_line(&app, 31), "1F[C-2]2 5 0");
}

#[test]
fn piano_keys_cover_two_octaves_of_semitones() {
    let app = phrase_app(&[Action::PianoKey(1), Action::PianoKey(13)]);

    assert_eq!(step_line(&app, 0), "00 C#2 2 5 0");
    assert_eq!(step_line(&app, 1), "01 C#3 2 5 0");
}

#[test]
fn pitch_33_is_named_a_2() {
    let app = phrase_app(&[Action::PianoKey(9)]);

    assert_eq!(step(&app, 0).pitch(), 33);
    assert_eq!(step_line(&app, 0), "00 A-2 2 5 0");
}

#[test]
fn the_octave_keys_change_the_octave_of_later_notes() {
    let app = phrase_app(&[
        Action::OctaveUp,
        C,
        Action::OctaveDown,
        Action::OctaveDown,
        C,
    ]);

    assert_eq!(step(&app, 0).pitch(), 36);
    assert_eq!(step(&app, 1).pitch(), 12);
}

#[test]
fn the_phrase_title_shows_the_octave() {
    let app = phrase_app(&[Action::OctaveUp]);

    assert_eq!(phrase_line(&app, 0), "PHRASE 00   OCT 3");
}

#[test]
fn the_octave_stops_at_0_and_4() {
    let app = phrase_app(&[Action::OctaveUp; 9]);
    assert_eq!(phrase_line(&app, 0), "PHRASE 00   OCT 4");

    let app = phrase_app(&[Action::OctaveDown; 9]);
    assert_eq!(phrase_line(&app, 0), "PHRASE 00   OCT 0");
}

#[test]
fn a_note_above_the_pitch_range_stops_at_d_sharp_5() {
    let app = phrase_app(&[&[Action::OctaveUp; 4][..], &[Action::PianoKey(23)]].concat());

    assert_eq!(step(&app, 0).pitch(), 63);
    assert_eq!(step_line(&app, 0), "00 D#5 2 5 0");
}

#[test]
fn hex_digits_edit_the_instrument_volume_and_effect_of_a_step() {
    let app = phrase_app(&[
        C,
        UP,
        RIGHT,
        Action::HexDigit(4),
        RIGHT,
        Action::HexDigit(3),
        RIGHT,
        Action::HexDigit(6),
    ]);

    let edited = step(&app, 0);
    assert_eq!(edited.instrument(), 4);
    assert_eq!(edited.volume(), 3);
    assert_eq!(edited.effect(), 6);
    assert_eq!(step_line(&app, 0), "00 C-2 4 3[6]");
}

#[test]
fn step_values_stop_at_their_limits() {
    let app = phrase_app(&[
        C,
        UP,
        RIGHT,
        RIGHT,
        Action::HexDigit(9),
        RIGHT,
        Action::HexDigit(8),
    ]);

    let edited = step(&app, 0);
    assert_eq!(edited.volume(), 7);
    assert_eq!(edited.effect(), 7);
}

#[test]
fn an_edited_instrument_and_volume_become_the_current_ones_for_new_notes() {
    let app = phrase_app(&[
        C,
        UP,
        RIGHT,
        Action::HexDigit(6),
        RIGHT,
        Action::HexDigit(2),
        LEFT,
        LEFT,
        DOWN,
        C,
    ]);

    assert_eq!(step(&app, 1).instrument(), 6);
    assert_eq!(step(&app, 1).volume(), 2);
}

#[test]
fn entering_a_note_replaces_the_volume_of_the_step_with_the_current_volume() {
    let app = phrase_app(&[
        C,
        UP,
        RIGHT,
        RIGHT,
        Action::HexDigit(7),
        DOWN,
        Action::HexDigit(3),
        UP,
        LEFT,
        LEFT,
        C,
    ]);

    assert_eq!(step(&app, 0).volume(), 3);
}

#[test]
fn a_volume_0_step_shows_as_empty_and_keeps_its_pitch_and_instrument() {
    let app = phrase_app(&[
        Action::PianoKey(9),
        UP,
        RIGHT,
        RIGHT,
        Action::HexDigit(0),
        DOWN,
    ]);

    assert_eq!(step_line(&app, 0), "00 ... . . .");
    let silent = step(&app, 0);
    assert_eq!(silent.volume(), 0);
    assert_eq!(silent.pitch(), 33);
    assert_eq!(silent.instrument(), 2);
}

#[test]
fn volume_0_can_be_the_current_volume() {
    let app = phrase_app(&[
        C,
        UP,
        RIGHT,
        RIGHT,
        Action::HexDigit(0),
        LEFT,
        LEFT,
        DOWN,
        C,
    ]);

    assert_eq!(step(&app, 1).pitch(), 24);
    assert_eq!(step(&app, 1).volume(), 0);
    assert_eq!(step_line(&app, 1), "01 ... . . .");
}

#[test]
fn nudging_changes_the_cell_under_the_cursor() {
    let nudge_up = Action::Nudge {
        up: true,
        coarse: false,
    };
    let octave_up = Action::Nudge {
        up: true,
        coarse: true,
    };
    let nudge_down = Action::Nudge {
        up: false,
        coarse: false,
    };

    let app = phrase_app(&[C, UP, nudge_up, octave_up, RIGHT, RIGHT, nudge_down]);

    assert_eq!(step(&app, 0).pitch(), 37);
    assert_eq!(step(&app, 0).volume(), 4);
}

#[test]
fn piano_keys_do_nothing_outside_the_pitch_column() {
    let app = phrase_app(&[RIGHT, C]);

    assert_eq!(step(&app, 0).volume(), 0);
}

#[test]
fn step_edits_do_nothing_outside_the_phrase_pane() {
    let app = phrase_app(&[Action::FocusNext, C, Action::HexDigit(3)]);

    assert_eq!(step(&app, 0), Step::default());
}

#[test]
fn the_key_context_follows_the_phrase_cursor() {
    let mut app = App::new();
    assert_eq!(app.key_context(), KeyContext::ReferenceValue);

    app.apply(Action::FocusNext);
    app.apply(Action::FocusNext);
    assert_eq!(app.key_context(), KeyContext::PhrasePitch);

    app.apply(RIGHT);
    assert_eq!(app.key_context(), KeyContext::PhraseValue);
}

#[test]
fn the_pc_tracker_keys_write_notes_in_the_pitch_column_and_digits_elsewhere() {
    let keymap = Keymap::pc_tracker();
    let mut app = phrase_app(&[]);
    let press = |app: &mut App, ch: char| {
        let press = KeyPress::new(Key::Char(ch), Modifiers::NONE);
        if let Some(action) = keymap.action_in(press, app.key_context()) {
            app.apply(action);
        }
    };

    press(&mut app, '2');
    press(&mut app, 'c');
    assert_eq!(step(&app, 0).pitch(), 37);
    assert_eq!(step(&app, 1).pitch(), 28);

    app.apply(UP);
    app.apply(RIGHT);
    press(&mut app, '3');
    assert_eq!(step(&app, 1).instrument(), 3);
}

#[test]
fn the_pc_tracker_piano_follows_two_rows_of_the_keyboard() {
    let keymap = Keymap::pc_tracker();
    let piano = |ch: char| {
        keymap.action_in(
            KeyPress::new(Key::Char(ch), Modifiers::NONE),
            KeyContext::PhrasePitch,
        )
    };

    for (semitone, ch) in "zsxdcvgbhnjm".chars().enumerate() {
        assert_eq!(piano(ch), Some(Action::PianoKey(semitone as u8)), "{ch}");
    }
    for (semitone, ch) in "q2w3er5t6y7u".chars().enumerate() {
        assert_eq!(
            piano(ch),
            Some(Action::PianoKey(12 + semitone as u8)),
            "{ch}"
        );
    }
}

#[test]
fn the_pc_tracker_brackets_change_the_octave_and_modified_arrows_nudge() {
    let keymap = Keymap::pc_tracker();
    let action = |key: Key, modifiers: Modifiers| keymap.action_for(KeyPress::new(key, modifiers));

    assert_eq!(
        action(Key::Char('['), Modifiers::NONE),
        Some(Action::OctaveDown)
    );
    assert_eq!(
        action(Key::Char(']'), Modifiers::NONE),
        Some(Action::OctaveUp)
    );
    assert_eq!(
        action(Key::Up, Modifiers::CTRL),
        Some(Action::Nudge {
            up: true,
            coarse: false
        })
    );
    assert_eq!(
        action(Key::Down, Modifiers::CTRL_SHIFT),
        Some(Action::Nudge {
            up: false,
            coarse: true
        })
    );
}

const PHRASE_COLUMN: usize = 63;
const PITCH_OFFSET: usize = 3;
const LEN_8: [Action; 10] = [
    UP,
    UP,
    UP,
    RIGHT,
    Action::HexDigit(0),
    Action::HexDigit(8),
    LEFT,
    DOWN,
    DOWN,
    DOWN,
];

fn pitch_colour(app: &App, index: usize) -> Colour {
    app.grid()
        .cell(PHRASE_COLUMN + PITCH_OFFSET, FIRST_STEP_LINE + index)
        .style
        .fg
}

fn label_colour(app: &App, index: usize) -> Colour {
    app.grid()
        .cell(PHRASE_COLUMN, FIRST_STEP_LINE + index)
        .style
        .fg
}

#[test]
fn steps_past_len_are_shown_dimmed_with_their_notes() {
    let app = phrase_app(&[&[DOWN; 10][..], &[C], &[UP; 11], &LEN_8].concat());

    assert_eq!(phrase(&app).length(), 8);
    assert_eq!(step_line(&app, 10), "0A C-2 2 5 0");
    assert_eq!(step_line(&app, 31), "1F ... . . .");
    assert_eq!(pitch_colour(&app, 10), Colour::DarkGrey);
    assert_eq!(label_colour(&app, 10), Colour::DarkGrey);
    assert_eq!(label_colour(&app, 7), Colour::Lavender);
}

#[test]
fn lengthening_shows_the_steps_in_the_normal_style_with_no_data_change() {
    let shortened = phrase_app(&[&[DOWN; 10][..], &[C], &[UP; 11], &LEN_8].concat());
    let app = phrase_app(
        &[
            &[DOWN; 10][..],
            &[C],
            &[UP; 11],
            &LEN_8,
            &[UP, UP, UP, RIGHT, Action::HexDigit(1), Action::HexDigit(0)],
        ]
        .concat(),
    );

    assert_eq!(phrase(&app).length(), 16);
    assert_eq!(phrase(&app).step(10), phrase(&shortened).step(10));
    assert_eq!(step_line(&app, 10), "0A C-2 2 5 0");
    assert_eq!(pitch_colour(&app, 10), Colour::White);
    assert_eq!(label_colour(&app, 10), Colour::Lavender);
}

#[test]
fn the_cursor_moves_past_len_and_stops_at_step_31() {
    let app = phrase_app(&[&LEN_8[..], &[DOWN; 40]].concat());

    assert_eq!(step_line(&app, 31), "1F[...]. . .");
}

#[test]
fn a_step_past_len_can_be_edited_and_still_does_not_play() {
    let app = phrase_app(
        &[
            &LEN_8[..],
            &[DOWN; 20],
            &[C, UP, RIGHT, Action::HexDigit(3)],
        ]
        .concat(),
    );

    assert_eq!(phrase(&app).length(), 8);
    assert_eq!(step(&app, 20).pitch(), 24);
    assert_eq!(step(&app, 20).instrument(), 3);
    assert_eq!(step_line(&app, 20), "14 C-2[3]5 0");
}

#[test]
fn entering_a_note_on_the_last_step_within_len_moves_into_the_dimmed_steps() {
    let app = phrase_app(&[&LEN_8[..], &[DOWN; 7], &[C]].concat());

    assert_eq!(step_line(&app, 8), "08[...]. . .");
}
