mod support;

use pikutracker_core::cart::Cart;
use pikutracker_core::export::export;
use pikutracker_core::project::{
    Chain, ChainEntry, ChainId, Claim, CustomInstrument, InstrumentSound, PhraseId, Project,
    SfxInstrument, Step, TRACKS, WaveformInstrument,
};
use pikutracker_core::synth::SAMPLES_PER_TICK;
use support::{HEADER, assert_same_samples, render_music, render_song};

const SFX_INSTRUMENT: u8 = 0x0C;
const WAVEFORM_INSTRUMENT: u8 = 0x09;
const FIRST_PATTERN: u8 = 12;
const TAIL_TICKS: usize = 40;

fn write_phrase(project: &mut Project, id: u8, length: i32, speed: i32, instruments: &[u8]) {
    let phrase = project.phrase_mut(PhraseId::new(id));
    phrase.set_length(length);
    phrase.set_speed(speed);
    for index in 0..32 {
        let pitch = 20 + (index * 5 % 17) as u8;
        let effect = if index % 6 == 4 { 2 } else { 0 };
        let (instrument, volume) = if index % 2 == 0 {
            (instruments[index / 2 % instruments.len()], 2)
        } else {
            (0, 0)
        };
        *phrase.step_mut(index) = Step::new(pitch, instrument, volume, effect).unwrap();
    }
}

fn plays(phrase: u8) -> Option<ChainEntry> {
    Some(ChainEntry::new(PhraseId::new(phrase)))
}

fn loop_from(project: &mut Project, phrase: u8, start: i32) {
    let phrase = project.phrase_mut(PhraseId::new(phrase));
    let end = phrase.length() as i32;
    phrase.set_loop(start, end);
}

fn transposed(phrase: u8, semitones: i32) -> Option<ChainEntry> {
    let mut entry = ChainEntry::new(PhraseId::new(phrase));
    entry.set_transpose(semitones);
    Some(entry)
}

fn place_song_row(
    project: &mut Project,
    song_row: usize,
    chain_rows: &[[Option<ChainEntry>; TRACKS]],
) {
    for track in 0..TRACKS {
        let mut chain = Chain::default();
        for (chain_row, entries) in chain_rows.iter().enumerate() {
            *chain.entry_mut(chain_row) = entries[track];
        }
        let id = ChainId::new((song_row * TRACKS + track) as u8);
        *project.chain_mut(id) = chain;
        project.set_song_cell(song_row, track, Some(id));
    }
}

fn instrumented_project() -> Project {
    let mut project = Project::new();
    let mut sfx = SfxInstrument {
        steps: vec![
            Step::new(24, 3, 5, 0).unwrap(),
            Step::new(31, 1, 4, 0).unwrap(),
            Step::new(36, 0, 6, 0).unwrap(),
            Step::new(28, 5, 3, 0).unwrap(),
        ],
        speed: 2,
        loop_start: 0,
        loop_end: 4,
        ..SfxInstrument::default()
    };
    sfx.filters.set_dampen(1);
    project.set_custom_instrument(
        SFX_INSTRUMENT,
        Some(CustomInstrument {
            name: None,
            sound: InstrumentSound::Sfx(sfx),
        }),
    );
    let wave = WaveformInstrument {
        samples: std::array::from_fn(|index| (index as i32 * 4 - 128) as i8),
        detune: 1,
        ..WaveformInstrument::default()
    };
    project.set_custom_instrument(
        WAVEFORM_INSTRUMENT,
        Some(CustomInstrument {
            name: None,
            sound: InstrumentSound::Waveform(wave),
        }),
    );
    project.set_song_loop_row(None);
    project
}

fn assert_exports_sample_for_sample(project: &Project, song_ticks: usize) {
    let claim = Claim {
        sfx: (0..8).chain(10..30).collect(),
        first_and_last_pattern: Some((FIRST_PATTERN, 30)),
    };
    let original = format!("{HEADER}__lua__\nprint(1)\n");

    let exported = export(project, &claim, original.as_bytes()).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    let samples = (song_ticks + TAIL_TICKS) * SAMPLES_PER_TICK as usize;
    assert_same_samples(
        &render_music(&cart, FIRST_PATTERN, samples),
        &render_song(project.clone(), samples),
    );
}

#[test]
fn every_kind_of_chain_row_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 3]);
    write_phrase(&mut project, 1, 16, 6, &[2, 4, 1]);
    write_phrase(&mut project, 2, 4, 6, &[SFX_INSTRUMENT, 0x0E]);
    write_phrase(&mut project, 3, 16, 3, &[WAVEFORM_INSTRUMENT, 5]);
    write_phrase(&mut project, 4, 8, 8, &[1]);
    write_phrase(&mut project, 5, 6, 8, &[3, 0]);
    write_phrase(&mut project, 6, 16, 8, &[4]);
    write_phrase(&mut project, 7, 4, 8, &[2]);
    write_phrase(&mut project, 8, 32, 5, &[0, SFX_INSTRUMENT]);
    let shorter_left_of_longest_then_looping_then_shorter =
        [plays(0), plays(1), plays(2), plays(3)];
    let every_phrase_looping = [plays(4), plays(5), None, None];
    let timed_by_t4_with_shorter_left_of_an_empty_channel = [plays(7), None, plays(2), plays(6)];
    let only_t4_plays = [None, None, None, plays(4)];
    loop_from(&mut project, 2, 1);
    loop_from(&mut project, 4, 0);
    loop_from(&mut project, 5, 2);
    place_song_row(
        &mut project,
        0,
        &[
            shorter_left_of_longest_then_looping_then_shorter,
            every_phrase_looping,
            timed_by_t4_with_shorter_left_of_an_empty_channel,
            only_t4_plays,
        ],
    );
    place_song_row(
        &mut project,
        1,
        &[[plays(0), transposed(8, 5), plays(8), None]],
    );

    let song_row_0_ticks = 96 + 64 + 128 + 64;
    let song_row_1_ticks = 160;
    assert_exports_sample_for_sample(&project, song_row_0_ticks + song_row_1_ticks);
}

#[test]
fn phrase_filters_play_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 6, 7]);
    write_phrase(&mut project, 1, 16, 6, &[2, SFX_INSTRUMENT, 7]);
    write_phrase(&mut project, 2, 4, 6, &[WAVEFORM_INSTRUMENT, 1]);
    write_phrase(&mut project, 3, 16, 6, &[5, 3]);
    project
        .phrase_mut(PhraseId::new(0))
        .filters_mut()
        .set_reverb(2);
    let filters = project.phrase_mut(PhraseId::new(1)).filters_mut();
    filters.set_detune(1);
    filters.set_reverb(1);
    let filters = project.phrase_mut(PhraseId::new(2)).filters_mut();
    filters.set_detune(2);
    filters.set_reverb(2);
    let filters = project.phrase_mut(PhraseId::new(3)).filters_mut();
    filters.set_dampen(2);
    filters.set_noiz(true);
    filters.set_buzz(true);
    filters.set_detune(2);
    loop_from(&mut project, 2, 0);
    place_song_row(
        &mut project,
        0,
        &[
            [plays(0), plays(1), plays(2), plays(3)],
            [plays(3), plays(0), plays(1), plays(2)],
        ],
    );

    assert_exports_sample_for_sample(&project, 96 + 96);
}

#[test]
fn a_shorter_32_step_phrase_left_of_a_longer_one_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 32, 3, &[0, 4]);
    write_phrase(&mut project, 1, 32, 5, &[2, 3]);
    project
        .phrase_mut(PhraseId::new(0))
        .filters_mut()
        .set_reverb(2);
    place_song_row(
        &mut project,
        0,
        &[
            [plays(0), plays(1), None, None],
            [plays(1), plays(0), None, None],
        ],
    );

    assert_exports_sample_for_sample(&project, 160 + 160);
}

#[test]
fn a_phrase_loop_longer_than_its_chain_row_is_cut_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 32, 8, &[0, 5]);
    write_phrase(&mut project, 1, 8, 6, &[2]);
    loop_from(&mut project, 0, 4);
    place_song_row(
        &mut project,
        0,
        &[
            [plays(0), plays(1), None, None],
            [plays(1), plays(0), None, None],
        ],
    );

    assert_exports_sample_for_sample(&project, 48 + 48);
}

#[test]
fn an_all_looping_chain_row_timed_by_the_rightmost_track_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 3]);
    write_phrase(&mut project, 1, 16, 8, &[1]);
    loop_from(&mut project, 0, 0);
    loop_from(&mut project, 1, 3);
    place_song_row(&mut project, 0, &[[plays(0), plays(0), None, plays(1)]]);

    assert_exports_sample_for_sample(&project, 128);
}

#[test]
fn a_looping_song_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, SFX_INSTRUMENT]);
    write_phrase(&mut project, 1, 16, 6, &[2, 4]);
    write_phrase(&mut project, 2, 4, 6, &[WAVEFORM_INSTRUMENT]);
    loop_from(&mut project, 2, 0);
    place_song_row(&mut project, 0, &[[plays(1), None, None, None]]);
    place_song_row(
        &mut project,
        1,
        &[
            [plays(0), plays(1), None, None],
            [plays(2), plays(0), None, None],
        ],
    );
    project.set_song_loop_row(Some(1));

    let song_row_0_ticks = 96;
    let song_row_1_ticks = 96 + 48;
    assert_exports_sample_for_sample(&project, song_row_0_ticks + 3 * song_row_1_ticks);
}

#[test]
fn a_shorter_phrase_under_reverb_and_dampen_left_of_a_longer_one_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 3]);
    write_phrase(&mut project, 1, 16, 6, &[2, 4]);
    let filters = project.phrase_mut(PhraseId::new(0)).filters_mut();
    filters.set_reverb(2);
    filters.set_dampen(2);
    place_song_row(&mut project, 0, &[[plays(0), plays(1), None, None]]);

    assert_exports_sample_for_sample(&project, 96);
}

fn write_silent_steps_on(project: &mut Project, id: u8, audible: &[u8], silent_instrument: u8) {
    write_phrase(project, id, 16, 6, audible);
    let phrase = project.phrase_mut(PhraseId::new(id));
    for index in (1..16).step_by(2) {
        *phrase.step_mut(index) = Step::new(30, silent_instrument, 0, 0).unwrap();
    }
    let filters = phrase.filters_mut();
    filters.set_detune(1);
    filters.set_reverb(1);
}

#[test]
fn silent_steps_on_custom_instruments_under_detune_and_reverb_play_the_same_after_export() {
    let mut project = instrumented_project();
    write_silent_steps_on(&mut project, 0, &[3, SFX_INSTRUMENT], SFX_INSTRUMENT);
    write_silent_steps_on(&mut project, 1, &[2], WAVEFORM_INSTRUMENT);
    write_silent_steps_on(&mut project, 2, &[4], 0x0E);
    place_song_row(&mut project, 0, &[[plays(0), plays(1), plays(2), None]]);

    assert_exports_sample_for_sample(&project, 96);
}
