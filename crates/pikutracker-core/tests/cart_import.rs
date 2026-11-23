mod support;

use pikutracker_core::cart::Cart;
use pikutracker_core::cart_import::import;
use pikutracker_core::export::export;
use pikutracker_core::project::{
    ChainId, CustomInstrument, InstrumentSound, Phrase, PhraseId, Project, SfxInstrument,
    SongRowEnding, SongRowMarks, Step, WaveformInstrument,
};
use pikutracker_core::project_file;
use support::{TestCart, TestPattern, TestSfx, music_samples, render_music, render_song};

const CART_NAME: &str = "demo.p8";

fn imported(cart: &TestCart) -> (Project, Vec<String>) {
    let imported = import(&Cart::read(&cart.p8()).unwrap(), CART_NAME);
    let warnings = imported.warnings.into_iter().map(|w| w.text).collect();
    (imported.project, warnings)
}

fn pattern(channels: [Option<u8>; 4]) -> TestPattern {
    TestPattern::playing(channels)
}

const EDITOR_NOIZ_BUZZ_DETUNE_1_REVERB_2_DAMPEN_1: u8 = 135;
const EDITOR_DETUNE_1: u8 = 9;
const NOIZ_BUZZ_DETUNE_2_REVERB_1_DAMPEN_1: u8 = 118;
const REVERB_1: u8 = 24;
const BASS_AND_INAUDIBLE_BITS: u8 = 0x31;
const WAVEFORM_AND_INAUDIBLE_BITS: u8 = 0x85;
const FNV_1A_OF_THE_CLAIMED_BYTES: &str = "S00=6bd8a93f P00=d0cfb638";

fn two_sfx() -> Vec<TestSfx> {
    vec![
        TestSfx::with_pitches(8, &[12, 14]),
        TestSfx::with_pitches(8, &[19, 21]),
    ]
}

fn chain_on(project: &Project, row: usize, track: usize) -> ChainId {
    project.song_cell(row, track).unwrap()
}

#[test]
fn each_pattern_becomes_one_song_row_with_channel_n_on_track_n() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![
            pattern([Some(0), Some(1), Some(0), Some(1)]),
            pattern([Some(1), Some(0), Some(1), Some(0)]),
        ],
    };

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    for (row, sfx_on_tracks) in [[12, 19, 12, 19], [19, 12, 19, 12]].iter().enumerate() {
        for (track, pitch) in sfx_on_tracks.iter().enumerate() {
            let entry = project.chain_row_entry(row, track, 0).unwrap();
            assert_eq!(project.phrase(entry.phrase()).step(0).pitch(), *pitch);
        }
    }
    assert!(project.song_row_is_empty(2));
}

#[test]
fn a_phrase_keeps_every_value_of_its_sfx() {
    let mut sfx = TestSfx::with_pitches(23, &[12, 14]);
    sfx.editor_and_filters = EDITOR_NOIZ_BUZZ_DETUNE_1_REVERB_2_DAMPEN_1;
    sfx.notes[1].instrument = 5;
    sfx.notes[1].volume = 6;
    sfx.notes[1].effect = 3;
    sfx.notes[2].pitch = 40;
    sfx.notes[2].instrument = 2;
    let cart = TestCart {
        sfx: vec![sfx],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    let entry = project.chain_row_entry(0, 0, 0).unwrap();
    let phrase = project.phrase(entry.phrase());
    assert_eq!(phrase.length(), 32);
    assert_eq!(phrase.speed(), 23);
    let filters = phrase.filters();
    assert!(filters.noiz() && filters.buzz());
    assert_eq!(
        (filters.detune(), filters.reverb(), filters.dampen()),
        (1, 2, 1)
    );
    let step = phrase.step(1);
    assert_eq!(
        (
            step.pitch(),
            step.instrument(),
            step.volume(),
            step.effect()
        ),
        (14, 5, 6, 3)
    );
    let silent = phrase.step(2);
    assert_eq!(
        (silent.pitch(), silent.instrument(), silent.volume()),
        (40, 2, 0)
    );
}

#[test]
fn a_disabled_channel_becomes_an_empty_chain_entry() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), None, Some(1), None])],
    };

    let (project, _) = imported(&cart);

    let chain = chain_on(&project, 0, 1);
    assert_eq!(project.chain(chain).entry(0), None);
    assert_eq!(project.chain_row_entry(0, 3, 0), None);
    assert!(project.chain_row_entry(0, 2, 0).is_some());
}

#[test]
fn identical_chains_and_exact_match_phrases_are_shared() {
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[12]),
            TestSfx::with_pitches(8, &[19]),
            TestSfx::with_pitches(8, &[12]),
        ],
        patterns: vec![
            pattern([Some(0), Some(1), None, None]),
            pattern([Some(2), Some(0), None, None]),
        ],
    };

    let (project, _) = imported(&cart);

    let shared = chain_on(&project, 0, 0);
    assert_eq!(chain_on(&project, 1, 0), shared);
    assert_eq!(chain_on(&project, 1, 1), shared);
    assert_ne!(chain_on(&project, 0, 1), shared);
    assert_eq!(chain_on(&project, 0, 2), chain_on(&project, 1, 3));
}

#[test]
fn phrases_that_differ_only_in_a_silent_step_are_not_shared() {
    let mut other = TestSfx::with_pitches(8, &[12]);
    other.notes[5].pitch = 33;
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[12]), other],
        patterns: vec![pattern([Some(0), Some(1), None, None])],
    };

    let (project, _) = imported(&cart);

    let first = project.chain_row_entry(0, 0, 0).unwrap().phrase();
    let second = project.chain_row_entry(0, 1, 0).unwrap().phrase();
    assert_ne!(first, second);
}

#[test]
fn the_claim_covers_exactly_the_songs_sfx_and_patterns() {
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[1]),
            TestSfx::with_pitches(8, &[12]),
            TestSfx::with_pitches(8, &[2]),
            TestSfx::with_pitches(8, &[19]),
        ],
        patterns: vec![
            pattern([Some(3), None, None, None]),
            pattern([Some(1), Some(3), None, None]),
        ],
    };

    let (project, _) = imported(&cart);

    let target = project.target_cart().unwrap();
    assert_eq!(target.path, CART_NAME);
    assert_eq!(target.claim.sfx, [1, 3]);
    assert_eq!(target.claim.first_and_last_pattern, Some((0, 1)));
}

#[test]
fn the_claim_includes_the_sfx_that_notes_use_as_instruments() {
    let mut sfx = TestSfx::with_pitches(8, &[12]);
    sfx.notes[0].custom = true;
    sfx.notes[0].instrument = 6;
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[1]), sfx],
        patterns: vec![pattern([Some(1), None, None, None])],
    };

    let (project, _) = imported(&cart);

    assert_eq!(project.target_cart().unwrap().claim.sfx, [1, 6]);
}

#[test]
fn the_claimed_bytes_hash_is_stored_with_one_checksum_per_slot() {
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[12, 14])],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    assert_eq!(
        project.target_cart().unwrap().claimed_bytes_hash.as_deref(),
        Some(FNV_1A_OF_THE_CLAIMED_BYTES)
    );
}

#[test]
fn every_pattern_from_the_first_non_empty_to_the_last_becomes_one_song_row() {
    let mut stop = pattern([Some(1), None, None, None]);
    stop.stop = true;
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![
            pattern([None, None, None, None]),
            pattern([Some(0), None, None, None]),
            stop,
            pattern([None, None, None, None]),
            pattern([Some(1), None, None, None]),
        ],
    };

    let (project, warnings) = imported(&cart);

    let first_pitch = |row| {
        let entry = project.chain_row_entry(row, 0, 0).unwrap();
        project.phrase(entry.phrase()).step(0).pitch()
    };
    assert_eq!(first_pitch(0), 12);
    assert_eq!(first_pitch(1), 19);
    assert!(project.song_row_is_empty(2));
    assert_eq!(first_pitch(3), 19);
    assert!(project.song_row_is_empty(4));
    assert_eq!(project.song_row_marks(1).ending, Some(SongRowEnding::Stop));
    assert_eq!(
        project.target_cart().unwrap().claim.first_and_last_pattern,
        Some((1, 4))
    );
    assert_eq!(warnings, Vec::<String>::new());
}

#[test]
fn each_patterns_flow_flags_become_the_marks_of_the_same_name_on_its_song_row() {
    let mut start_and_stop = pattern([Some(0), None, None, None]);
    start_and_stop.loop_start = true;
    start_and_stop.stop = true;
    let mut back = pattern([Some(1), None, None, None]);
    back.loop_back = true;
    let mut empty_start = pattern([None, None, None, None]);
    empty_start.loop_start = true;
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![
            start_and_stop,
            back,
            empty_start,
            pattern([Some(0), None, None, None]),
        ],
    };

    let (project, _) = imported(&cart);

    let marks = |loop_start, ending| SongRowMarks { loop_start, ending };
    assert_eq!(
        project.song_row_marks(0),
        marks(true, Some(SongRowEnding::Stop))
    );
    assert_eq!(
        project.song_row_marks(1),
        marks(false, Some(SongRowEnding::LoopBack))
    );
    assert_eq!(project.song_row_marks(2), marks(true, None));
    assert_eq!(project.song_row_marks(3), marks(false, None));
}

#[test]
fn the_claim_covers_every_sfx_the_imported_patterns_play_and_the_rest_stay_loose() {
    let mut stop = pattern([Some(0), None, None, None]);
    stop.stop = true;
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[12]),
            TestSfx::with_pitches(8, &[14]),
            TestSfx::with_pitches(8, &[16]),
        ],
        patterns: vec![stop, pattern([None, Some(2), None, None])],
    };

    let (project, warnings) = imported(&cart);

    let claim = &project.target_cart().unwrap().claim;
    assert_eq!(claim.sfx, [0, 2]);
    assert_eq!(claim.first_and_last_pattern, Some((0, 1)));
    assert_eq!(phrases_starting_on(&project, 14).len(), 1);
    assert_eq!(
        warnings,
        ["SFX 01 KEPT AS A LOOSE PHRASE OUTSIDE THE CLAIM."]
    );
}

#[test]
fn importing_and_exporting_straight_back_keeps_the_claimed_pattern_flags() {
    let mut start = pattern([Some(0), None, None, None]);
    start.loop_start = true;
    let mut back = pattern([Some(1), Some(0), None, None]);
    back.loop_back = true;
    let mut stop = pattern([Some(1), None, None, None]);
    stop.stop = true;
    let mut start_and_stop = pattern([Some(0), None, None, None]);
    start_and_stop.loop_start = true;
    start_and_stop.stop = true;
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![
            start,
            back,
            stop,
            pattern([None, None, None, None]),
            start_and_stop,
            pattern([Some(0), None, None, None]),
        ],
    };
    let original = cart.p8();
    let (project, _) = imported(&cart);
    let claim = project.target_cart().unwrap().claim.clone();

    let exported = export(&project, &claim, &original).unwrap();

    let (before, after) = (
        Cart::read(&original).unwrap(),
        Cart::read(&exported.cart_bytes).unwrap(),
    );
    let flags = |cart: &Cart, slot| {
        let pattern = cart.pattern(slot);
        (pattern.loop_start(), pattern.loop_back(), pattern.stop())
    };
    for slot in 0..=5 {
        assert_eq!(flags(&after, slot), flags(&before, slot), "pattern {slot}");
    }
}

#[test]
fn loop_back_jumps_to_the_nearest_loop_start_before_it() {
    let mut start = pattern([Some(1), None, None, None]);
    start.loop_start = true;
    let mut last = pattern([Some(0), None, None, None]);
    last.loop_back = true;
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), None, None, None]), start, last],
    };

    let (project, _) = imported(&cart);

    assert!(project.song_row_marks(1).loop_start);
    assert_eq!(
        project.song_row_marks(2).ending,
        Some(SongRowEnding::LoopBack)
    );
}

#[test]
fn a_song_that_runs_into_an_empty_pattern_ends_there_without_a_loop() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    assert_eq!(project.song_row_marks(0), SongRowMarks::default());
    assert_eq!(
        project.target_cart().unwrap().claim.first_and_last_pattern,
        Some((0, 0))
    );
}

#[test]
fn a_cart_with_no_music_gives_an_empty_song_and_an_empty_claim() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![],
    };

    let (project, _) = imported(&cart);

    assert!(project.song_row_is_empty(0));
    assert_eq!(project.song_row_marks(0), SongRowMarks::default());
    let claim = &project.target_cart().unwrap().claim;
    assert!(claim.sfx.is_empty());
    assert_eq!(claim.first_and_last_pattern, None);
    assert_eq!(phrases_starting_on(&project, 12).len(), 1);
    assert_eq!(phrases_starting_on(&project, 19).len(), 1);
}

#[test]
fn a_shorter_channel_right_of_the_longest_is_simple() {
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(16, &[12]),
            TestSfx::with_pitches(8, &[19]),
        ],
        patterns: vec![pattern([Some(0), Some(1), None, None])],
    };

    let (_, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
}

fn render_cart(cart: &TestCart) -> Vec<f32> {
    let cart = Cart::read(&cart.p8()).unwrap();
    render_music(&cart, 0, music_samples(&cart, 0))
}

#[test]
fn an_import_with_no_warnings_renders_the_same_samples_as_the_cart() {
    let mut lead = TestSfx::with_pitches(6, &[24, 27, 31, 36, 31, 27, 24, 19]);
    lead.editor_and_filters = 2 * 24;
    lead.notes[3].effect = 2;
    let mut bass = TestSfx::with_pitches(12, &[12, 12, 7, 7]);
    bass.notes[1].instrument = 3;
    let mut slide = TestSfx::with_pitches(6, &[40, 44]);
    slide.notes[1].effect = 1;
    slide.notes[2].pitch = 48;
    slide.notes[2].volume = 2;
    slide.notes[2].instrument = 2;
    let mut stop = pattern([Some(1), Some(2), Some(0), None]);
    stop.stop = true;
    let cart = TestCart {
        sfx: vec![lead, bass, slide],
        patterns: vec![
            pattern([Some(1), Some(0), None, None]),
            pattern([Some(1), None, Some(2), Some(0)]),
            stop,
        ],
    };

    let (project, warnings) = imported(&cart);
    assert_eq!(warnings, Vec::<String>::new());

    let expected = render_cart(&cart);
    let rendered = render_song(project, expected.len());

    assert!(expected.iter().any(|sample| sample.abs() > 0.01));
    let first_difference = expected
        .iter()
        .zip(&rendered)
        .position(|(expected, rendered)| expected != rendered);
    assert_eq!(first_difference, None);
}

fn snapshot_cart() -> TestCart {
    let mut first = pattern([Some(0), Some(1), None, None]);
    first.loop_start = true;
    let mut last = pattern([Some(1), None, Some(0), None]);
    last.loop_back = true;
    TestCart {
        sfx: vec![
            TestSfx::with_pitches(16, &[12, 14]),
            TestSfx::with_pitches(16, &[19]),
        ],
        patterns: vec![first, last],
    }
}

#[test]
fn an_imported_project_saves_as_the_expected_ptrk_text() {
    let (project, warnings) = imported(&snapshot_cart());

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(
        project_file::write(&project),
        include_str!("snapshots/imported_cart.ptrk")
    );
}

#[test]
fn a_png_cart_imports_to_the_same_project_as_its_text_twin() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), Some(1), None, None])],
    };

    let from_text = import(&Cart::read(&cart.p8()).unwrap(), CART_NAME);
    let from_pixels = import(&Cart::read(&cart.png()).unwrap(), CART_NAME);

    assert_eq!(from_text.project, from_pixels.project);
}

fn melody_on_sfx_1() -> TestSfx {
    let mut melody = TestSfx::with_pitches(16, &[36, 40]);
    melody.notes[0].custom = true;
    melody.notes[0].instrument = 1;
    melody.notes[1].instrument = 3;
    melody
}

#[test]
fn an_sfx_used_as_an_instrument_becomes_an_sfx_instrument_numbered_8_above_its_slot() {
    let mut instrument = TestSfx::with_pitches(4, &[24, 31, 28]);
    instrument.loop_start = 1;
    instrument.loop_end = 3;
    instrument.editor_and_filters = EDITOR_DETUNE_1;
    instrument.notes[1].instrument = 3;
    instrument.notes[2].custom = true;
    let cart = TestCart {
        sfx: vec![melody_on_sfx_1(), instrument],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    let phrase = project.phrase(project.chain_row_entry(0, 0, 0).unwrap().phrase());
    assert_eq!(phrase.step(0).instrument(), 9);
    assert_eq!(phrase.step(1).instrument(), 3);
    let mut steps = vec![Step::new(0, 0, 0, 0).unwrap(); 32];
    steps[0] = Step::new(24, 0, 3, 0).unwrap();
    steps[1] = Step::new(31, 3, 3, 0).unwrap();
    steps[2] = Step::new(28, 0, 3, 0).unwrap();
    let Some(CustomInstrument {
        sound: InstrumentSound::Sfx(imported),
        ..
    }) = project.custom_instrument(9)
    else {
        panic!("no SFX Instrument 09");
    };
    let mut filters = phrase.filters();
    filters.set_detune(1);
    let expected = SfxInstrument {
        steps,
        speed: 4,
        filters,
        loop_start: 1,
        loop_end: 3,
    };
    assert_eq!(imported, &expected);
    assert_eq!(project.custom_instruments().count(), 1);
    assert_eq!(project.target_cart().unwrap().claim.sfx, [0, 1]);
}

#[test]
fn an_import_export_round_trip_keeps_every_instrument_in_its_slot() {
    let mut melody = TestSfx::with_pitches(16, &[36, 40]);
    melody.notes[0].custom = true;
    melody.notes[0].instrument = 5;
    melody.notes[1].instrument = 3;
    let silent = TestSfx::with_pitches(8, &[]);
    let instrument = TestSfx::with_pitches(4, &[24, 31, 28]);
    let cart = TestCart {
        sfx: vec![melody, silent, silent, silent, silent, instrument],
        patterns: vec![pattern([Some(0), None, None, None])],
    };
    let original = cart.p8();
    let (project, _) = imported(&cart);
    let claim = project.target_cart().unwrap().claim.clone();

    let exported = export(&project, &claim, &original).unwrap();

    let (before, after) = (
        Cart::read(&original).unwrap(),
        Cart::read(&exported.cart_bytes).unwrap(),
    );
    assert!(project.custom_instrument(0x0D).is_some());
    assert_eq!(after.sfx_bytes(5)[..64], before.sfx_bytes(5)[..64]);
    assert_eq!(after.sfx_bytes(0)[..4], before.sfx_bytes(0)[..4]);
}

#[test]
fn an_sfx_instrument_with_len_keeps_only_the_steps_inside_len() {
    let mut instrument = TestSfx::with_pitches(4, &[24, 31, 28]);
    instrument.loop_start = 2;
    let cart = TestCart {
        sfx: vec![melody_on_sfx_1(), instrument],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    let Some(InstrumentSound::Sfx(imported)) = project.custom_instrument(9).map(|i| &i.sound)
    else {
        panic!("no SFX Instrument 09");
    };
    assert_eq!(imported.steps.len(), 2);
    assert_eq!((imported.loop_start, imported.loop_end), (0, 0));
}

#[test]
fn an_instrument_sfx_that_a_pattern_plays_directly_is_also_a_phrase() {
    let cart = TestCart {
        sfx: vec![melody_on_sfx_1(), TestSfx::with_pitches(4, &[24, 31])],
        patterns: vec![pattern([Some(0), Some(1), None, None])],
    };

    let (project, _) = imported(&cart);

    let played = project.chain_row_entry(0, 1, 0).unwrap().phrase();
    assert_eq!(project.phrase(played).step(1).pitch(), 31);
    assert!(project.custom_instrument(9).is_some());
}

#[test]
fn a_waveform_instrument_keeps_its_samples_bass_and_filters_and_drops_the_rest_silently() {
    let mut wave = TestSfx::with_pitches(1, &[]);
    for (index, note) in wave.notes.iter_mut().enumerate() {
        note.pitch = (index * 2) as u8;
        note.instrument = (index % 8) as u8;
        note.volume = 7 - (index % 8) as u8;
        note.effect = (index % 8) as u8;
        note.custom = index % 2 == 0;
    }
    wave.speed = BASS_AND_INAUDIBLE_BITS;
    wave.loop_start = WAVEFORM_AND_INAUDIBLE_BITS;
    wave.loop_end = 9;
    wave.editor_and_filters = NOIZ_BUZZ_DETUNE_2_REVERB_1_DAMPEN_1;
    let cart = TestCart {
        sfx: vec![melody_on_sfx_1(), wave],
        patterns: vec![pattern([Some(0), None, None, None])],
    };
    let read = Cart::read(&cart.p8()).unwrap();

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    let expected = WaveformInstrument {
        samples: std::array::from_fn(|index| read.sfx_bytes(1)[index] as i8),
        bass: true,
        detune: 2,
        reverb: 1,
        dampen: 1,
    };
    assert_eq!(
        project.custom_instrument(9).map(|i| &i.sound),
        Some(&InstrumentSound::Waveform(expected))
    );
}

#[test]
fn an_import_with_sfx_instruments_renders_the_same_samples_as_the_cart() {
    let mut melody = TestSfx::with_pitches(12, &[36, 40, 43, 36]);
    melody.notes[0].custom = true;
    melody.notes[0].instrument = 1;
    melody.notes[2].custom = true;
    melody.notes[2].instrument = 1;
    melody.notes[3].custom = true;
    melody.notes[3].instrument = 2;
    let mut pluck = TestSfx::with_pitches(3, &[24, 36, 31, 24]);
    pluck.notes[1].instrument = 3;
    pluck.loop_start = 2;
    pluck.loop_end = 4;
    pluck.editor_and_filters = REVERB_1;
    let mut short = TestSfx::with_pitches(2, &[24, 28, 31]);
    short.loop_start = 2;
    let mut stop = pattern([Some(0), None, None, None]);
    stop.stop = true;
    let cart = TestCart {
        sfx: vec![melody, pluck, short],
        patterns: vec![stop],
    };

    let (project, warnings) = imported(&cart);
    assert_eq!(warnings, Vec::<String>::new());

    let expected = render_cart(&cart);
    let rendered = render_song(project, expected.len());

    assert!(expected.iter().any(|sample| sample.abs() > 0.01));
    let first_difference = expected
        .iter()
        .zip(&rendered)
        .position(|(expected, rendered)| expected != rendered);
    assert_eq!(first_difference, None);
}

fn phrases_starting_on(project: &Project, pitch: u8) -> Vec<Phrase> {
    (0..=255)
        .map(|id| *project.phrase(PhraseId::new(id)))
        .filter(|phrase| phrase.step(0).pitch() == pitch && !phrase.step(0).is_silent())
        .collect()
}

#[test]
fn every_other_sfx_becomes_a_loose_unclaimed_phrase_with_its_loop() {
    let mut looping = TestSfx::with_pitches(5, &[40, 41, 42, 43]);
    looping.loop_start = 1;
    looping.loop_end = 3;
    let mut with_len = TestSfx::with_pitches(7, &[45, 46, 47]);
    with_len.loop_start = 2;
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[12]),
            looping,
            TestSfx::with_pitches(8, &[]),
            with_len,
        ],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, warnings) = imported(&cart);

    assert_eq!(project.target_cart().unwrap().claim.sfx, [0]);
    let [kept] = phrases_starting_on(&project, 40)[..] else {
        panic!("SFX 01 is not kept once");
    };
    assert_eq!(
        (kept.loop_start(), kept.loop_end(), kept.speed()),
        (1, 3, 5)
    );
    assert_eq!(kept.step(3).pitch(), 43);
    let [kept] = phrases_starting_on(&project, 45)[..] else {
        panic!("SFX 03 is not kept once");
    };
    assert_eq!((kept.length(), kept.speed()), (2, 7));
    assert_eq!(kept.step(2).pitch(), 47);
    assert_eq!(
        warnings,
        ["SFX 01 03 KEPT AS LOOSE PHRASES OUTSIDE THE CLAIM."]
    );
}

#[test]
fn an_sfx_that_only_a_loose_phrase_uses_as_an_instrument_is_a_custom_instrument_only() {
    let mut loose = TestSfx::with_pitches(8, &[40]);
    loose.notes[0].custom = true;
    loose.notes[0].instrument = 2;
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[12]),
            loose,
            TestSfx::with_pitches(4, &[24]),
        ],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    assert!(project.custom_instrument(10).is_some());
    assert!(phrases_starting_on(&project, 24).is_empty());
    assert_eq!(
        phrases_starting_on(&project, 40)[0].step(0).instrument(),
        10
    );
    assert_eq!(project.target_cart().unwrap().claim.sfx, [0]);
}
