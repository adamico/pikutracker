mod midi_support;

use midi_support::{Song, TICKS_PER_BEAT, TICKS_PER_STEP};
use pikutracker_core::midi_import::{self, Analysis, Choices, Imported};
use pikutracker_core::project::{Phrase, Project, Step};

const TILTED_SAW: u8 = 1;
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

fn note(pitch: u8, instrument: u8, volume: u8) -> Step {
    Step::new(pitch, instrument, volume, 0).unwrap()
}

#[test]
fn a_note_becomes_a_step_at_the_matching_pitch_and_volume() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127);

    let phrase = phrase(&project(&song), 0, 0, 0);

    assert_eq!(phrase.step(0), note(MIDDLE_C_PITCH, TILTED_SAW, 7));
    assert!(phrase.step(1).is_silent());
}

#[test]
fn velocity_becomes_a_volume_rounded_up() {
    let volumes = [(1, 1), (18, 1), (19, 2), (64, 4), (126, 7), (127, 7)];
    for (velocity, volume) in volumes {
        let song = Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, velocity);

        let phrase = phrase(&project(&song), 0, 0, 0);

        assert_eq!(phrase.step(0).volume(), volume, "velocity {velocity}");
    }
}

#[test]
fn a_held_note_repeats_and_a_gap_is_silent() {
    let song = Song::new()
        .note(0, TICKS_PER_STEP * 3, 1, MIDDLE_C, 127)
        .note(TICKS_PER_STEP * 5, TICKS_PER_STEP, 1, MIDDLE_C + 2, 127);

    let phrase = phrase(&project(&song), 0, 0, 0);

    let held = note(MIDDLE_C_PITCH, TILTED_SAW, 7);
    assert_eq!(
        [phrase.step(0), phrase.step(1), phrase.step(2)],
        [held, held, held]
    );
    assert!(phrase.step(3).is_silent() && phrase.step(4).is_silent());
    assert_eq!(phrase.step(5).pitch(), MIDDLE_C_PITCH + 2);
}

#[test]
fn note_starts_snap_to_the_nearest_step_and_lengths_last_at_least_one_step() {
    let song = Song::new()
        .note(TICKS_PER_STEP + TICKS_PER_STEP / 2, 1, 1, MIDDLE_C, 127)
        .note(
            TICKS_PER_STEP * 4 + TICKS_PER_STEP / 2 - 1,
            1,
            1,
            MIDDLE_C + 2,
            127,
        );

    let phrase = phrase(&project(&song), 0, 0, 0);

    assert!(phrase.step(1).is_silent());
    assert_eq!(phrase.step(2).pitch(), MIDDLE_C_PITCH);
    assert!(phrase.step(3).is_silent());
    assert_eq!(phrase.step(4).pitch(), MIDDLE_C_PITCH + 2);
}

#[test]
fn the_sustain_pedal_holds_a_released_note_until_it_lifts() {
    let song = Song::new()
        .controller(0, 1, 64, 127)
        .note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127)
        .controller(TICKS_PER_STEP * 3, 1, 64, 0);

    let phrase = phrase(&project(&song), 0, 0, 0);

    assert!(!phrase.step(2).is_silent());
    assert!(phrase.step(3).is_silent());
}

#[test]
fn the_speed_comes_from_the_tempo_and_the_steps_per_beat() {
    let song = Song::new()
        .tempo(0, 100)
        .note(0, TICKS_PER_BEAT, 1, MIDDLE_C, 127);
    let analysis = analysed(&song);
    let mut choices = Choices::defaults_for(&analysis);

    assert_eq!(
        phrase(&midi_import::build(&analysis, &choices).project, 0, 0, 0).speed(),
        18
    );

    choices.steps_per_beat = 8;
    assert_eq!(
        phrase(&midi_import::build(&analysis, &choices).project, 0, 0, 0).speed(),
        9
    );
}

#[test]
fn a_file_with_no_tempo_plays_at_120_bpm() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127);

    assert_eq!(phrase(&project(&song), 0, 0, 0).speed(), 15);
}

#[test]
fn the_project_has_only_built_in_instruments_no_phrase_loop_no_claim_and_no_target_cart() {
    let song = Song::new().note(0, TICKS_PER_STEP, 1, MIDDLE_C, 127);

    let project = project(&song);

    assert_eq!(project.custom_instruments().count(), 0);
    assert_eq!(project.target_cart(), None);
    let phrase = project.chain_row_entry(0, 0, 0).unwrap().phrase();
    assert!(!project.phrase(phrase).loops());
}

#[test]
fn bytes_that_are_not_a_midi_file_are_refused() {
    assert!(midi_import::analyse(b"not a midi file").is_err());
}
