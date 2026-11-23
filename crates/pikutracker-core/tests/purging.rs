mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::player::AudioRequest;
use pikutracker_core::project::{Chain, ChainEntry, ChainId, Phrase, PhraseId, Project, Step};
use support::{apply, status_line};

const RIGHT: Action = Action::Move(Direction::Right);

fn put_entry(project: &mut Project, chain: u8, index: usize, phrase: u8) {
    *project.chain_mut(ChainId::new(chain)).entry_mut(index) =
        Some(ChainEntry::new(PhraseId::new(phrase)));
}

fn put_note(project: &mut Project, phrase: u8) {
    *project.phrase_mut(PhraseId::new(phrase)).step_mut(0) = Step::new(24, 0, 5, 0).unwrap();
}

fn chain_is_empty(app: &App, chain: u8) -> bool {
    *app.project().chain(ChainId::new(chain)) == Chain::default()
}

fn phrase_is_empty(app: &App, phrase: u8) -> bool {
    *app.project().phrase(PhraseId::new(phrase)) == Phrase::default()
}

fn with_unused_items() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    put_entry(&mut project, 0, 0, 0);
    put_note(&mut project, 0);
    put_entry(&mut project, 5, 0, 6);
    put_note(&mut project, 6);
    put_note(&mut project, 9);
    project
}

#[test]
fn purge_asks_first_and_says_how_many_will_go() {
    let mut app = App::with_project(with_unused_items());
    let before = app.project().clone();

    app.apply(Action::PurgeUnused);

    assert!(app.project() == &before, "nothing goes before the answer");
    let status = status_line(&app);
    assert!(status.contains("PURGE 1 CHAIN AND 2 PHRASES"), "{status}");
    assert!(status.contains("ENTER"), "{status}");
    assert!(status.contains("ESC"), "{status}");
}

#[test]
fn the_confirmation_says_when_loose_phrases_will_go() {
    let mut app = App::with_project(with_unused_items());

    app.apply(Action::PurgeUnused);

    assert!(status_line(&app).contains("1 LOOSE"));
}

#[test]
fn enter_removes_unreferenced_chains_and_phrases() {
    let mut app = App::with_project(with_unused_items());

    apply(&mut app, &[Action::PurgeUnused, Action::Confirm]);

    assert!(chain_is_empty(&app, 5));
    assert!(phrase_is_empty(&app, 6), "only the purged Chain played it");
    assert!(phrase_is_empty(&app, 9), "a loose Phrase goes too");
}

#[test]
fn enter_keeps_everything_the_song_uses() {
    let project = with_unused_items();
    let kept_chain = *project.chain(ChainId::new(0));
    let kept_phrase = *project.phrase(PhraseId::new(0));
    let mut app = App::with_project(project);

    apply(&mut app, &[Action::PurgeUnused, Action::Confirm]);

    assert!(*app.project().chain(ChainId::new(0)) == kept_chain);
    assert!(*app.project().phrase(PhraseId::new(0)) == kept_phrase);
}

#[test]
fn a_phrase_a_kept_chain_also_plays_is_kept() {
    let mut project = with_unused_items();
    put_entry(&mut project, 0, 1, 6);
    let mut app = App::with_project(project);

    app.apply(Action::PurgeUnused);
    assert!(status_line(&app).contains("PURGE 1 CHAIN AND 1 PHRASE "));

    app.apply(Action::Confirm);
    assert!(chain_is_empty(&app, 5));
    assert!(!phrase_is_empty(&app, 6));
}

#[test]
fn esc_keeps_everything() {
    let mut app = App::with_project(with_unused_items());
    let before = app.project().clone();

    apply(&mut app, &[Action::PurgeUnused, Action::Cancel]);

    assert!(app.project() == &before);
    assert!(!status_line(&app).contains("PURGE"));
}

#[test]
fn esc_answers_the_question_even_with_a_selection_active() {
    let mut app = App::with_project(with_unused_items());
    let before = app.project().clone();

    apply(
        &mut app,
        &[
            Action::Select(Direction::Right),
            Action::PurgeUnused,
            Action::Cancel,
        ],
    );

    assert!(!status_line(&app).contains("PURGE"));
    app.apply(Action::Confirm);
    assert!(app.project() == &before);
}

#[test]
fn other_keys_wait_for_an_answer() {
    let mut app = App::with_project(with_unused_items());
    let before = app.project().clone();

    apply(&mut app, &[Action::PurgeUnused, Action::Clone, RIGHT]);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("PURGE"));
}

#[test]
fn empty_slots_are_not_counted_and_nothing_to_purge_asks_nothing() {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    put_entry(&mut project, 0, 0, 0);
    let mut app = App::with_project(project);

    app.apply(Action::PurgeUnused);
    assert!(status_line(&app).contains("NOTHING TO PURGE"));

    let before = app.project().clone();
    app.apply(Action::Confirm);
    assert!(app.project() == &before);
}

#[test]
fn a_purge_is_one_undo_step() {
    let mut app = App::with_project(with_unused_items());
    let before = app.project().clone();

    apply(&mut app, &[Action::PurgeUnused, Action::Confirm]);
    let after = app.project().clone();
    assert!(after != before);

    app.apply(Action::Undo);
    assert!(app.project() == &before);
    app.apply(Action::Redo);
    assert!(app.project() == &after);
}

#[test]
fn a_purge_says_what_went() {
    let mut app = App::with_project(with_unused_items());

    apply(&mut app, &[Action::PurgeUnused, Action::Confirm]);

    assert!(status_line(&app).contains("PURGED 1 CHAIN AND 2 PHRASES"));
}

#[test]
fn the_player_hears_a_purge_made_during_playback() {
    let mut app = App::with_project(with_unused_items());
    app.apply(Action::PlayFocusedPane);
    app.take_audio_requests();

    apply(&mut app, &[Action::PurgeUnused, Action::Confirm]);

    let requests = app.take_audio_requests();
    let heard = requests.iter().any(|request| {
        matches!(request, AudioRequest::UpdateProject(project) if **project == *app.project())
    });
    assert!(heard);
}
