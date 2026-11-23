use pikutracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use pikutracker_core::player::{AudioRequest, Player, TrackMix};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::project_file;
use pikutracker_core::synth::SAMPLES_PER_TICK;
use std::path::PathBuf;

const STEP_SAMPLES: usize = 16 * SAMPLES_PER_TICK as usize;
const TRACK_HEADER_ROW: usize = 1;
const CONFIRM: Action = Action::Confirm;

fn song() -> Project {
    let mut project = Project::new();
    let id = PhraseId::new(1);
    let step = project.phrase_mut(id).step_mut(0);
    step.set_pitch(33);
    step.set_volume(5);
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(id));
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    project
}

fn choose_next_file(app: &mut App) {
    app.apply(Action::OpenProject);
    for request in app.take_file_requests() {
        if let FileRequest::List { directory } = request {
            app.receive_file(FileResponse::Listed {
                directory,
                entries: vec![DirectoryEntry {
                    name: "next.ptrk".to_owned(),
                    is_directory: false,
                }],
            });
        }
    }
    move_past_the_parent_folder_row(app);
    app.apply(CONFIRM);
}

fn move_past_the_parent_folder_row(app: &mut App) {
    app.apply(Action::Move(Direction::Down));
}

fn open(app: &mut App, project: &Project) {
    let path = PathBuf::from("./next.ptrk");
    choose_next_file(app);
    assert!(
        app.take_file_requests()
            .contains(&FileRequest::Read { path: path.clone() }),
        "the file list should ask for the chosen file"
    );
    app.receive_file(FileResponse::Read {
        path,
        contents: project_file::write(project),
    });
}

fn headers(app: &App) -> String {
    app.grid()
        .text_dump()
        .lines()
        .nth(TRACK_HEADER_ROW)
        .unwrap()[4..27]
        .trim_end()
        .to_owned()
}

#[test]
fn opening_a_project_stops_playback() {
    let mut app = App::with_project(song());
    app.apply(Action::PlaySongFromCursor);
    app.take_audio_requests();

    open(&mut app, &song());

    assert!(app.take_audio_requests().contains(&AudioRequest::Stop));
}

#[test]
fn space_plays_again_after_opening_a_project() {
    let mut app = App::with_project(song());
    app.apply(Action::PlaySongFromCursor);
    open(&mut app, &song());
    app.take_audio_requests();

    app.apply(Action::PlayFocusedPane);

    assert!(matches!(
        app.take_audio_requests().as_slice(),
        [AudioRequest::PlaySong { .. }]
    ));
}

#[test]
fn opening_a_project_clears_the_playhead_marker() {
    let mut app = App::with_project(song());
    let mut player = Player::new();
    app.apply(Action::PlaySongFromCursor);
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    player.render(STEP_SAMPLES);
    let report = player.report();
    app.show_playback(&report);

    open(&mut app, &song());
    app.show_playback(&report);

    assert!(
        app.grid()
            .text_dump()
            .lines()
            .all(|line| !line.contains('>'))
    );
}

#[test]
fn opening_a_project_resets_mute_and_solo() {
    let mut app = App::with_project(song());
    app.apply(Action::Mute { track_number: 1 });
    app.apply(Action::Solo { track_number: 2 });
    app.take_audio_requests();

    open(&mut app, &song());

    assert_eq!(headers(&app), "T1 T2 T3 T4");
    assert!(
        app.take_audio_requests()
            .contains(&AudioRequest::SetTrackMix(TrackMix::default())),
        "the Player must hear every Track again"
    );
}

#[test]
fn a_new_project_resets_mute_and_solo_too() {
    let mut app = App::with_project(Project::new());
    app.apply(Action::Mute { track_number: 1 });

    app.apply(Action::NewProject);

    assert_eq!(headers(&app), "T1 T2 T3 T4");
}

#[test]
fn a_failed_open_leaves_mute_and_solo_alone() {
    let mut app = App::with_project(song());
    app.apply(Action::Mute { track_number: 1 });
    choose_next_file(&mut app);
    app.receive_file(FileResponse::Read {
        path: PathBuf::from("./next.ptrk"),
        contents: "not a project".to_owned(),
    });

    assert_eq!(headers(&app), "M1 T2 T3 T4");
}
