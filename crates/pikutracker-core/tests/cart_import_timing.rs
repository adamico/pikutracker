mod support;

use pikutracker_core::cart::Cart;
use pikutracker_core::cart_import::import;
use pikutracker_core::project::{ChainEntry, Phrase, Project};
use support::{
    TestCart, TestPattern, TestSfx, assert_same_samples, render_notes, render_song, sound_of,
};

fn imported(cart: &TestCart) -> (Project, Vec<String>) {
    let imported = import(&Cart::read(&cart.p8()).unwrap(), "demo.p8");
    let warnings = imported.warnings.into_iter().map(|w| w.text).collect();
    (imported.project, warnings)
}

fn one_pattern(sfx: Vec<TestSfx>) -> TestCart {
    let mut channels = [None; 4];
    for (channel, slot) in channels.iter_mut().zip(0..sfx.len() as u8) {
        *channel = Some(slot);
    }
    TestCart {
        sfx,
        patterns: vec![TestPattern::playing(channels)],
    }
}

fn counting(speed: u8) -> TestSfx {
    let pitches: Vec<u8> = (10..42).collect();
    TestSfx::with_pitches(speed, &pitches)
}

fn with_len(mut sfx: TestSfx, len: u8) -> TestSfx {
    sfx.loop_start = len;
    sfx.loop_end = 0;
    sfx
}

fn looping(mut sfx: TestSfx, start: u8, end: u8) -> TestSfx {
    sfx.loop_start = start;
    sfx.loop_end = end;
    sfx
}

fn entry(project: &Project, track: usize, chain_row: usize) -> ChainEntry {
    project.chain_row_entry(0, track, chain_row).unwrap()
}

fn phrase(project: &Project, track: usize, chain_row: usize) -> &Phrase {
    project.phrase(entry(project, track, chain_row).phrase())
}

fn pitches(phrase: &Phrase) -> Vec<Option<u8>> {
    (0..phrase.length())
        .map(|index| phrase.step(index))
        .map(|step| (!step.is_silent()).then_some(step.pitch()))
        .collect()
}

fn audible(range: std::ops::Range<u8>) -> Vec<Option<u8>> {
    range.map(Some).collect()
}

#[test]
fn the_left_most_non_looping_channel_sets_the_length_and_cuts_off_the_others() {
    let cart = one_pattern(vec![with_len(counting(8), 8), counting(8), counting(4)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(project.chain_row_ticks(0, 0), 64);
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..18));
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..18));
    assert_eq!(pitches(phrase(&project, 2, 0)), audible(10..26));
}

#[test]
fn a_pattern_ending_partway_through_a_step_drops_that_step_with_a_warning() {
    let cart = one_pattern(vec![with_len(counting(8), 5), counting(3)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..23));
    assert_eq!(
        warnings,
        ["PATTERN 00 ENDS INSIDE A STEP: T2 PHRASE 01 LOST ITS LAST STEP."]
    );
}

#[test]
fn a_channel_too_slow_for_one_whole_step_becomes_an_empty_entry_with_a_warning() {
    let cart = one_pattern(vec![with_len(counting(1), 1), counting(8)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(project.chain_row_entry(0, 1, 0), None);
    assert_eq!(warnings, ["PATTERN 00 ENDS INSIDE A STEP: T2 LOST A STEP."]);
}

#[test]
fn a_loop_over_the_whole_sfx_becomes_a_phrase_loop_and_keeps_the_notes_after_it() {
    let cart = one_pattern(vec![counting(8), looping(counting(8), 0, 4)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(phrase(&project, 1, 0).phrase_loop(), Some(0..4));
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..14));
    assert_eq!(phrase(&project, 1, 0).step(4).pitch(), 14);
    assert_eq!(project.chain_row_ticks(0, 0), 256);
}

#[test]
fn a_lead_in_loop_becomes_a_phrase_loop_after_its_lead_in() {
    let cart = one_pattern(vec![with_len(counting(8), 10), looping(counting(8), 2, 4)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(phrase(&project, 1, 0).phrase_loop(), Some(2..4));
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..14));
    assert_eq!(project.chain_row_ticks(0, 0), 80);
}

#[test]
fn a_lead_in_loop_that_plays_past_32_steps_keeps_its_lead_in_without_a_warning() {
    let cart = one_pattern(vec![counting(16), looping(counting(8), 2, 6)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(phrase(&project, 1, 0).phrase_loop(), Some(2..6));
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..16));
}

#[test]
fn len_on_a_channel_that_does_not_set_the_length_does_not_stop_it() {
    let cart = one_pattern(vec![counting(8), with_len(counting(8), 4)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..42));
}

#[test]
fn when_every_channel_loops_the_slowest_is_unrolled_to_32_steps_without_a_phrase_loop() {
    let cart = one_pattern(vec![looping(counting(4), 0, 8), looping(counting(8), 0, 8)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert!(phrase(&project, 0, 0).loops());
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..18));
    assert!(!phrase(&project, 1, 0).loops());
    let unrolled: Vec<Option<u8>> = (0..32).map(|step| Some(10 + step % 8)).collect();
    assert_eq!(pitches(phrase(&project, 1, 0)), unrolled);
}

#[test]
fn when_every_channel_loops_a_slowest_loop_over_all_32_notes_keeps_its_phrase_loop() {
    let cart = one_pattern(vec![
        looping(counting(4), 0, 8),
        looping(counting(8), 0, 32),
    ]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert_eq!(phrase(&project, 1, 0).phrase_loop(), Some(0..32));
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..42));
}

#[test]
fn a_loop_start_at_or_past_a_non_zero_loop_end_plays_all_32_notes_and_sets_the_length() {
    let cart = one_pattern(vec![looping(counting(8), 8, 4), counting(4)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..42));
}

#[test]
fn spd_0_becomes_speed_1_with_one_warning_per_sfx() {
    let cart = TestCart {
        sfx: vec![counting(0)],
        patterns: vec![TestPattern::playing([Some(0), Some(0), None, None])],
    };

    let (project, warnings) = imported(&cart);

    assert_eq!(phrase(&project, 0, 0).speed(), 1);
    assert_eq!(project.chain_row_ticks(0, 0), 32);
    assert_eq!(
        warnings,
        ["SFX 00 HAS SPD 0: SET TO SPEED 1, TIMING FOLLOWS PICO-8."]
    );
}

const BIT_7_WARNING: &str =
    "SFX 00 HAS BIT 7 SET: IMPORTED AS A PLAIN PHRASE THAT NEVER SETS THE PATTERN LENGTH.";

#[test]
fn a_bit_7_sfx_is_an_ordinary_phrase_that_never_sets_the_length() {
    let cart = one_pattern(vec![
        looping(counting(8), 0x88, 0x10),
        with_len(counting(16), 8),
    ]);

    let (project, warnings) = imported(&cart);

    assert_eq!(project.chain_row_ticks(0, 0), 128);
    assert!(!phrase(&project, 0, 0).loops());
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..26));
    assert_eq!(warnings, [BIT_7_WARNING]);
}

#[test]
fn a_len_of_128_or_more_counts_as_bit_7() {
    let cart = one_pattern(vec![with_len(counting(8), 200), with_len(counting(8), 4)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(project.chain_row_ticks(0, 0), 32);
    assert_eq!(warnings, [BIT_7_WARNING]);
}

#[test]
fn a_bit_7_sfx_stops_sounding_after_its_32_notes() {
    let cart = one_pattern(vec![looping(counting(4), 0x80, 0x08), counting(8)]);

    let (project, _) = imported(&cart);

    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..42));
}

#[test]
fn beside_channels_that_all_loop_the_length_follows_the_looping_channels_alone() {
    let cart = one_pattern(vec![
        looping(counting(16), 0x88, 0x90),
        looping(counting(8), 0, 8),
    ]);

    let (project, warnings) = imported(&cart);

    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..26));
    assert!(!phrase(&project, 1, 0).loops());
    assert_eq!(phrase(&project, 1, 0).length(), 32);
    assert_eq!(warnings, [BIT_7_WARNING]);
}

#[test]
fn a_pattern_holding_only_bit_7_sfx_stays_as_a_32_step_chain_row_with_a_warning() {
    let cart = one_pattern(vec![
        looping(counting(8), 0x88, 0x10),
        with_len(counting(4), 0xff),
    ]);

    let (project, warnings) = imported(&cart);

    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..42));
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..42));
    assert_eq!(
        warnings,
        [
            "PATTERN 00 HOLDS ONLY BIT 7 SFX: THE CART STALLS HERE, THE EXPORTED SONG PLAYS \
             THROUGH.",
            BIT_7_WARNING,
            "SFX 01 HAS BIT 7 SET: IMPORTED AS A PLAIN PHRASE THAT NEVER SETS THE PATTERN LENGTH.",
        ]
    );
}

#[test]
fn len_above_32_splits_the_pattern_into_chain_rows_of_one_song_row() {
    let cart = one_pattern(vec![
        with_len(counting(8), 40),
        counting(8),
        looping(counting(8), 0, 4),
    ]);

    let (project, warnings) = imported(&cart);

    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert_eq!(project.chain_row_ticks(0, 1), 64);
    assert_eq!(project.chain_row_ticks(0, 2), 0);
    assert!(project.song_row_is_empty(1));
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..42));
    assert_eq!(pitches(phrase(&project, 0, 1)), [None; 8]);
    assert_eq!(pitches(phrase(&project, 1, 0)), audible(10..42));
    assert_eq!(project.chain_row_entry(0, 1, 1), None);
    assert_eq!(phrase(&project, 2, 0).phrase_loop(), Some(0..4));
    assert_eq!(phrase(&project, 2, 1).phrase_loop(), Some(0..4));
    assert_eq!(pitches(phrase(&project, 2, 1)), audible(10..14));
    assert_eq!(
        warnings,
        [
            "SONG NEEDS 2 PATTERNS AND 4 SFX BUT THE CLAIM HOLDS 1 AND 3: WIDEN THE CLAIM BEFORE \
          EXPORT."
        ]
    );
}

#[test]
fn len_64_lasts_exactly_two_chain_rows() {
    let cart = one_pattern(vec![with_len(counting(4), 64)]);

    let (project, _) = imported(&cart);

    assert_eq!(project.chain_row_ticks(0, 0), 128);
    assert_eq!(project.chain_row_ticks(0, 1), 128);
    assert_eq!(project.chain_row_ticks(0, 2), 0);
}

#[test]
fn a_step_straddling_a_split_is_dropped_with_a_warning() {
    let cart = one_pattern(vec![with_len(counting(3), 40), counting(5)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(phrase(&project, 1, 0).length(), 19);
    assert_eq!(pitches(phrase(&project, 1, 1)), audible(30..34));
    assert_eq!(
        warnings[0],
        "PATTERN 00 SPLITS INSIDE A STEP: T2 PHRASE 02 LOST ITS LAST STEP."
    );
}

#[test]
fn a_loop_across_a_split_starts_the_next_chain_row_where_the_cart_is() {
    let cart = one_pattern(vec![with_len(counting(3), 40), looping(counting(5), 1, 4)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(phrase(&project, 1, 0).phrase_loop(), Some(1..4));
    assert_eq!(phrase(&project, 1, 1).phrase_loop(), Some(0..3));
    assert_eq!(pitches(phrase(&project, 1, 1)), [12, 13, 11].map(Some));
    assert!(!warnings.iter().any(|w| w.contains("LOST")), "{warnings:?}");
}

#[test]
fn steps_that_a_split_moves_early_are_reported() {
    let cart = one_pattern(vec![with_len(counting(3), 40), counting(5)]);

    let (_, warnings) = imported(&cart);

    assert!(
        warnings.iter().any(
            |w| w == "PATTERN 00 SPLITS INSIDE A STEP: T2 PLAYS ITS LATER STEPS 4 TICKS EARLY."
        ),
        "{warnings:?}"
    );
}

#[test]
fn a_split_on_a_step_boundary_moves_nothing() {
    let cart = one_pattern(vec![with_len(counting(4), 40), counting(8)]);

    let (_, warnings) = imported(&cart);

    assert!(
        !warnings.iter().any(|w| w.contains("EARLY")),
        "{warnings:?}"
    );
}

#[test]
fn a_song_that_still_fits_its_claim_gets_no_claim_warning() {
    let cart = one_pattern(vec![counting(8), counting(4)]);

    let (_, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
}

#[test]
fn a_song_with_more_phrases_than_a_project_holds_warns_and_still_imports() {
    let mut sfx: Vec<TestSfx> = (0..60)
        .map(|slot: u8| {
            let notes: Vec<u8> = (0..7).map(|note| (slot + note) % 64).collect();
            looping(TestSfx::with_pitches(1, &notes), 0, 7)
        })
        .collect();
    sfx.extend([1, 2, 3, 5].map(|speed| with_len(counting(speed), 97)));
    let patterns = (0..64)
        .map(|at: u8| {
            let loop_at = |offset: u8| Some((at + offset) % 60);
            TestPattern::playing([Some(60 + at % 4), loop_at(0), loop_at(21), loop_at(42)])
        })
        .collect();

    let (project, warnings) = imported(&TestCart { sfx, patterns });

    assert!(!project.song_row_is_empty(63));
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "SONG NEEDS MORE THAN 256 PHRASES: SOME ENTRIES STAY EMPTY."),
        "{warnings:?}"
    );
}

fn with_fast_arpeggio(mut sfx: TestSfx, notes: std::ops::Range<usize>) -> TestSfx {
    for note in &mut sfx.notes[notes] {
        note.effect = 6;
    }
    sfx
}

#[test]
fn an_arpeggio_reading_notes_past_len_imports_with_the_same_sound() {
    let mut sfx = with_len(counting(8), 6);
    sfx = with_fast_arpeggio(sfx, 4..6);
    sfx.notes[6].pitch = 40;
    sfx.notes[7].pitch = 44;
    let cart = one_pattern(vec![sfx]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    let original = Cart::read(&cart.p8()).unwrap();
    let expected = render_notes(sound_of(&original, 0), &[0, 1, 2, 3, 4, 5]);
    assert_same_samples(&render_song(project, expected.len()), &expected);
}

#[test]
fn a_copied_loop_keeps_its_arpeggio_groups() {
    let looped = with_fast_arpeggio(looping(counting(8), 2, 5), 2..5);
    let cart = one_pattern(vec![with_len(counting(8), 10), looped]);

    let (_, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
}

#[test]
fn an_arpeggio_in_the_lead_in_of_an_unrolled_loop_keeps_its_sound() {
    let sfx = with_fast_arpeggio(looping(counting(8), 4, 8), 0..4);
    let cart = one_pattern(vec![sfx]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    let played: Vec<usize> = (0..32)
        .map(|step| if step < 8 { step } else { 4 + (step - 4) % 4 })
        .collect();
    let original = Cart::read(&cart.p8()).unwrap();
    let expected = render_notes(sound_of(&original, 0), &played);
    assert_same_samples(&render_song(project, expected.len()), &expected);
}

#[test]
fn a_loop_past_the_last_note_does_not_set_the_length_and_stays_silent_after_note_31() {
    let cart = one_pattern(vec![looping(counting(1), 32, 33), counting(8)]);

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(project.chain_row_ticks(0, 0), 256);
    assert!(!phrase(&project, 0, 0).loops());
    assert_eq!(pitches(phrase(&project, 0, 0)), audible(10..42));
}
