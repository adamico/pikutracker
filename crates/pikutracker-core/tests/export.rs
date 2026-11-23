mod support;

use pikutracker_core::cart::Cart;
use pikutracker_core::export::{ExportFailure, export};
use pikutracker_core::project::{
    Chain, ChainEntry, ChainId, Claim, CustomInstrument, Filters, InstrumentSound, PhraseId,
    Project, SfxInstrument, SongRowMarks, Step, WaveformInstrument,
};
use support::{HEADER, assert_same_samples, render_notes, render_song, sound_of};

const EMPTY_PATTERN: [u8; 4] = [0x41, 0x42, 0x43, 0x44];

fn blank_cart() -> Vec<u8> {
    format!("{HEADER}__lua__\nprint(1)\n").into_bytes()
}

fn claim(sfx: &[u8], patterns: (u8, u8)) -> Claim {
    Claim {
        sfx: sfx.to_vec(),
        first_and_last_pattern: Some(patterns),
    }
}

fn write_phrase(project: &mut Project, id: u8, length: i32, speed: i32) {
    let phrase = project.phrase_mut(PhraseId::new(id));
    phrase.set_length(length);
    phrase.set_speed(speed);
    *phrase.step_mut(0) = Step::new(12, 3, 5, 1).unwrap();
}

fn place(project: &mut Project, row: usize, track: usize, chain: u8, phrase: u8) {
    let mut entries = Chain::default();
    *entries.entry_mut(0) = Some(ChainEntry::new(PhraseId::new(phrase)));
    *project.chain_mut(ChainId::new(chain)) = entries;
    project.set_song_cell(row, track, Some(ChainId::new(chain)));
}

fn one_row_project() -> Project {
    let mut project = Project::new();
    write_phrase(&mut project, 0, 16, 20);
    place(&mut project, 0, 0, 0, 0);
    project
}

fn mark(project: &mut Project, row: usize, loop_start: bool, loop_back: bool, stop: bool) {
    let marks = SongRowMarks {
        loop_start,
        loop_back,
        stop,
    };
    project.set_song_row_marks(row, marks);
}

fn exported_cart(project: &Project, claim: &Claim) -> Cart {
    let exported = export(project, claim, &blank_cart()).unwrap();
    Cart::read(&exported.cart_bytes).unwrap()
}

#[test]
fn a_phrase_becomes_an_sfx_with_its_speed_len_and_notes() {
    let cart = exported_cart(&one_row_project(), &claim(&[8], (0, 0)));

    let sfx = cart.sfx_bytes(8);
    let pitch_12_instrument_3_volume_5_effect_1 = 12u16 | 3 << 6 | 5 << 9 | 1 << 12;
    assert_eq!(
        sfx[..2],
        pitch_12_instrument_3_volume_5_effect_1.to_le_bytes()
    );
    let (editor_mode_bit, speed, len_as_loop_start, loop_end) = (0x01, 20, 16, 0);
    assert_eq!(
        sfx[64..],
        [editor_mode_bit, speed, len_as_loop_start, loop_end]
    );
}

#[test]
fn a_32_step_phrase_needs_no_len() {
    let mut project = one_row_project();
    project.phrase_mut(PhraseId::new(0)).set_length(32);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx_bytes(8)[66..], [0, 0]);
}

#[test]
fn filters_come_from_the_phrase() {
    let mut project = one_row_project();
    let filters = project.phrase_mut(PhraseId::new(0)).filters_mut();
    filters.set_noiz(true);
    filters.set_buzz(true);
    filters.set_detune(1);
    filters.set_reverb(2);
    filters.set_dampen(1);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    let (editor, noiz, buzz, detune_unit, reverb_unit, dampen_unit) = (1, 2, 4, 8, 24, 72);
    assert_eq!(
        cart.sfx_bytes(8)[64],
        editor + noiz + buzz + detune_unit + 2 * reverb_unit + dampen_unit
    );
}

#[test]
fn a_transpose_is_baked_into_the_notes() {
    let mut project = one_row_project();
    let chain = project.chain_mut(ChainId::new(0));
    chain.entry_mut(0).as_mut().unwrap().set_transpose(7);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx(8).note(0).pitch, 19);
}

#[test]
fn each_chain_row_becomes_one_pattern_in_song_order() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 16, 10);
    let mut chain = *project.chain(ChainId::new(0));
    *chain.entry_mut(1) = Some(ChainEntry::new(PhraseId::new(1)));
    *project.chain_mut(ChainId::new(0)) = chain;
    place(&mut project, 1, 0, 1, 1);

    let cart = exported_cart(&project, &claim(&[8, 9], (4, 7)));

    assert_eq!(cart.pattern_bytes(4), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(5), [9, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(6), [9, 0x42, 0x43, 0x44]);
}

#[test]
fn track_n_exports_to_channel_n_minus_1_with_nothing_to_pick() {
    let mut project = Project::new();
    for (track, phrase, speed) in [(1, 1, 20), (3, 3, 10)] {
        write_phrase(&mut project, phrase, 16, speed);
        place(&mut project, 0, track, phrase, phrase);
    }

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 0)));

    let without_flags = cart.pattern_bytes(0).map(|byte| byte & 0x7F);
    assert_eq!(without_flags, [0x41, 8, 0x43, 9]);
}

#[test]
fn loop_start_and_loop_back_on_one_row_flag_its_pattern() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    mark(&mut project, 1, true, true, false);

    let cart = exported_cart(&project, &claim(&[8], (2, 3)));

    assert_eq!(cart.pattern_bytes(2), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(3), [8 | 0x80, 0x42 | 0x80, 0x43, 0x44]);
}

#[test]
fn identical_sfx_share_a_slot_and_slots_fill_in_ascending_order() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    write_phrase(&mut project, 2, 8, 9);
    place(&mut project, 2, 0, 2, 2);

    let exported = export(&project, &claim(&[30, 10], (0, 2)), &blank_cart()).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    assert_eq!(exported.sfx_count, 2);
    assert_eq!(exported.pattern_count, 3);
    assert_eq!(cart.pattern(0).channel(0), Some(10));
    assert_eq!(cart.pattern(1).channel(0), Some(10));
    assert_eq!(cart.pattern(2).channel(0), Some(30));
}

#[test]
fn unused_claimed_slots_are_reset_to_empty() {
    let sfx = format!("01100000{}", "18350".repeat(32));
    let original = format!("{HEADER}__sfx__\n{sfx}\n{sfx}\n__music__\n00 00014344\n00 00014344\n");

    let exported = export(
        &one_row_project(),
        &claim(&[0, 1], (0, 1)),
        original.as_bytes(),
    );

    let cart = Cart::read(&exported.unwrap().cart_bytes).unwrap();
    let mut blank = [0; 68];
    blank[65] = 16;
    assert_eq!(cart.sfx_bytes(1), &blank);
    assert_eq!(cart.pattern_bytes(1), EMPTY_PATTERN);
}

#[test]
fn the_new_hash_describes_the_exported_claimed_bytes() {
    let claim = claim(&[8], (0, 1));

    let exported = export(&one_row_project(), &claim, &blank_cart()).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    assert_eq!(exported.claimed_bytes_hash, cart.claimed_bytes_hash(&claim));
}

#[test]
fn every_failure_is_reported_at_once() {
    let failures = export(&Project::new(), &Claim::default(), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::EmptyClaim, ExportFailure::EmptySong]
    );
}

#[test]
fn a_note_out_of_range_does_not_hide_a_claim_that_is_too_small() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    let chain = project.chain_mut(ChainId::new(0));
    chain.entry_mut(0).as_mut().unwrap().set_transpose(60);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert!(matches!(failures[0], ExportFailure::NoteOutOfRange { .. }));
    assert_eq!(
        failures.last(),
        Some(&ExportFailure::TooFewPatterns {
            needed: 2,
            claimed: 1
        })
    );
}

#[test]
fn marks_become_flags_over_bytes_that_were_there() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    mark(&mut project, 0, true, false, false);
    mark(&mut project, 1, false, true, false);
    let mut original = blank_cart();
    original.extend(b"__music__\n07 08424344\n07 08424344\n");

    let exported = export(&project, &claim(&[8], (0, 1)), &original).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    assert_eq!(cart.pattern_bytes(0), [8 | 0x80, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(1), [8, 0x42 | 0x80, 0x43, 0x44]);
}

#[test]
fn a_stop_mark_becomes_the_stop_flag() {
    let mut project = one_row_project();
    mark(&mut project, 0, false, false, true);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.pattern_bytes(0), [8, 0x42, 0x43 | 0x80, 0x44]);
}

#[test]
fn a_row_with_loop_back_and_stop_exports_both_flags() {
    let mut project = one_row_project();
    mark(&mut project, 0, true, true, true);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(
        cart.pattern_bytes(0),
        [8 | 0x80, 0x42 | 0x80, 0x43 | 0x80, 0x44]
    );
}

#[test]
fn a_song_row_of_several_chain_rows_flags_its_first_and_last_patterns() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 9);
    *project.chain_mut(ChainId::new(0)).entry_mut(1) = Some(ChainEntry::new(PhraseId::new(1)));
    *project.chain_mut(ChainId::new(0)).entry_mut(2) = Some(ChainEntry::new(PhraseId::new(0)));
    place(&mut project, 1, 0, 1, 1);
    mark(&mut project, 0, true, true, false);
    mark(&mut project, 1, true, false, true);

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 3)));

    assert_eq!(cart.pattern_bytes(0), [8 | 0x80, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(1), [9, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(2), [8, 0x42 | 0x80, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(3), [9 | 0x80, 0x42, 0x43 | 0x80, 0x44]);
}

#[test]
fn an_empty_song_row_between_played_rows_becomes_an_empty_pattern_with_its_marks() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 9);
    place(&mut project, 2, 0, 1, 1);
    mark(&mut project, 1, false, false, true);

    let exported = export(&project, &claim(&[8, 9], (0, 2)), &blank_cart()).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    assert_eq!(exported.pattern_count, 3);
    assert_eq!(cart.pattern_bytes(0), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(1), [0x41, 0x42, 0x43 | 0x80, 0x44]);
    assert_eq!(cart.pattern_bytes(2), [9, 0x42, 0x43, 0x44]);
}

#[test]
fn a_loop_back_with_no_loop_start_adds_no_loop_start_flag_on_a_first_pattern_past_pattern_0() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    mark(&mut project, 1, false, true, false);

    let cart = exported_cart(&project, &claim(&[8], (2, 3)));

    assert_eq!(cart.pattern_bytes(2), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(3), [8, 0x42 | 0x80, 0x43, 0x44]);
}

#[test]
fn a_loop_back_to_song_row_00_needs_no_loop_start_on_pattern_0() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    mark(&mut project, 1, false, true, false);

    let cart = exported_cart(&project, &claim(&[8], (0, 1)));

    assert_eq!(cart.pattern_bytes(0), [8, 0x42, 0x43, 0x44]);
}

#[test]
fn marks_after_the_last_played_song_row_are_not_exported() {
    let mut project = one_row_project();
    mark(&mut project, 5, true, false, true);

    let exported = export(&project, &claim(&[8], (0, 1)), &blank_cart()).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    assert_eq!(exported.pattern_count, 1);
    assert_eq!(cart.pattern_bytes(0), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(1), EMPTY_PATTERN);
}

#[test]
fn a_claim_naming_a_slot_twice_exports_into_that_slot_once() {
    let cart = exported_cart(&one_row_project(), &claim(&[8, 8], (0, 0)));

    assert_eq!(cart.pattern(0).channel(0), Some(8));
}

#[test]
fn a_claim_naming_slots_a_cart_does_not_have_fails_the_export() {
    for bad in [
        claim(&[64], (0, 0)),
        claim(&[8], (0, 64)),
        claim(&[8], (5, 2)),
    ] {
        let failures = export(&one_row_project(), &bad, &blank_cart()).unwrap_err();

        assert!(failures.contains(&ExportFailure::BadClaim), "{failures:?}");
    }
}

#[test]
fn too_few_claimed_slots_say_how_many_are_needed() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 9);
    place(&mut project, 1, 0, 1, 1);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [
            ExportFailure::TooFewSfx {
                needed: 2,
                claimed: 1
            },
            ExportFailure::TooFewPatterns {
                needed: 2,
                claimed: 1
            },
        ]
    );
}

#[test]
fn an_audible_note_pushed_out_of_range_is_named() {
    let mut project = one_row_project();
    let chain = project.chain_mut(ChainId::new(0));
    chain.entry_mut(0).as_mut().unwrap().set_transpose(60);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NoteOutOfRange {
            phrase: 0,
            step: 0,
            track: 0,
            song_row: 0,
            chain_row: 0,
        }]
    );
}

#[test]
fn a_silent_step_pushed_out_of_range_is_clamped() {
    let mut project = one_row_project();
    *project.phrase_mut(PhraseId::new(0)).step_mut(1) = Step::new(60, 0, 0, 0).unwrap();
    project
        .chain_mut(ChainId::new(0))
        .entry_mut(0)
        .as_mut()
        .unwrap()
        .set_transpose(10);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx(8).note(1).pitch, 63);
}

#[test]
fn a_silent_step_an_arpeggio_reads_pushed_out_of_range_is_named() {
    let mut project = one_row_project();
    let phrase = project.phrase_mut(PhraseId::new(0));
    *phrase.step_mut(0) = Step::new(24, 0, 5, 6).unwrap();
    *phrase.step_mut(1) = Step::new(63, 0, 0, 0).unwrap();
    project
        .chain_mut(ChainId::new(0))
        .entry_mut(0)
        .as_mut()
        .unwrap()
        .set_transpose(1);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NoteOutOfRange {
            phrase: 0,
            step: 1,
            track: 0,
            song_row: 0,
            chain_row: 0,
        }]
    );
}

#[test]
fn an_empty_song_has_nothing_to_export() {
    let project = Project::new();

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(failures, [ExportFailure::EmptySong]);
}

#[test]
fn a_cart_that_cannot_be_read_fails_the_export() {
    let failures = export(&one_row_project(), &claim(&[8], (0, 0)), b"not a cart").unwrap_err();

    assert!(matches!(failures[..], [ExportFailure::UnreadableCart(_)]));
}

#[test]
fn a_phrase_loop_is_written_as_the_sfx_loop_byte_for_byte() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 32, 20);
    place(&mut project, 0, 1, 1, 1);
    project.phrase_mut(PhraseId::new(0)).set_loop(4, 12);

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 0)));

    assert_eq!(cart.sfx_bytes(8)[66..], [4, 12]);
    assert_eq!(cart.sfx_bytes(9)[66..], [0, 0]);
}

#[test]
fn a_pair_that_plays_all_32_steps_is_written_as_entered() {
    let mut project = one_row_project();
    project.phrase_mut(PhraseId::new(0)).set_loop(12, 4);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx_bytes(8)[66..], [12, 4]);
}

#[test]
fn in_an_all_looping_chain_row_the_longest_phrase_plays_once_through_with_len() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 20);
    place(&mut project, 0, 1, 1, 1);
    project.phrase_mut(PhraseId::new(0)).set_loop(4, 16);
    project.phrase_mut(PhraseId::new(1)).set_loop(2, 8);

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 0)));

    assert_eq!(cart.sfx_bytes(8)[66..], [16, 0]);
    assert_eq!(cart.sfx_bytes(9)[66..], [2, 8]);
}

#[test]
fn a_shorter_phrase_on_the_left_loops_on_the_step_after_its_length() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 32, 20);
    place(&mut project, 0, 1, 1, 1);

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 0)));

    assert_eq!(cart.sfx_bytes(8)[66..], [16, 17]);
    assert_eq!(cart.sfx_bytes(9)[66..], [0, 0]);
}

#[test]
fn a_shorter_32_step_phrase_on_the_left_loops_past_its_last_step() {
    let mut project = one_row_project();
    project.phrase_mut(PhraseId::new(0)).set_length(32);
    write_phrase(&mut project, 1, 32, 30);
    place(&mut project, 0, 1, 1, 1);

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 0)));

    assert_eq!(cart.sfx_bytes(8)[66..], [32, 33]);
}

#[test]
fn a_shorter_phrase_on_the_right_is_simple() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 20);
    place(&mut project, 0, 1, 1, 1);

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 0)));

    assert_eq!(cart.pattern_bytes(0), [8, 9, 0x43, 0x44]);
}

fn arpeggio_past_len() -> Project {
    let mut project = one_row_project();
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_length(6);
    phrase.set_speed(8);
    for index in 0..4 {
        *phrase.step_mut(index) = Step::new(24, 0, 5, 0).unwrap();
    }
    *phrase.step_mut(4) = Step::new(30, 0, 5, 6).unwrap();
    *phrase.step_mut(5) = Step::new(34, 0, 5, 6).unwrap();
    *phrase.step_mut(6) = Step::new(40, 0, 0, 0).unwrap();
    *phrase.step_mut(7) = Step::new(44, 0, 0, 0).unwrap();
    project
}

#[test]
fn an_arpeggio_reading_steps_past_len_sounds_the_same_after_export() {
    let project = arpeggio_past_len();

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    let rendered = render_notes(sound_of(&cart, 8), &[0, 1, 2, 3, 4, 5]);
    assert_same_samples(&rendered, &render_song(project, 6 * 8 * 183));
}

#[test]
fn the_steps_an_arpeggio_reads_past_len_stay_silent_in_the_cart() {
    let cart = exported_cart(&arpeggio_past_len(), &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx(8).note(6).volume, 0);
    assert_eq!(cart.sfx(8).note(7).volume, 0);
}

#[test]
fn a_pitch_an_arpeggio_reads_past_the_top_of_the_range_is_named() {
    let mut project = arpeggio_past_len();
    *project.phrase_mut(PhraseId::new(0)).step_mut(7) = Step::new(60, 0, 0, 0).unwrap();
    project
        .chain_mut(ChainId::new(0))
        .entry_mut(0)
        .as_mut()
        .unwrap()
        .set_transpose(10);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NoteOutOfRange {
            phrase: 0,
            step: 7,
            track: 0,
            song_row: 0,
            chain_row: 0,
        }]
    );
}

fn custom_instrument_project(sound: InstrumentSound) -> Project {
    let mut project = one_row_project();
    *project.phrase_mut(PhraseId::new(0)).step_mut(2) = Step::new(12, 0x0A, 5, 0).unwrap();
    let instrument = CustomInstrument { name: None, sound };
    project.set_custom_instrument(0x0A, Some(instrument));
    project
}

fn sfx_instrument() -> InstrumentSound {
    let mut filters = Filters::default();
    filters.set_reverb(1);
    InstrumentSound::Sfx(SfxInstrument {
        steps: vec![
            Step::new(36, 2, 5, 0).unwrap(),
            Step::new(40, 1, 4, 3).unwrap(),
        ],
        speed: 4,
        filters,
        loop_start: 0,
        loop_end: 2,
    })
}

#[test]
fn custom_instrument_8_plus_k_exports_to_sfx_k() {
    let project = custom_instrument_project(sfx_instrument());

    let cart = exported_cart(&project, &claim(&[20, 5, 2, 8], (0, 0)));

    let first_note = 36u16 | 2 << 6 | 5 << 9;
    let second_note = 40u16 | 1 << 6 | 4 << 9 | 3 << 12;
    let instrument = cart.sfx_bytes(2);
    assert_eq!(instrument[..2], first_note.to_le_bytes());
    assert_eq!(instrument[2..4], second_note.to_le_bytes());
    assert_eq!(instrument[4..64], [0; 60]);
    assert_eq!(instrument[64..], [1 + 24, 4, 0, 2]);
    assert_eq!(cart.pattern(0).channel(0), Some(5));
}

#[test]
fn steps_playing_a_custom_instrument_keep_its_digit() {
    let project = custom_instrument_project(sfx_instrument());

    let cart = exported_cart(&project, &claim(&[20, 5, 2, 8], (0, 0)));

    let note = cart.sfx(5).note(2);
    assert_eq!((note.instrument, note.custom), (2, true));
    assert!(!cart.sfx(5).note(0).custom);
}

#[test]
fn each_custom_instrument_keeps_its_slot_whatever_else_is_claimed() {
    let mut project = custom_instrument_project(sfx_instrument());
    *project.phrase_mut(PhraseId::new(0)).step_mut(3) = Step::new(12, 0x0F, 5, 0).unwrap();
    project.set_custom_instrument(0x0F, Some(CustomInstrument::default()));

    let cart = exported_cart(&project, &claim(&[0, 1, 2, 7, 8], (0, 0)));

    assert_eq!(cart.sfx(0).note(3).instrument, 7);
    assert_eq!(cart.sfx(0).note(2).instrument, 2);
    assert_eq!(cart.sfx_bytes(2)[64..], [1 + 24, 4, 0, 2]);
    assert_eq!(cart.pattern(0).channel(0), Some(0));
}

#[test]
fn a_waveform_instrument_writes_its_samples_and_the_fixed_header() {
    let mut samples = [0; 64];
    samples[0] = -1;
    samples[1] = 100;
    let project = custom_instrument_project(InstrumentSound::Waveform(WaveformInstrument {
        samples,
        bass: true,
        detune: 2,
        reverb: 0,
        dampen: 1,
    }));

    let cart = exported_cart(&project, &claim(&[2, 8], (0, 0)));

    let instrument = cart.sfx_bytes(2);
    assert_eq!(instrument[..3], [0xFF, 100, 0]);
    assert_eq!(instrument[64..], [1 + 16 + 72, 17, 0x80, 0]);
}

#[test]
fn a_custom_instrument_the_project_does_not_define_plays_an_empty_slot() {
    let mut project = custom_instrument_project(sfx_instrument());
    project.set_custom_instrument(0x0A, None);

    let cart = exported_cart(&project, &claim(&[2, 8], (0, 0)));

    let mut blank = [0; 68];
    blank[65] = 16;
    assert_eq!(cart.sfx_bytes(2), &blank);
    assert_eq!(cart.sfx(8).note(2).instrument, 2);
}

#[test]
fn a_silent_step_needs_no_slot_for_its_custom_instrument() {
    let mut project = one_row_project();
    *project.phrase_mut(PhraseId::new(0)).step_mut(2) = Step::new(12, 0x0A, 0, 0).unwrap();

    let exported = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap();

    assert_eq!(exported.sfx_count, 1);
}

#[test]
fn under_reverb_a_silent_step_takes_a_slot_for_its_custom_instrument() {
    let mut project = one_row_project();
    let phrase = project.phrase_mut(PhraseId::new(0));
    *phrase.step_mut(2) = Step::new(12, 0x0A, 0, 0).unwrap();
    phrase.filters_mut().set_reverb(1);

    let exported = export(&project, &claim(&[2, 8], (0, 0)), &blank_cart()).unwrap();

    assert_eq!(exported.sfx_count, 2);
    let note = Cart::read(&exported.cart_bytes).unwrap().sfx(8).note(2);
    assert_eq!((note.instrument, note.custom, note.volume), (2, true, 0));
}

#[test]
fn a_claim_without_sfx_k_fails_an_export_that_plays_custom_instrument_8_plus_k() {
    let project = custom_instrument_project(sfx_instrument());

    let failures = export(&project, &claim(&[0, 1, 3, 8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::MissingInstrumentSlot {
            instrument: 0x0A,
            slot: 2
        }]
    );
    assert_eq!(
        failures[0].to_string(),
        "INSTRUMENT A NEEDS SFX 02, WHICH THE CLAIM LACKS"
    );
}

#[test]
fn instruments_count_towards_the_sfx_a_claim_must_hold() {
    let project = custom_instrument_project(sfx_instrument());

    let failures = export(&project, &claim(&[2], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::TooFewSfx {
            needed: 2,
            claimed: 1
        }]
    );
}
