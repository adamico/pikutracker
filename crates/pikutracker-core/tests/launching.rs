mod support;

use pikutracker_core::app::{
    Action, App, Direction, DirectoryEntry, FileRequest, FileResponse, Launch, SettingsContents,
};
use pikutracker_core::project::Project;
use pikutracker_core::project_file;
use pikutracker_core::settings::{SettingsRead, read_state};
use std::path::PathBuf;
use support::{let_the_notice_fade, state_ron, status_line};

const STATE_FILE: &str = "/state/pikutracker/state.ron";
const HOME: &str = "/home/user";

fn launch(state: Option<String>) -> App {
    launch_with(state.map_or(SettingsContents::Missing, |text| {
        SettingsContents::Bytes(text.into_bytes())
    }))
}

fn launch_with(state: SettingsContents) -> App {
    let mut app = App::new();
    app.launch(Launch {
        config_file: None,
        config: SettingsContents::Missing,
        state_file: Some(PathBuf::from(STATE_FILE)),
        state,
        directory: PathBuf::from(HOME),
    });
    app
}

fn save_a_new_project(app: &mut App, folder: &str) -> Vec<FileRequest> {
    app.apply(Action::HexDigit(1));
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(folder),
        entries: vec![],
    });
    app.apply(Action::TypeChar('s'));
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Written {
        path: PathBuf::from(folder).join("s.ptrk"),
    });
    app.take_file_requests()
}

fn state_written(requests: &[FileRequest]) -> Option<Vec<PathBuf>> {
    requests.iter().find_map(|request| match request {
        FileRequest::Write { path, contents }
            if path.as_path() == std::path::Path::new(STATE_FILE) =>
        {
            match read_state(contents) {
                SettingsRead::Loaded(state) => Some(state.recent_projects().to_vec()),
                other => panic!("the app wrote an unreadable state: {other:?}"),
            }
        }
        _ => None,
    })
}

fn project_text() -> String {
    project_file::write(&Project::new())
}

#[test]
fn with_no_state_file_the_app_starts_empty_in_the_launch_folder() {
    let mut app = launch(None);

    assert!(app.take_file_requests().is_empty());
    app.apply(Action::OpenProject);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(HOME)
        }]
    );
}

#[test]
fn the_last_opened_project_is_opened_at_launch() {
    let mut app = launch(Some(state_ron(&["/music/b.ptrk", "/music/a.ptrk"])));

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::Read {
            path: PathBuf::from("/music/b.ptrk")
        }]
    );
    app.receive_file(FileResponse::Read {
        path: PathBuf::from("/music/b.ptrk"),
        contents: project_text(),
    });

    let_the_notice_fade(&mut app);
    assert!(
        status_line(&app).ends_with("B.PTRK"),
        "{}",
        status_line(&app)
    );
    app.take_file_requests();
    app.apply(Action::Save);
    assert_eq!(
        app.take_file_requests()[0],
        FileRequest::Write {
            path: PathBuf::from("/music/b.ptrk"),
            contents: project_text(),
        }
    );
}

#[test]
fn a_last_project_that_is_gone_leaves_an_empty_project_and_says_why() {
    let mut app = launch(Some(state_ron(&["/music/gone.ptrk"])));
    app.take_file_requests();

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("/music/gone.ptrk"),
        reason: "No such file or directory".to_owned(),
    });

    assert!(status_line(&app).starts_with("! CANNOT OPEN GONE.PTRK: NO SUCH FILE OR DIRECTORY"));
    assert!(!app.has_unsaved_changes());
}

#[test]
fn opening_a_project_remembers_it_first() {
    let mut app = launch(Some(state_ron(&["/music/a.ptrk"])));
    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("/music/a.ptrk"),
        reason: "busy".to_owned(),
    });
    app.take_file_requests();

    app.apply(Action::OpenProject);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME),
        entries: vec![DirectoryEntry {
            name: "new.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    for _ in 0..2 {
        app.apply(Action::Move(pikutracker_core::app::Direction::Down));
    }
    app.apply(Action::Confirm);
    let requests = app.take_file_requests();
    let opened = PathBuf::from(HOME).join("new.ptrk");
    assert_eq!(
        requests,
        [FileRequest::Read {
            path: opened.clone()
        }]
    );
    app.receive_file(FileResponse::Read {
        path: opened.clone(),
        contents: project_text(),
    });

    assert_eq!(
        state_written(&app.take_file_requests()),
        Some(vec![opened, PathBuf::from("/music/a.ptrk")])
    );
}

#[test]
fn saving_a_project_remembers_it_first() {
    let mut app = launch(None);
    app.apply(Action::HexDigit(1));
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME),
        entries: vec![],
    });
    for ch in "song".chars() {
        app.apply(Action::TypeChar(ch));
    }
    app.apply(Action::Confirm);
    let saved = PathBuf::from(HOME).join("song.ptrk");
    assert!(matches!(
        &app.take_file_requests()[..],
        [FileRequest::Write { path, .. }] if *path == saved
    ));

    app.receive_file(FileResponse::Written {
        path: saved.clone(),
    });

    assert_eq!(state_written(&app.take_file_requests()), Some(vec![saved]));
}

#[test]
fn a_damaged_state_file_is_renamed_and_the_app_says_so() {
    for damaged in [
        "(version: 1, recent_projects: [",
        "not ron at all",
        "(recent_projects: [])",
    ] {
        let mut app = launch(Some(damaged.to_owned()));

        assert_eq!(
            app.take_file_requests(),
            [FileRequest::Rename {
                from: PathBuf::from(STATE_FILE),
                to: PathBuf::from(format!("{STATE_FILE}.bad")),
            }],
            "{damaged}"
        );
        assert!(
            status_line(&app)
                .starts_with("! STATE.RON WAS DAMAGED: RENAMED TO STATE.RON.BAD, USING DEFAULTS"),
            "{damaged}"
        );
    }
}

#[test]
fn a_state_file_with_no_recent_projects_field_starts_with_none() {
    let mut app = launch(Some("(version: 1)".to_owned()));

    assert!(app.take_file_requests().is_empty());
    assert!(!status_line(&app).starts_with('!'));
    assert_eq!(
        state_written(&save_a_new_project(&mut app, HOME)),
        Some(vec![PathBuf::from(HOME).join("s.ptrk")])
    );
}

#[test]
fn the_state_file_written_names_its_format_version_and_reopens_the_project_at_next_launch() {
    let mut app = launch(None);
    let requests = save_a_new_project(&mut app, HOME);
    let [FileRequest::Write { contents, .. }] = &requests[..] else {
        panic!("expected one state write, got {requests:?}");
    };
    assert!(contents.contains("version: 1"), "{contents}");

    let mut next = launch(Some(contents.clone()));

    assert_eq!(
        next.take_file_requests(),
        [FileRequest::Read {
            path: PathBuf::from(HOME).join("s.ptrk")
        }]
    );
}

#[test]
fn a_newer_state_file_is_left_alone_for_the_whole_session() {
    let mut app = launch(Some("(version: 9)".to_owned()));

    assert!(app.take_file_requests().is_empty());
    assert!(
        status_line(&app)
            .starts_with("! STATE.RON IS FROM A NEWER PIKUTRACKER: USING DEFAULTS THIS SESSION")
    );

    assert_eq!(state_written(&save_a_new_project(&mut app, HOME)), None);
}

#[test]
fn an_app_that_was_not_launched_with_a_state_file_never_writes_one() {
    let mut app = App::new();

    assert!(save_a_new_project(&mut app, ".").is_empty());
}

#[test]
fn a_state_file_that_is_not_text_is_damaged_and_renamed() {
    let mut app = launch_with(SettingsContents::Bytes(vec![0xFF, 0xFE, b'(']));

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::Rename {
            from: PathBuf::from(STATE_FILE),
            to: PathBuf::from(format!("{STATE_FILE}.bad")),
        }]
    );
    assert!(status_line(&app).starts_with("! STATE.RON WAS DAMAGED"));
}

#[test]
fn a_state_file_that_cannot_be_read_is_never_overwritten() {
    let mut app = launch_with(SettingsContents::Unreadable {
        reason: "Permission denied".to_owned(),
    });

    assert!(app.take_file_requests().is_empty());
    assert!(
        status_line(&app)
            .starts_with("! CANNOT READ STATE.RON: PERMISSION DENIED. USING DEFAULTS THIS SESSION")
    );
    assert_eq!(state_written(&save_a_new_project(&mut app, HOME)), None);
}

#[test]
fn a_damaged_state_file_that_cannot_be_renamed_is_never_overwritten() {
    let mut app = launch(Some("(version: 1, recent_projects: [".to_owned()));
    app.take_file_requests();

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from(STATE_FILE),
        reason: "Read-only file system".to_owned(),
    });

    assert!(
        status_line(&app)
            .starts_with("! CANNOT RENAME STATE.RON TO STATE.RON.BAD: READ-ONLY FILE SYSTEM")
    );
    assert_eq!(state_written(&save_a_new_project(&mut app, HOME)), None);
}

#[test]
fn a_renamed_damaged_state_file_is_replaced_by_the_next_save() {
    let mut app = launch(Some("(version: 1, recent_projects: [".to_owned()));
    app.take_file_requests();

    app.receive_file(FileResponse::Renamed {
        path: PathBuf::from(format!("{STATE_FILE}.bad")),
    });

    assert_eq!(
        state_written(&save_a_new_project(&mut app, HOME)),
        Some(vec![PathBuf::from(HOME).join("s.ptrk")])
    );
}

#[test]
fn with_no_place_for_a_state_file_the_app_still_starts_in_the_launch_folder() {
    let mut app = App::new();
    app.launch(Launch {
        config_file: None,
        config: SettingsContents::Missing,
        state_file: None,
        state: SettingsContents::Missing,
        directory: PathBuf::from(HOME),
    });

    app.apply(Action::OpenProject);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(HOME)
        }]
    );
    assert!(save_a_new_project(&mut app, HOME).is_empty());
}

#[test]
fn the_help_notice_follows_the_reopened_project() {
    let mut app = launch(Some(state_ron(&["/music/b.ptrk"])));
    app.take_file_requests();

    app.receive_file(FileResponse::Read {
        path: PathBuf::from("/music/b.ptrk"),
        contents: project_text(),
    });

    assert!(
        status_line(&app).starts_with("^H HELP"),
        "{}",
        status_line(&app)
    );
}

#[test]
fn the_help_notice_is_not_announced_once_the_user_has_acted() {
    let mut app = launch_with(SettingsContents::Unreadable {
        reason: "permission denied".to_owned(),
    });
    assert!(status_line(&app).starts_with("! CANNOT READ STATE.RON"));

    app.apply(Action::OpenProject);
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME),
        entries: vec![DirectoryEntry {
            name: "b.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    app.apply(Action::Move(Direction::Down));
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Read {
        path: PathBuf::from(HOME).join("b.ptrk"),
        contents: project_text(),
    });

    assert!(
        status_line(&app).starts_with("OPENED B.PTRK"),
        "{}",
        status_line(&app)
    );
}
