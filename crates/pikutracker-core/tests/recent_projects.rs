mod support;

use pikutracker_core::app::layout;
use pikutracker_core::app::{
    Action, App, Direction, DirectoryEntry, FileRequest, FileResponse, Launch, SettingsContents,
};
use pikutracker_core::project::Project;
use pikutracker_core::project_file;
use std::path::PathBuf;
use support::{pane_text, state_ron};

const DOWN: Action = Action::Move(Direction::Down);
const HOME: &str = "/home/user";
const STATE_FILE: &str = "/state/state.ron";

fn right_slot(app: &App, line: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, line)
}

fn launch_remembering(paths: &[&str]) -> App {
    let mut app = App::new();
    app.launch(Launch {
        config_file: None,
        config: SettingsContents::Missing,
        state_file: Some(PathBuf::from(STATE_FILE)),
        state: SettingsContents::Bytes(state_ron(paths).into_bytes()),
        directory: PathBuf::from(HOME),
    });
    app.take_file_requests();
    if let Some(first) = paths.first() {
        app.receive_file(FileResponse::Failed {
            path: PathBuf::from(first),
            reason: "gone".to_owned(),
        });
    }
    app
}

fn open_list(app: &mut App, entries: Vec<DirectoryEntry>) {
    app.apply(Action::OpenProject);
    let requests = app.take_file_requests();
    let [FileRequest::List { directory }] = &requests[..] else {
        panic!("expected one listing, got {requests:?}");
    };
    app.receive_file(FileResponse::Listed {
        directory: directory.clone(),
        entries,
    });
}

fn folder(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: true,
    }
}

fn file(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: false,
    }
}

#[test]
fn recent_projects_are_listed_above_the_folder_with_the_cursor_on_the_newest() {
    let mut app = launch_remembering(&["/music/b.ptrk", "/old/a.ptrk"]);

    open_list(&mut app, vec![folder("songs"), file("c.ptrk")]);

    assert_eq!(right_slot(&app, 0), "OPEN PROJECT");
    assert_eq!(right_slot(&app, 1), "RECENT");
    assert_eq!(right_slot(&app, 2), ">b.ptrk");
    assert_eq!(right_slot(&app, 3), " a.ptrk");
    assert_eq!(right_slot(&app, 4), HOME);
    assert_eq!(right_slot(&app, 5), " ../");
    assert_eq!(right_slot(&app, 6), " songs/");
    assert_eq!(right_slot(&app, 7), " c.ptrk");
}

#[test]
fn enter_on_a_recent_project_opens_it() {
    let mut app = launch_remembering(&["/music/b.ptrk", "/old/a.ptrk"]);
    open_list(&mut app, vec![]);

    app.apply(DOWN);
    app.apply(Action::Confirm);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::Read {
            path: PathBuf::from("/old/a.ptrk")
        }]
    );
}

#[test]
fn the_cursor_moves_from_the_recent_projects_into_the_folder() {
    let mut app = launch_remembering(&["/music/b.ptrk"]);
    open_list(&mut app, vec![folder("songs")]);

    app.apply(DOWN);
    app.apply(DOWN);
    app.apply(Action::Confirm);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(HOME).join("songs")
        }]
    );
}

#[test]
fn after_moving_to_another_folder_the_cursor_is_on_its_first_row() {
    let mut app = launch_remembering(&["/music/b.ptrk"]);
    open_list(&mut app, vec![folder("songs")]);
    app.apply(DOWN);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();

    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME).join("songs"),
        entries: vec![],
    });

    assert_eq!(right_slot(&app, 2), " b.ptrk");
    assert_eq!(right_slot(&app, 4), ">../");
}

#[test]
fn the_open_project_is_not_listed_as_recent() {
    let mut app = App::new();
    app.launch(Launch {
        config_file: None,
        config: SettingsContents::Missing,
        state_file: Some(PathBuf::from(STATE_FILE)),
        state: SettingsContents::Bytes(state_ron(&["/music/b.ptrk", "/old/a.ptrk"]).into_bytes()),
        directory: PathBuf::from(HOME),
    });
    app.take_file_requests();
    app.receive_file(FileResponse::Read {
        path: PathBuf::from("/music/b.ptrk"),
        contents: project_file::write(&Project::new()),
    });
    app.take_file_requests();

    open_list(&mut app, vec![]);

    assert_eq!(right_slot(&app, 1), "RECENT");
    assert_eq!(right_slot(&app, 2), ">a.ptrk");
    assert_eq!(right_slot(&app, 3), "/music");
}

#[test]
fn save_as_and_import_list_no_recent_projects() {
    let mut app = launch_remembering(&["/music/b.ptrk"]);

    app.apply(Action::SaveAs);
    assert_ne!(right_slot(&app, 1), "RECENT");
    app.apply(Action::Cancel);
    app.take_file_requests();

    app.apply(Action::Import);
    assert_ne!(right_slot(&app, 1), "RECENT");
}

#[test]
fn with_no_recent_projects_the_list_starts_with_the_folder() {
    let mut app = launch_remembering(&[]);

    open_list(&mut app, vec![]);

    assert_eq!(right_slot(&app, 1), HOME);
    assert_eq!(right_slot(&app, 2), ">../");
}

fn recent_projects_written(app: &mut App) -> String {
    let requests = app.take_file_requests();
    let written = requests.iter().find_map(|request| match request {
        FileRequest::Write { path, contents } if path == std::path::Path::new(STATE_FILE) => {
            Some(contents.clone())
        }
        _ => None,
    });
    let text = written.unwrap_or_else(|| panic!("no state.ron written in {requests:?}"));
    let listed = text
        .split_once("recent_projects:")
        .map_or("", |(_, rest)| rest);
    listed
        .split('"')
        .skip(1)
        .step_by(2)
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn reopening_a_recent_project_moves_it_to_the_front_without_a_duplicate() {
    let mut app = launch_remembering(&["/music/a.ptrk", "/music/b.ptrk"]);
    open_list(&mut app, vec![]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();

    app.receive_file(FileResponse::Read {
        path: PathBuf::from("/music/b.ptrk"),
        contents: project_file::write(&Project::new()),
    });

    assert_eq!(
        recent_projects_written(&mut app),
        "/music/b.ptrk /music/a.ptrk"
    );
}

#[test]
fn only_the_ten_most_recent_projects_are_kept() {
    let remembered: Vec<String> = (0..10).map(|n| format!("/music/{n}.ptrk")).collect();
    let remembered: Vec<&str> = remembered.iter().map(String::as_str).collect();
    let mut app = launch_remembering(&remembered);
    app.apply(Action::HexDigit(1));
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME),
        entries: vec![],
    });
    app.apply(Action::TypeChar('s'));
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Written {
        path: PathBuf::from(HOME).join("s.ptrk"),
    });

    assert_eq!(
        recent_projects_written(&mut app),
        "/home/user/s.ptrk /music/0.ptrk /music/1.ptrk /music/2.ptrk /music/3.ptrk \
         /music/4.ptrk /music/5.ptrk /music/6.ptrk /music/7.ptrk /music/8.ptrk"
    );
}
