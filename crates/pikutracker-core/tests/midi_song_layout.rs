mod midi_support;

use midi_support::{Song, TICKS_PER_STEP};
use pikutracker_core::midi_import::{self, Analysis, Choices, Imported};
use pikutracker_core::project::{Phrase, Project, Step};

const MIDDLE_C: u8 = 60;
const MIDDLE_C_PITCH: u8 = 24;

fn analysed(song: &Song) -> Analysis {
    midi_import::analyse(&song.bytes()).unwrap()
}

fn imported(song: &Song) -> Imported {
    let analysis = analysed(song);
    midi_import::build(&analysis, &Choices::defaults_for(&analysis))
}

fn project(song: &Song) -> Project {
    imported(song).project
}

fn phrase(project: &Project, row: usize, track: usize, k: usize) -> Phrase {
    let entry = project.chain_row_entry(row, track, k).unwrap();
    *project.phrase(entry.phrase())
}

fn pitch_on(project: &Project, track: usize) -> u8 {
    phrase(project, 0, track, 0).step(0).pitch()
}

fn five_note_chord() -> Song {
    (0..5).fold(Song::new(), |song, voice| {
        song.note(0, TICKS_PER_STEP, 1, MIDDLE_C + voice, 127)
    })
}

#[test]
fn a_chord_splits_into_stacked_voices_with_the_highest_note_first() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127)
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C + 4, 127)
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C + 7, 127);

    let project = project(&song);

    assert_eq!(pitch_on(&project, 0), MIDDLE_C_PITCH + 7);
    assert_eq!(pitch_on(&project, 1), MIDDLE_C_PITCH + 4);
    assert_eq!(pitch_on(&project, 2), MIDDLE_C_PITCH);
}

#[test]
fn voices_fill_tracks_in_file_order_keeping_a_source_together() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127)
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C + 4, 127)
        .track()
        .note(0, TICKS_PER_STEP, 2, MIDDLE_C + 12, 127);

    let project = project(&song);

    assert_eq!(pitch_on(&project, 0), MIDDLE_C_PITCH + 4);
    assert_eq!(pitch_on(&project, 1), MIDDLE_C_PITCH);
    assert_eq!(pitch_on(&project, 2), MIDDLE_C_PITCH + 12);
}

#[test]
fn two_channels_on_one_midi_track_are_two_sources() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127).note(
        0,
        TICKS_PER_STEP,
        2,
        MIDDLE_C + 4,
        127,
    );

    let analysis = analysed(&song);

    let sources: Vec<usize> = analysis.voices().iter().map(|v| v.source()).collect();
    assert_eq!(sources, [0, 1]);
}

#[test]
fn notes_that_barely_overlap_stay_on_one_voice() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP + 2, 1, MIDDLE_C, 127)
        .note(TICKS_PER_STEP, TICKS_PER_STEP, 1, MIDDLE_C + 2, 127);

    assert_eq!(analysed(&song).voices().len(), 1);
}

#[test]
fn drum_hits_that_land_together_go_to_voices_kick_then_snare_then_hats() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP, 10, 42, 127)
        .note(0, TICKS_PER_STEP, 10, 38, 127)
        .note(0, TICKS_PER_STEP, 10, 36, 127);

    let project = project(&song);

    assert_eq!(
        phrase(&project, 0, 0, 0).step(0),
        Step::new(12, 0, 7, 3).unwrap()
    );
    assert_eq!(
        phrase(&project, 0, 1, 0).step(0),
        Step::new(30, 6, 7, 5).unwrap()
    );
    assert_eq!(
        phrase(&project, 0, 2, 0).step(0),
        Step::new(55, 6, 7, 5).unwrap()
    );
}

#[test]
fn channel_10_on_two_midi_tracks_is_one_source() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP, 10, 36, 127)
        .track()
        .note(0, TICKS_PER_STEP, 10, 38, 127);

    let analysis = analysed(&song);

    let sources: Vec<usize> = analysis.voices().iter().map(|v| v.source()).collect();
    assert_eq!(sources, [0, 0]);
}

#[test]
fn drum_notes_are_never_transposed() {
    let song = Song::new().note(0, TICKS_PER_STEP, 10, 35, 127);

    let project = project(&song);

    assert_eq!(project.chain_row_entry(0, 0, 0).unwrap().transpose(), 0);
}

#[test]
fn toms_play_at_pitches_that_match_their_notes() {
    let song = Song::new().note(0, TICKS_PER_STEP, 10, 41, 127).note(
        TICKS_PER_STEP,
        TICKS_PER_STEP,
        10,
        50,
        127,
    );

    let phrase = phrase(&project(&song), 0, 0, 0);

    assert_eq!(phrase.step(0), Step::new(17, 0, 7, 3).unwrap());
    assert_eq!(phrase.step(1), Step::new(26, 0, 7, 3).unwrap());
}

#[test]
fn drum_notes_outside_the_kit_are_dropped_with_a_warning() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP, 10, 36, 127)
        .note(TICKS_PER_STEP, TICKS_PER_STEP, 10, 56, 127)
        .note(TICKS_PER_STEP * 2, TICKS_PER_STEP, 10, 39, 127);

    let warnings = imported(&song).warnings;

    assert_eq!(
        warnings[0].text,
        "2 DRUM NOTES OUTSIDE THE KIT DROPPED: 39, 56."
    );
}

#[test]
fn voices_past_the_fourth_track_are_dropped_with_a_warning() {
    let imported = imported(&five_note_chord());

    assert_eq!(
        imported.warnings[0].text,
        "5 VOICES, 4 TRACKS: DROPPED PIANO 5."
    );
    assert_eq!(pitch_on(&imported.project, 3), MIDDLE_C_PITCH + 1);
}

#[test]
fn unticking_a_voice_makes_room_for_a_dropped_one() {
    let analysis = analysed(&five_note_chord());
    let mut choices = Choices::defaults_for(&analysis);
    choices.ticked[0] = false;

    let imported = midi_import::build(&analysis, &choices);

    assert!(imported.warnings.is_empty());
    assert_eq!(pitch_on(&imported.project, 3), MIDDLE_C_PITCH);
}

#[test]
fn programs_map_to_built_in_instruments_by_general_midi_family() {
    let families = [
        ("bass", 33, 0),
        ("piano", 0, 1),
        ("chromatic percussion", 8, 1),
        ("organ", 16, 5),
        ("guitar", 24, 2),
        ("strings", 40, 2),
        ("lead", 80, 3),
        ("pad", 88, 4),
        ("ensemble", 48, 4),
        ("synth FX", 96, 7),
    ];
    for (family, program, instrument) in families {
        let song = Song::new()
            .program(0, 1, program)
            .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127);

        let step = phrase(&project(&song), 0, 0, 0).step(0);

        assert_eq!(step.instrument(), instrument, "{family} program {program}");
    }
}

#[test]
fn a_program_change_partway_through_a_voice_applies_from_that_step() {
    let song = Song::new()
        .program(0, 1, 33)
        .note(0, TICKS_PER_STEP * 4, 1, MIDDLE_C, 127)
        .program(TICKS_PER_STEP * 2, 1, 80);

    let phrase = phrase(&project(&song), 0, 0, 0);

    assert_eq!(phrase.step(1).instrument(), 0);
    assert_eq!(phrase.step(2).instrument(), 3);
    assert_eq!(phrase.step(3).instrument(), 3);
}

#[test]
fn an_edited_table_changes_what_the_notes_play() {
    let song =
        Song::new()
            .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127)
            .note(0, TICKS_PER_STEP, 10, 36, 127);
    let analysis = analysed(&song);
    let mut choices = Choices::defaults_for(&analysis);
    choices.table.change_program(0, true);
    choices.table.change_hit_note(0, true);
    choices.table.cycle_hit_effect(0);

    let project = midi_import::build(&analysis, &choices).project;

    assert_eq!(phrase(&project, 0, 0, 0).step(0).instrument(), 2);
    assert_eq!(
        phrase(&project, 0, 1, 0).step(0),
        Step::new(13, 0, 7, 4).unwrap()
    );
}

#[test]
fn when_two_notes_land_on_one_step_the_later_wins_and_import_warns() {
    let song = Song::new()
        .note(TICKS_PER_STEP * 4, 2, 1, MIDDLE_C, 127)
        .note(TICKS_PER_STEP * 4 + 4, 2, 1, MIDDLE_C + 2, 127);

    let imported = imported(&song);

    let step = phrase(&imported.project, 0, 0, 0).step(4);
    assert_eq!(step.pitch(), MIDDLE_C_PITCH + 2);
    assert_eq!(
        imported.warnings[0].text,
        "1 NOTE LANDED ON A STEP ALREADY TAKEN: THE LATER NOTE WON. FIRST: T1 PHRASE 00 STEP 04"
    );
}

#[test]
fn a_phrase_holds_the_most_whole_bars_that_fit_in_32_steps() {
    let cases = [
        (4, 2, 4, 32),
        (3, 2, 4, 24),
        (4, 2, 8, 32),
        (5, 2, 8, 32),
        (6, 3, 4, 24),
    ];
    for (numerator, power, steps_per_beat, length) in cases {
        let song = Song::new()
            .metre(0, numerator, power)
            .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127);
        let analysis = analysed(&song);
        let mut choices = Choices::defaults_for(&analysis);
        choices.steps_per_beat = steps_per_beat;

        let project = midi_import::build(&analysis, &choices).project;

        let found = phrase(&project, 0, 0, 0).length();
        assert_eq!(found, length, "{numerator}/{power} at {steps_per_beat}");
    }
}

#[test]
fn each_song_row_holds_a_chain_of_four_chain_rows() {
    let song = (0..5).fold(Song::new(), |song, phrase| {
        song.note(
            phrase * TICKS_PER_STEP * 32,
            TICKS_PER_STEP,
            1,
            MIDDLE_C + phrase as u8,
            127,
        )
    });

    let project = project(&song);

    assert!(project.chain_row_entry(0, 0, 3).is_some());
    assert!(project.chain_row_entry(0, 0, 4).is_none());
    assert_eq!(
        phrase(&project, 1, 0, 0).step(0).pitch(),
        MIDDLE_C_PITCH + 4
    );
    assert!(project.chain_row_entry(1, 0, 1).is_none());
}

#[test]
fn a_tempo_change_cuts_every_track_and_starts_a_new_chain_row() {
    let song = Song::new()
        .tempo(0, 120)
        .tempo(TICKS_PER_STEP * 8, 60)
        .note(0, TICKS_PER_STEP * 12, 1, MIDDLE_C, 127)
        .note(0, TICKS_PER_STEP * 12, 2, MIDDLE_C + 12, 127);

    let project = project(&song);

    for track in 0..2 {
        let before = phrase(&project, 0, track, 0);
        let after = phrase(&project, 0, track, 1);
        assert_eq!((before.length(), before.speed()), (8, 15), "track {track}");
        assert_eq!((after.length(), after.speed()), (32, 30), "track {track}");
    }
}

#[test]
fn a_tempo_change_that_rounds_to_the_same_speed_is_ignored() {
    let song = Song::new()
        .tempo(0, 120)
        .tempo(TICKS_PER_STEP * 8, 121)
        .note(0, TICKS_PER_STEP * 12, 1, MIDDLE_C, 127);

    let project = project(&song);

    assert_eq!(phrase(&project, 0, 0, 0).length(), 32);
    assert!(project.chain_row_entry(0, 0, 1).is_none());
}

#[test]
fn tempo_changes_closer_than_one_step_leave_the_last_one_and_warn() {
    let song = Song::new()
        .tempo(0, 120)
        .tempo(TICKS_PER_STEP * 8, 60)
        .tempo(TICKS_PER_STEP * 8 + 2, 40)
        .note(0, TICKS_PER_STEP * 12, 1, MIDDLE_C, 127);

    let imported = imported(&song);

    assert_eq!(phrase(&imported.project, 0, 0, 1).speed(), 45);
    assert_eq!(
        imported.warnings[0].text,
        "1 TEMPO CHANGE CLOSER THAN ONE STEP: THE LAST ONE WON. (ROW 00)"
    );
}

#[test]
fn a_time_signature_change_starts_a_new_chain_row() {
    let song = Song::new()
        .metre(0, 4, 2)
        .metre(TICKS_PER_STEP * 32, 3, 2)
        .note(0, TICKS_PER_STEP * 36, 1, MIDDLE_C, 127);

    let project = project(&song);

    assert_eq!(phrase(&project, 0, 0, 0).length(), 32);
    assert_eq!(phrase(&project, 0, 0, 1).length(), 24);
}

#[test]
fn a_silent_phrase_becomes_an_empty_chain_entry() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP * 64, 1, MIDDLE_C, 127)
        .note(TICKS_PER_STEP * 32, TICKS_PER_STEP, 2, MIDDLE_C, 127);

    let project = project(&song);

    assert!(project.chain_row_entry(0, 1, 0).is_none());
    assert!(project.chain_row_entry(0, 1, 1).is_some());
}

#[test]
fn a_rest_on_every_track_keeps_its_length() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127).note(
        TICKS_PER_STEP * 64,
        TICKS_PER_STEP,
        1,
        MIDDLE_C,
        127,
    );

    let project = project(&song);

    assert_eq!(project.chain_row_ticks(0, 1), 32 * 15);
    assert!(phrase(&project, 0, 0, 1).is_silent());
}

#[test]
fn identical_phrases_and_chains_are_shared() {
    let song = (0..256).fold(Song::new(), |song, step| {
        song.note(step * TICKS_PER_STEP, TICKS_PER_STEP, 1, MIDDLE_C, 127)
    });

    let project = project(&song);

    let first = project.chain_row_entry(0, 0, 0).unwrap().phrase();
    assert_eq!(project.chain_row_entry(0, 0, 3).unwrap().phrase(), first);
    assert_eq!(project.song_cell(0, 0), project.song_cell(1, 0));
}

#[test]
fn notes_below_the_range_sit_an_octave_up_with_the_offset_in_the_chain_transpose() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, 24, 127);

    let project = project(&song);

    assert_eq!(pitch_on(&project, 0), 0);
    assert_eq!(project.chain_row_entry(0, 0, 0).unwrap().transpose(), -12);
}

#[test]
fn notes_above_the_range_sit_an_octave_down_with_the_offset_in_the_chain_transpose() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, 108, 127);

    let project = project(&song);

    assert_eq!(pitch_on(&project, 0), 60);
    assert_eq!(project.chain_row_entry(0, 0, 0).unwrap().transpose(), 12);
}

#[test]
fn a_voice_that_fits_no_whole_octave_still_lands_inside_the_range() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, 31, 127).note(
        TICKS_PER_STEP,
        TICKS_PER_STEP,
        1,
        90,
        127,
    );

    let project = project(&song);

    let entry = project.chain_row_entry(0, 0, 0).unwrap();
    assert_eq!(entry.transpose(), -5);
    let phrase = project.phrase(entry.phrase());
    assert_eq!((phrase.step(0).pitch(), phrase.step(1).pitch()), (0, 59));
}

#[test]
fn a_voice_too_wide_for_a_phrase_is_folded_by_octave_with_a_warning() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, 24, 127).note(
        TICKS_PER_STEP,
        TICKS_PER_STEP,
        1,
        97,
        127,
    );

    let imported = imported(&song);

    let phrase = phrase(&imported.project, 0, 0, 0);
    assert_eq!((phrase.step(0).pitch(), phrase.step(1).pitch()), (0, 61));
    assert_eq!(
        imported.warnings[0].text,
        "PIANO VOICE SPANS 74 SEMITONES: 1 NOTE FOLDED BY AN OCTAVE. (T1)"
    );
}

#[test]
fn volume_and_expression_controllers_are_ignored_with_a_warning() {
    let song = Song::new()
        .controller(0, 1, 7, 10)
        .controller(0, 1, 11, 10)
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127);

    let imported = imported(&song);

    assert_eq!(phrase(&imported.project, 0, 0, 0).step(0).volume(), 7);
    assert_eq!(
        imported.warnings[0].text,
        "CC7 AND CC11 (VOLUME, EXPRESSION) IGNORED."
    );
}

#[test]
fn pitch_bend_and_other_controllers_share_one_warning() {
    let song = Song::new()
        .pitch_bend(0, 1)
        .controller(0, 1, 91, 10)
        .controller(0, 1, 1, 10)
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127);

    let warnings = imported(&song).warnings;

    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].text, "DROPPED: PITCH BEND, CC1, CC91.");
}

#[test]
fn the_grids_compare_speed_phrase_length_bars_and_collisions() {
    let song = Song::new()
        .note(0, 4, 1, MIDDLE_C, 127)
        .note(20, 4, 1, MIDDLE_C + 2, 127);
    let analysis = analysed(&song);

    let options = midi_import::compare_grids(&analysis, &Choices::defaults_for(&analysis));

    let rows: Vec<_> = options
        .iter()
        .map(|o| {
            (
                o.steps_per_beat,
                o.speed,
                o.phrase_length,
                o.bars,
                o.collisions,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (2, 30, 32, 4, 1),
            (3, 20, 24, 2, 0),
            (4, 15, 32, 2, 0),
            (6, 10, 24, 1, 0),
            (8, 8, 32, 1, 0),
        ]
    );
}
