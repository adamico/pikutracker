use pikutracker_core::app::{Action, App};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, Step};

const FIRST_ENTRY_LINE: usize = 2;
const FIRST_STEP_LINE: usize = 5;

fn phrase(project: &mut Project, id: u8, length: i32, speed: i32) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_length(length);
    phrase.set_speed(speed);
    id
}

fn place(project: &mut Project, row: usize, track: usize, chain: u8, entries: &[ChainEntry]) {
    let id = ChainId::new(chain);
    for (index, entry) in entries.iter().enumerate() {
        *project.chain_mut(id).entry_mut(index) = Some(*entry);
    }
    project.set_song_cell(row, track, Some(id));
}

fn looping(project: &mut Project, id: u8, start: i32, end: i32, speed: i32) -> PhraseId {
    let id = PhraseId::new(id);
    let phrase = project.phrase_mut(id);
    phrase.set_loop(start, end);
    phrase.set_speed(speed);
    id
}

fn transposed(phrase: PhraseId, semitones: i32) -> ChainEntry {
    let mut entry = ChainEntry::new(phrase);
    entry.set_transpose(semitones);
    entry
}

fn pane_text(app: &App, line: usize, columns: std::ops::Range<usize>) -> String {
    let text: String = app
        .grid()
        .text_dump()
        .lines()
        .nth(line)
        .unwrap()
        .chars()
        .chain(std::iter::repeat(' '))
        .skip(columns.start)
        .take(columns.len())
        .collect();
    text.trim_end().to_owned()
}

fn chain_text(app: &App, entry: usize) -> String {
    pane_text(app, FIRST_ENTRY_LINE + entry, 29..62)
}

fn step_text(app: &App, step: usize) -> String {
    pane_text(app, FIRST_STEP_LINE + step, 63..82)
}

#[test]
fn each_k_line_shows_the_ticks_of_its_chain_row() {
    let mut project = Project::new();
    let short = phrase(&mut project, 5, 4, 8);
    let long = phrase(&mut project, 6, 8, 8);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(short)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(long)]);

    let app = App::with_project(project);

    assert_eq!(chain_text(&app, 0), "0 05 +00          64");
    assert_eq!(chain_text(&app, 1), "1 -- ---           -");
}

#[test]
fn the_ticks_count_every_track_whichever_chain_is_on_show() {
    let mut project = Project::new();
    let short = phrase(&mut project, 5, 4, 8);
    let long = phrase(&mut project, 6, 8, 8);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(short)]);
    place(&mut project, 0, 7, 1, &[ChainEntry::new(long)]);

    let app = App::with_project(project);

    assert_eq!(chain_text(&app, 0), "0 05 +00          64");
}

#[test]
fn the_ticks_follow_the_song_row_under_the_cursor() {
    let mut project = Project::new();
    let id = phrase(&mut project, 5, 4, 8);
    place(&mut project, 1, 1, 0, &[ChainEntry::new(id)]);

    let mut app = App::with_project(project);
    assert_eq!(chain_text(&app, 0), "0 -- ---           -");

    app.apply(Action::Move(pikutracker_core::app::Direction::Down));
    assert_eq!(chain_text(&app, 0), "0 -- ---          32");
}

#[test]
fn the_chain_row_table_is_gone() {
    let app = App::with_project(Project::new());

    assert!(!app.grid().text_dump().contains("CHAIN ROWS"));
}

#[test]
fn a_chain_row_lasts_as_long_as_its_longest_phrase_without_a_phrase_loop() {
    let mut project = Project::new();
    let long_loop = looping(&mut project, 1, 0, 32, 16);
    let short = phrase(&mut project, 2, 4, 8);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(long_loop)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(short)]);

    assert_eq!(project.chain_row_ticks(0, 0), 32);
}

#[test]
fn an_all_looping_chain_row_lasts_as_long_as_its_longest_phrase_played_once_through() {
    let mut project = Project::new();
    let shorter = looping(&mut project, 1, 0, 4, 8);
    let longer = looping(&mut project, 2, 6, 8, 8);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(shorter)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(longer)]);

    assert_eq!(project.chain_row_ticks(0, 0), 64);
}

#[test]
fn a_looping_phrase_with_a_lead_in_never_sets_the_length() {
    let mut project = Project::new();
    let lead_in_and_loop = looping(&mut project, 1, 4, 32, 8);
    let short = phrase(&mut project, 2, 4, 8);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(lead_in_and_loop)]);
    place(&mut project, 0, 1, 1, &[ChainEntry::new(short)]);

    assert_eq!(project.chain_row_ticks(0, 0), 32);
}

#[test]
fn a_16_16_chain_beside_a_32_chain_lasts_48_steps() {
    let mut project = Project::new();
    let half = phrase(&mut project, 1, 16, 1);
    let whole = phrase(&mut project, 2, 32, 1);
    place(
        &mut project,
        0,
        0,
        0,
        &[ChainEntry::new(half), ChainEntry::new(half)],
    );
    place(&mut project, 0, 1, 1, &[ChainEntry::new(whole)]);

    let ticks: u32 = (0..16).map(|k| project.chain_row_ticks(0, k)).sum();

    assert_eq!(ticks, 48);
}

#[test]
fn a_chain_row_with_no_entries_lasts_no_time() {
    let mut project = Project::new();
    let id = phrase(&mut project, 1, 4, 8);
    place(&mut project, 0, 0, 0, &[ChainEntry::new(id)]);

    assert_eq!(project.chain_row_ticks(0, 1), 0);
    assert_eq!(project.chain_row_ticks(1, 0), 0);
}

fn transposed_app(semitones: i32, volume: i32) -> App {
    let mut project = Project::new();
    let id = phrase(&mut project, 1, 4, 8);
    let step = project.phrase_mut(id).step_mut(0);
    step.set_pitch(24);
    step.set_volume(volume);
    place(&mut project, 0, 0, 0, &[transposed(id, semitones)]);
    App::with_project(project)
}

#[test]
fn a_step_transposed_above_63_is_flagged_in_the_phrase_and_chain_panes() {
    let app = transposed_app(40, 5);

    assert_eq!(step_text(&app, 0), "00 C-2 0 5 0  !");
    assert_eq!(chain_text(&app, 0), "0 01 +28      !   32");
}

#[test]
fn a_step_transposed_below_0_is_flagged() {
    let app = transposed_app(-25, 5);

    assert_eq!(step_text(&app, 0), "00 C-2 0 5 0  !");
    assert_eq!(chain_text(&app, 0), "0 01 -19      !   32");
}

#[test]
fn the_edges_of_the_pitch_range_are_not_flagged() {
    assert_eq!(step_text(&transposed_app(39, 5), 0), "00 C-2 0 5 0");
    assert_eq!(step_text(&transposed_app(-24, 5), 0), "00 C-2 0 5 0");
    assert_eq!(
        chain_text(&transposed_app(39, 5), 0),
        "0 01 +27          32"
    );
}

#[test]
fn a_silent_step_is_not_flagged() {
    let app = transposed_app(40, 0);

    assert_eq!(step_text(&app, 0), "00 ... . . .");
    assert_eq!(chain_text(&app, 0), "0 01 +28          32");
}

#[test]
fn a_step_past_len_is_not_flagged() {
    let mut project = Project::new();
    let id = phrase(&mut project, 1, 4, 8);
    let step = project.phrase_mut(id).step_mut(6);
    step.set_pitch(60);
    step.set_volume(5);
    place(&mut project, 0, 0, 0, &[transposed(id, 12)]);

    let app = App::with_project(project);

    assert_eq!(chain_text(&app, 0), "0 01 +0C          32");
}

fn arpeggio_reading_a_silent_d_sharp_5(length: i32) -> App {
    let mut project = Project::new();
    let id = phrase(&mut project, 1, length, 8);
    *project.phrase_mut(id).step_mut(0) = Step::new(24, 0, 5, 6).unwrap();
    *project.phrase_mut(id).step_mut(1) = Step::new(63, 0, 0, 0).unwrap();
    place(&mut project, 0, 0, 0, &[transposed(id, 1)]);
    App::with_project(project)
}

#[test]
fn a_silent_step_an_arpeggio_reads_is_flagged_like_export_rejects_it() {
    let app = arpeggio_reading_a_silent_d_sharp_5(4);

    assert_eq!(step_text(&app, 0), "00 C-2 0 5 6");
    assert_eq!(step_text(&app, 1), "01 ... . . .  !");
    assert_eq!(chain_text(&app, 0), "0 01 +01      !   32");
}

#[test]
fn a_step_past_len_an_arpeggio_reads_flags_the_chain_entry() {
    let app = arpeggio_reading_a_silent_d_sharp_5(1);

    assert_eq!(chain_text(&app, 0), "0 01 +01      !    8");
}
