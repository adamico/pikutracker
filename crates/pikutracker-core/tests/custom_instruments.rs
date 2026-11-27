mod support;

use pikutracker_core::app::{Action, App, Direction, KeyContext, layout};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::player::{AudioRequest, Player};
use pikutracker_core::project::{
    ChainEntry, ChainId, CustomInstrument, Filters, InstrumentSound, PhraseId, Project,
    SfxInstrument, Step, WaveformInstrument,
};
use pikutracker_core::project_file;
use pikutracker_core::synth::SAMPLE_RATE;
use support::{apply, render_song};

const C2: u8 = 24;
const A2: u8 = 33;
const C3: u8 = 36;
const TRIANGLE: u8 = 0;
const SETTLED: usize = 2_000;

fn step(pitch: u8, instrument: u8, volume: u8) -> Step {
    Step::new(pitch, instrument, volume, 0).unwrap()
}

fn playing_one_note(pitch: u8, instrument: u8) -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_speed(64);
    phrase.set_length(1);
    *phrase.step_mut(0) = step(pitch, instrument, 5);
    project
}

fn waveform_instrument() -> CustomInstrument {
    CustomInstrument {
        name: None,
        sound: InstrumentSound::Waveform(WaveformInstrument {
            samples: std::array::from_fn(|index| if index < 32 { 100 } else { -100 }),
            bass: false,
            detune: 0,
            reverb: 0,
            dampen: 0,
        }),
    }
}

fn sfx_instrument() -> CustomInstrument {
    CustomInstrument {
        name: None,
        sound: InstrumentSound::Sfx(SfxInstrument {
            steps: vec![step(A2, TRIANGLE, 7); 32],
            speed: 32,
            filters: Filters::default(),
            loop_start: 0,
            loop_end: 32,
        }),
    }
}

fn frequency(samples: &[f32]) -> f32 {
    let rising: Vec<usize> = samples
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, _)| index)
        .collect();
    assert!(rising.len() > 2, "no tone");
    let (first, last) = (rising[0], rising[rising.len() - 1]);
    (rising.len() - 1) as f32 * SAMPLE_RATE as f32 / (last - first) as f32
}

fn assert_near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} within {tolerance}, got {actual}"
    );
}

#[test]
fn a_step_on_a_waveform_instrument_at_c2_plays_at_261_6_hz() {
    let mut project = playing_one_note(C2, 0x0C);
    project.set_custom_instrument(0x0C, Some(waveform_instrument()));

    let samples = render_song(project, 10_000);

    assert_near(frequency(&samples[SETTLED..]), 261.63, 1.0);
}

#[test]
fn an_sfx_instrument_plays_relative_to_c2() {
    let mut project = playing_one_note(C3, 0x08);
    project.set_custom_instrument(0x08, Some(sfx_instrument()));

    let samples = render_song(project, 10_000);

    assert_near(frequency(&samples[SETTLED..]), 880.0, 4.0);
}

#[test]
fn a_project_holds_custom_instruments_8_to_f_only() {
    let mut project = Project::new();
    project.set_custom_instrument(0x0F, Some(sfx_instrument()));
    project.set_custom_instrument(0x10, Some(sfx_instrument()));

    let ids: Vec<u8> = project.custom_instruments().map(|(id, _)| id).collect();
    assert_eq!(ids, [0x0F]);
}

#[test]
fn a_step_on_an_undefined_custom_instrument_is_silent() {
    let project = playing_one_note(C2, 0x0E);

    let samples = render_song(project, 10_000);

    assert!(samples.iter().all(|sample| *sample == 0.0));
}

const UP: Action = Action::Move(Direction::Up);
const DOWN: Action = Action::Move(Direction::Down);
const LEFT: Action = Action::Move(Direction::Left);
const RIGHT: Action = Action::Move(Direction::Right);
const ENTER: Action = Action::Confirm;
const ESC: Action = Action::Cancel;
const SPACE: Action = Action::PlayFocusedPane;
const C_KEY: Action = Action::PianoKey(0);

fn phrase_app(actions: &[Action]) -> App {
    let mut app = App::new();
    apply(
        &mut app,
        &[
            Action::HexDigit(0),
            Action::FocusPrevious,
            Action::HexDigit(0),
            Action::FocusPrevious,
        ],
    );
    apply(&mut app, actions);
    app
}

fn typed(value: u8) -> [Action; 2] {
    [Action::HexDigit(value >> 4), Action::HexDigit(value & 0xf)]
}

fn on_instrument(instrument: u8) -> App {
    let mut app = phrase_app(&[C_KEY, UP, RIGHT]);
    app.apply(Action::HexDigit(instrument));
    app
}

fn first_step(app: &App) -> Step {
    app.project().phrase(PhraseId::new(0)).step(0)
}

fn lines(app: &App) -> Vec<String> {
    app.grid()
        .text_dump()
        .lines()
        .map(|line| line.trim_end().to_owned())
        .collect()
}

#[test]
fn a_step_instrument_is_one_hex_digit() {
    let mut app = on_instrument(0x0C);
    app.apply(Action::HexDigit(0x0A));

    assert_eq!(first_step(&app).instrument(), 0x0A);
    assert_eq!(
        support::pane_text(&app, layout::PHRASE, 6),
        "00 C-2 [A]   5   0"
    );
}

const CUSTOM_INSTRUMENTS_END_AT_F: &str = "CUSTOM INSTRUMENTS END AT F";

#[test]
fn nudging_a_step_instrument_past_f_stays_on_f_with_a_remark() {
    let mut app = on_instrument(0x0F);
    app.apply(NUDGE_UP);

    assert_eq!(first_step(&app).instrument(), 0x0F);
    assert_eq!(support::status_line(&app), CUSTOM_INSTRUMENTS_END_AT_F);
}

fn on_the_pen_instrument() -> App {
    phrase_app(&[UP, UP, RIGHT])
}

#[test]
fn nudging_the_pen_instrument_past_f_stays_on_f_with_a_remark() {
    let mut app = on_the_pen_instrument();
    apply(&mut app, &[Action::HexDigit(0x0F), NUDGE_UP]);

    assert_eq!(support::pane_text(&app, layout::PHRASE, 3), "OCT 2  INS[F]");
    assert_eq!(support::status_line(&app), CUSTOM_INSTRUMENTS_END_AT_F);
}

#[test]
fn setting_a_step_to_an_undefined_custom_instrument_keeps_it_with_a_remark() {
    let app = on_instrument(0x0A);

    assert_eq!(first_step(&app).instrument(), 0x0A);
    assert_eq!(
        support::status_line(&app),
        "CUSTOM INSTRUMENT A IS NOT DEFINED"
    );
}

#[test]
fn nudging_a_step_onto_an_undefined_custom_instrument_keeps_it_with_a_remark() {
    let mut app = on_instrument(0x09);
    app.apply(NUDGE_UP);

    assert_eq!(first_step(&app).instrument(), 0x0A);
    assert_eq!(
        support::status_line(&app),
        "CUSTOM INSTRUMENT A IS NOT DEFINED"
    );
}

#[test]
fn setting_the_pen_to_an_undefined_custom_instrument_keeps_it_with_a_remark() {
    let mut app = on_the_pen_instrument();
    app.apply(Action::HexDigit(0x0C));

    assert_eq!(support::pane_text(&app, layout::PHRASE, 3), "OCT 2  INS[C]");
    assert_eq!(
        support::status_line(&app),
        "CUSTOM INSTRUMENT C IS NOT DEFINED"
    );
}

#[test]
fn setting_a_step_to_a_defined_custom_instrument_needs_no_remark() {
    let mut project = Project::new();
    project.set_custom_instrument(0x0A, Some(sfx_instrument()));
    let mut app = App::with_project(project);
    apply(
        &mut app,
        &[
            Action::HexDigit(0),
            Action::FocusPrevious,
            Action::HexDigit(0),
            Action::FocusPrevious,
            C_KEY,
            UP,
            RIGHT,
            Action::HexDigit(0x0A),
        ],
    );

    assert_eq!(first_step(&app).instrument(), 0x0A);
    assert!(!support::status_line(&app).contains("NOT DEFINED"));
}

#[test]
fn jumping_past_instrument_f_stays_on_f_with_a_remark() {
    let mut app = instrument_view(0x0F);
    app.apply(Action::Jump(Direction::Right));

    assert_eq!(lines(&app)[0], "INSTRUMENT F");
    assert_eq!(support::status_line(&app), CUSTOM_INSTRUMENTS_END_AT_F);
}

#[test]
fn nudging_a_step_instrument_goes_on_past_the_built_in_instruments() {
    let mut app = on_instrument(0x07);
    app.apply(Action::Nudge {
        up: true,
        coarse: false,
    });

    assert_eq!(first_step(&app).instrument(), 0x08);
}

#[test]
fn new_notes_take_the_custom_instrument_last_typed() {
    let mut app = on_instrument(0x0B);
    apply(&mut app, &[LEFT, DOWN, C_KEY]);

    assert_eq!(
        app.project().phrase(PhraseId::new(0)).step(1).instrument(),
        0x0B
    );
}

const NUDGE_UP: Action = Action::Nudge {
    up: true,
    coarse: false,
};

fn instrument_view(instrument: u8) -> App {
    let mut app = on_instrument(instrument);
    app.apply(ENTER);
    app
}

fn instrument(app: &App, id: u8) -> CustomInstrument {
    app.project().custom_instrument(id).cloned().unwrap()
}

fn sfx(app: &App, id: u8) -> SfxInstrument {
    match instrument(app, id).sound {
        InstrumentSound::Sfx(sfx) => sfx,
        InstrumentSound::Waveform(_) => panic!("a Waveform Instrument"),
    }
}

fn wave(app: &App, id: u8) -> WaveformInstrument {
    match instrument(app, id).sound {
        InstrumentSound::Waveform(wave) => wave,
        InstrumentSound::Sfx(_) => panic!("an SFX Instrument"),
    }
}

fn waveform_view(id: u8) -> App {
    let mut app = instrument_view(id);
    apply(&mut app, &[DOWN, NUDGE_UP]);
    app
}

#[test]
fn enter_on_a_step_instrument_column_opens_the_instrument_full_screen() {
    let app = instrument_view(0x08);

    let lines = lines(&app);
    assert_eq!(lines[0], "INSTRUMENT 8");
    assert!(lines.iter().all(|line| !line.contains("PHRASE")));
    assert_eq!(lines[2], "NAME[............]");
    assert_eq!(lines[3], "KIND SFX");
    assert_eq!(lines[4], "SPD 10  LEN 10  LOOP 00 TO 00");
    assert_eq!(lines[5], "NZ 0  BZ 0  DT 0  RV 0  DP 0");
    assert_eq!(lines[7], "   NOTE INST VOL FX");
    assert_eq!(lines[8], "00 ...  .    .   .");
    assert_eq!(lines[23], "0F ...  .    .   .");
    assert_eq!(lines[24], "");
}

#[test]
fn a_built_in_instrument_has_no_view() {
    let mut app = on_instrument(0x03);
    app.apply(ENTER);

    assert!(lines(&app)[0].starts_with("PHRASE"));
    assert_eq!(
        support::status_line(&app),
        "BUILT-IN INSTRUMENTS HAVE NO SETTINGS"
    );
}

#[test]
fn escape_closes_the_instrument_view() {
    let mut app = instrument_view(0x08);
    app.apply(ESC);

    assert!(lines(&app)[0].starts_with("PHRASE"));
    assert_eq!(first_step(&app).instrument(), 0x08);
}

#[test]
fn opening_an_instrument_defines_nothing_until_it_is_edited() {
    let app = instrument_view(0x08);

    assert_eq!(app.project().custom_instrument(0x08), None);
}

#[test]
fn an_sfx_instrument_edits_its_steps() {
    let mut app = instrument_view(0x08);
    apply(&mut app, &[DOWN, DOWN, DOWN, DOWN, C_KEY]);

    assert_eq!(sfx(&app, 0x08).steps[0], step(C2, 2, 5));
    assert_eq!(lines(&app)[8], "00 C-2  2    5   0");
}

#[test]
fn an_sfx_instrument_steps_use_only_built_in_instruments() {
    let mut app = instrument_view(0x08);
    apply(&mut app, &[DOWN, DOWN, DOWN, DOWN, C_KEY, UP, RIGHT]);
    app.apply(Action::HexDigit(0x0C));

    assert_eq!(sfx(&app, 0x08).steps[0].instrument(), 7);
}

#[test]
fn an_sfx_instrument_edits_speed_length_and_loop() {
    let mut app = instrument_view(0x08);
    apply(&mut app, &[DOWN, DOWN]);
    apply(&mut app, &typed(0x04));
    app.apply(RIGHT);
    apply(&mut app, &typed(0x20));
    app.apply(RIGHT);
    apply(&mut app, &typed(0x02));
    app.apply(RIGHT);
    apply(&mut app, &typed(0x06));

    let edited = sfx(&app, 0x08);
    assert_eq!(edited.speed, 4);
    assert_eq!(edited.steps.len(), 32);
    assert_eq!((edited.loop_start, edited.loop_end), (2, 6));
    assert_eq!(lines(&app)[4], "SPD 04  LEN 20  LOOP 02 TO[06]");
}

#[test]
fn an_sfx_instrument_has_1_to_32_steps() {
    let mut app = instrument_view(0x08);
    apply(&mut app, &[DOWN, DOWN, RIGHT]);
    apply(&mut app, &typed(0x40));
    assert_eq!(sfx(&app, 0x08).steps.len(), 32);

    apply(&mut app, &typed(0x00));
    assert_eq!(sfx(&app, 0x08).steps.len(), 1);
}

#[test]
fn an_sfx_instrument_edits_its_filters() {
    let mut app = instrument_view(0x08);
    apply(&mut app, &[DOWN, DOWN, DOWN, Action::HexDigit(1)]);
    for _ in 0..4 {
        apply(&mut app, &[RIGHT, Action::HexDigit(2)]);
    }

    let filters = sfx(&app, 0x08).filters;
    assert!(filters.noiz() && filters.buzz());
    assert_eq!(
        (filters.detune(), filters.reverb(), filters.dampen()),
        (2, 2, 2)
    );
}

#[test]
fn a_waveform_instrument_shows_bass_and_its_filters_but_not_noiz_buzz_speed_or_loop() {
    let app = waveform_view(0x08);

    let lines = lines(&app);
    assert_eq!(lines[3], "KIND[WAVE]");
    assert_eq!(lines[4], "BASS 0");
    assert_eq!(lines[5], "DT 0  RV 0  DP 0");
    assert_eq!(lines[7], "00 00 00 00 00 00 00 00 00");
    assert_eq!(lines[14], "38 00 00 00 00 00 00 00 00");
    let text = lines.join("\n");
    for hidden in ["NZ", "BZ", "SPD", "LOOP"] {
        assert!(!text.contains(hidden), "{hidden} shows");
    }
}

#[test]
fn a_waveform_instrument_edits_64_signed_samples() {
    let mut app = waveform_view(0x08);
    apply(&mut app, &[DOWN, DOWN, DOWN]);
    apply(&mut app, &typed(0x80));
    app.apply(RIGHT);
    apply(&mut app, &typed(0x7f));
    for _ in 0..7 {
        app.apply(DOWN);
    }
    for _ in 0..7 {
        app.apply(RIGHT);
    }
    apply(&mut app, &typed(0xff));

    let samples = wave(&app, 0x08).samples;
    assert_eq!((samples[0], samples[1], samples[63]), (-128, 127, -1));
    assert_eq!(lines(&app)[7], "00 80 7F 00 00 00 00 00 00");
}

#[test]
fn a_waveform_instrument_edits_bass_detune_reverb_and_dampen() {
    let mut app = waveform_view(0x08);
    apply(
        &mut app,
        &[DOWN, Action::HexDigit(1), DOWN, Action::HexDigit(1)],
    );
    apply(
        &mut app,
        &[RIGHT, Action::HexDigit(2), RIGHT, Action::HexDigit(1)],
    );

    let edited = wave(&app, 0x08);
    assert!(edited.bass);
    assert_eq!((edited.detune, edited.reverb, edited.dampen), (1, 2, 1));
}

#[test]
fn a_custom_instrument_takes_an_optional_name_of_up_to_12_characters() {
    let mut app = instrument_view(0x08);
    assert_eq!(app.key_context(), KeyContext::InstrumentName);
    for ch in "SOFTBASSLINE2".chars() {
        app.apply(Action::TypeChar(ch));
    }

    assert_eq!(instrument(&app, 0x08).name.as_deref(), Some("SOFTBASSLINE"));
    assert_eq!(lines(&app)[2], "NAME[SOFTBASSLINE]");

    for _ in 0..12 {
        app.apply(Action::Backspace);
    }
    assert_eq!(instrument(&app, 0x08).name, None);
}

fn type_samples(app: &mut App, sample: impl Fn(usize) -> u8) {
    apply(app, &[DOWN, DOWN, DOWN]);
    for row in 0..8 {
        for column in 0..8 {
            apply(app, &typed(sample(row * 8 + column)));
            app.apply(RIGHT);
        }
        apply(app, &[DOWN, LEFT, LEFT, LEFT, LEFT, LEFT, LEFT, LEFT]);
    }
}

#[test]
fn space_in_the_instrument_view_auditions_it_at_c2_until_pressed_again() {
    let mut app = waveform_view(0x0C);
    type_samples(&mut app, |index| if index < 32 { 0x64 } else { 0x9c });
    app.take_audio_requests();
    let mut player = Player::new();

    app.apply(SPACE);
    let samples = support::play_along(&mut app, &mut player, 20_000);
    assert_near(frequency(&samples[SETTLED..]), 261.63, 1.0);
    assert!(samples[18_000..].iter().any(|sample| sample.abs() > 0.05));

    app.apply(SPACE);
    assert_eq!(app.take_audio_requests(), vec![AudioRequest::Stop]);
}

#[test]
fn an_edit_during_an_audition_is_heard() {
    let mut app = waveform_view(0x0C);
    app.apply(SPACE);
    app.take_audio_requests();
    apply(&mut app, &[DOWN, DOWN, DOWN]);
    apply(&mut app, &typed(0x64));

    assert!(
        app.take_audio_requests()
            .iter()
            .any(|request| matches!(request, AudioRequest::UpdateProject(_)))
    );
}

#[test]
fn closing_the_instrument_view_ends_an_audition() {
    let mut app = waveform_view(0x0C);
    app.apply(SPACE);
    app.take_audio_requests();
    app.apply(ESC);

    assert_eq!(app.take_audio_requests(), vec![AudioRequest::Stop]);
}

#[test]
fn instrument_edits_are_undoable() {
    let mut app = instrument_view(0x08);
    apply(&mut app, &[DOWN, DOWN]);
    apply(&mut app, &typed(0x04));
    app.apply(Action::Undo);
    assert_eq!(app.project().custom_instrument(0x08), None);

    app.apply(Action::Redo);
    assert_eq!(sfx(&app, 0x08).speed, 4);
}

#[test]
fn custom_instruments_save_and_reload_through_the_project_file() {
    let mut app = waveform_view(0x0C);
    apply(&mut app, &[UP, Action::TypeChar('W'), DOWN]);
    apply(&mut app, &[DOWN, DOWN, DOWN]);
    apply(&mut app, &typed(0x9c));
    app.apply(ESC);
    apply(&mut app, &[LEFT, DOWN]);
    apply(&mut app, &[RIGHT]);
    apply(&mut app, &typed(0x08));
    app.apply(ENTER);
    apply(&mut app, &[DOWN, DOWN, DOWN, DOWN, C_KEY]);

    let saved = project_file::write(app.project());
    let reloaded = project_file::parse(&saved).unwrap();

    assert_eq!(reloaded, *app.project());
    assert_eq!(wave(&app, 0x0C).samples[0], -100);
    assert_eq!(instrument(&app, 0x0C).name.as_deref(), Some("W"));
    assert_eq!(sfx(&app, 0x08).steps[0], step(C2, 2, 5));
}

#[test]
fn letters_type_into_an_instrument_name_and_space_still_auditions() {
    let keymap = Keymap::pc_tracker();
    let in_name = |key| {
        keymap.action_in(
            KeyPress::new(key, Modifiers::NONE),
            KeyContext::InstrumentName,
        )
    };

    assert_eq!(in_name(Key::Char('b')), Some(Action::TypeChar('b')));
    assert_eq!(in_name(Key::Backspace), Some(Action::Backspace));
    assert_eq!(in_name(Key::Space), Some(SPACE));
    assert_eq!(in_name(Key::Escape), Some(ESC));
}

#[test]
fn esc_in_the_file_list_returns_to_the_instrument_view() {
    let mut app = instrument_view(0x08);
    app.apply(Action::Save);
    assert_eq!(app.key_context(), KeyContext::FileName);

    app.apply(ESC);

    assert_eq!(lines(&app)[0], "INSTRUMENT 8");
}
