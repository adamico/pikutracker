mod support;

use pikutracker_core::app::layout;
use pikutracker_core::app::{Action, App, Direction, LogItem, Report, Severity};
use pikutracker_core::project::SongPlace;
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use pikutracker_core::screen::palette::Colour;
use std::time::Duration;
use support::{pane_text, status_line};

const STATUS_ROW: usize = 44;
const DOWN: Action = Action::Move(Direction::Down);

fn status_background(app: &App) -> Colour {
    app.grid().cell(0, STATUS_ROW).style.bg
}

fn right_slot(app: &App, line: usize) -> String {
    pane_text(app, layout::RIGHT_SLOT, line)
}

fn right_slot_lines(app: &App) -> Vec<String> {
    (0..STATUS_ROW).map(|line| right_slot(app, line)).collect()
}

fn song_line(app: &App, line: usize) -> String {
    pane_text(app, layout::SONG, line)
}

fn item(text: &str) -> LogItem {
    LogItem::new(Severity::Warning, text)
}

#[test]
fn info_shows_in_green_and_fades_after_about_three_seconds() {
    let mut app = App::new();
    app.report(Report::info("SAVED DEMO.PTRK"));

    assert_eq!(status_line(&app), "SAVED DEMO.PTRK");
    assert_eq!(status_background(&app), Colour::Green);

    app.advance_time(Duration::from_millis(2500));
    assert_eq!(status_line(&app), "SAVED DEMO.PTRK");
    assert_eq!(status_background(&app), Colour::DarkGreen);

    app.advance_time(Duration::from_millis(600));
    assert!(!status_line(&app).contains("SAVED"));
}

#[test]
fn the_app_says_when_the_status_line_next_changes() {
    let mut app = App::new();
    assert_eq!(app.time_until_status_change(), None);

    app.report(Report::info("SAVED DEMO.PTRK"));
    assert_eq!(
        app.time_until_status_change(),
        Some(Duration::from_millis(2000))
    );

    assert!(!app.advance_time(Duration::from_millis(500)));
    assert!(app.advance_time(Duration::from_millis(1600)));
    assert_eq!(
        app.time_until_status_change(),
        Some(Duration::from_millis(900))
    );
}

#[test]
fn a_failure_stays_in_red_until_esc() {
    let mut app = App::new();
    app.report(Report::failure("EXPORT FAILED: NO CLAIM"));

    app.advance_time(Duration::from_secs(60));
    app.apply(DOWN);
    assert_eq!(status_line(&app), "! EXPORT FAILED: NO CLAIM");
    assert_eq!(status_background(&app), Colour::Red);

    app.apply(Action::Cancel);
    assert!(!status_line(&app).contains("EXPORT FAILED"));
}

#[test]
fn a_warning_stays_in_orange_until_esc() {
    let mut app = App::new();
    app.report(Report::warning("IMPORTED WITH 6 WARNINGS"));

    app.advance_time(Duration::from_secs(60));
    assert_eq!(status_line(&app), "! IMPORTED WITH 6 WARNINGS");
    assert_eq!(status_background(&app), Colour::Orange);

    app.apply(Action::Cancel);
    assert!(!status_line(&app).contains("IMPORTED"));
}

#[test]
fn a_result_with_items_points_to_the_log() {
    let mut app = App::new();
    app.report(Report::failure("EXPORT FAILED: 3 PROBLEMS").with_items(vec![item("ONE")]));

    assert_eq!(status_line(&app), "! EXPORT FAILED: 3 PROBLEMS  ^L LOG");
}

#[test]
fn a_result_with_items_opens_the_log_without_taking_focus() {
    let mut app = App::new();
    app.report(Report::warning("IMPORTED").with_items(vec![item("ONE")]));

    assert_eq!(right_slot(&app, 0), "LOG");
    assert_eq!(song_line(&app, 2), "00 [--]-- -- --");
}

#[test]
fn a_result_without_items_leaves_the_right_hand_view_alone() {
    let mut app = App::new();
    app.report(Report::failure("CANNOT SAVE"));

    assert!(right_slot(&app, 0).starts_with("MULTITRACK"));
}

#[test]
fn ctrl_l_opens_the_log_with_focus_and_closes_it_back_to_the_pane_you_came_from() {
    let mut app = App::new();
    app.apply(Action::FocusPrevious);

    app.apply(Action::ToggleLog);
    assert_eq!(right_slot(&app, 0), "LOG");
    assert_eq!(pane_text(&app, layout::CHAIN, 2), "00 --     ---");

    app.apply(Action::ToggleLog);
    assert!(right_slot(&app, 0).starts_with("MULTITRACK"));
    assert_eq!(pane_text(&app, layout::CHAIN, 2), "00[--]    ---");
}

#[test]
fn esc_in_the_log_closes_it() {
    let mut app = App::new();
    app.apply(Action::ToggleLog);

    app.apply(Action::Cancel);

    assert!(right_slot(&app, 0).starts_with("MULTITRACK"));
    assert_eq!(song_line(&app, 2), "00 [--]-- -- --");
}

#[test]
fn esc_clears_a_failure_before_it_closes_the_log() {
    let mut app = App::new();
    app.report(Report::failure("EXPORT FAILED").with_items(vec![item("ONE")]));
    app.apply(Action::FocusNext);

    app.apply(Action::Cancel);
    assert_eq!(right_slot(&app, 0), "LOG");
    assert!(!status_line(&app).contains("EXPORT FAILED"));

    app.apply(Action::Cancel);
    assert!(right_slot(&app, 0).starts_with("MULTITRACK"));
}

#[test]
fn the_log_joins_the_tab_cycle_as_the_visible_right_hand_view() {
    let mut app = App::new();
    app.report(Report::warning("IMPORTED").with_items(vec![item("ONE"), item("TWO")]));
    assert_eq!(right_slot(&app, 1), " WRN ONE");

    for _ in 0..3 {
        app.apply(Action::FocusPrevious);
    }
    assert_eq!(right_slot(&app, 1), ">WRN ONE");

    app.apply(DOWN);
    assert_eq!(right_slot(&app, 1), " WRN ONE");
    assert_eq!(right_slot(&app, 2), ">WRN TWO");

    app.apply(Action::FocusPrevious);
    assert_eq!(right_slot(&app, 0), "LOG");
    assert_eq!(song_line(&app, 2), "00 [--]-- -- --");
}

#[test]
fn each_item_shows_a_tag_wrapped_text_and_an_action_hint() {
    let mut app = App::new();
    app.report(Report::failure("EXPORT FAILED").with_items(vec![
            LogItem::new(
                Severity::Failure,
                "NEEDS 11 PATTERNS, THE CLAIM HAS 8. WIDEN THE CLAIM",
            )
            .with_hint("^T EDIT CLAIM"),
            LogItem::new(Severity::Info, "CLAIM: 14 SFX"),
        ]));

    assert_eq!(
        right_slot_lines(&app)[..6],
        [
            "LOG",
            " ERR NEEDS 11 PATTERNS,",
            "     THE CLAIM HAS 8.",
            "     WIDEN THE CLAIM",
            "     ^T EDIT CLAIM",
            " OK  CLAIM: 14 SFX",
        ]
    );
}

#[test]
fn the_log_keeps_a_short_history_with_the_newest_first() {
    let mut app = App::new();
    for number in 0..40 {
        app.report(Report::info(format!("RESULT {number}")));
    }
    app.apply(Action::ToggleLog);

    let lines = right_slot_lines(&app);
    assert_eq!(lines[1], ">OK  RESULT 39");
    assert_eq!(lines[32], " OK  RESULT 8");
    assert_eq!(lines[33], "");
}

#[test]
fn the_log_scrolls_to_keep_the_picked_item_visible() {
    let mut app = App::new();
    let items = (0..32).map(|number| {
        item(&format!(
            "ITEM {number} HAS TEXT LONG ENOUGH TO TAKE TWO LINES"
        ))
    });
    app.report(Report::warning("IMPORTED").with_items(items.collect()));
    app.apply(Action::FocusNext);

    for _ in 0..31 {
        app.apply(DOWN);
    }

    let dump = right_slot_lines(&app).join("\n");
    assert!(dump.contains(">WRN ITEM 31"), "{dump}");
}

fn project_with_a_located_phrase() -> Project {
    let mut project = Project::new();
    project.set_song_cell(1, 2, Some(ChainId::new(4)));
    *project.chain_mut(ChainId::new(4)).entry_mut(2) = Some(ChainEntry::new(PhraseId::new(7)));
    project
}

#[test]
fn enter_on_an_item_jumps_to_its_location_and_the_panes_stay_usable() {
    let mut app = App::with_project(project_with_a_located_phrase());
    let location = SongPlace {
        song_row: 1,
        track: 2,
        chain_row: 2,
        step: 5,
    };
    app.report(Report::failure("EXPORT FAILED").with_items(vec![
            LogItem::new(Severity::Failure, "T3 ROW 01 STEP 05: ABOVE D#5")
                .with_hint("ENTER: GO THERE")
                .at(location),
        ]));
    app.apply(Action::FocusNext);

    app.apply(Action::Confirm);

    assert_eq!(app.shown_chain_id(), Some(ChainId::new(4)));
    assert_eq!(app.shown_phrase_id(), Some(PhraseId::new(7)));
    assert!(app.grid().text_dump().contains("05[...]"));
    assert_eq!(right_slot(&app, 0), "LOG");

    app.apply(Action::Move(Direction::Down));
    assert!(app.grid().text_dump().contains("06[...]"));
}

#[test]
fn enter_on_an_item_with_no_location_stays_in_the_log() {
    let mut app = App::new();
    app.report(Report::warning("IMPORTED").with_items(vec![item("ONE"), item("TWO")]));
    app.apply(Action::FocusNext);

    app.apply(Action::Confirm);

    assert_eq!(right_slot(&app, 1), ">WRN ONE");
}
