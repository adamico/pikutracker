use super::focus::Focus;
use super::panes::chain::ChainCursorPlace;
use super::panes::phrase::PhraseCursorPlace;
use super::panes::song::SongCursorPlace;
use super::shown::Shown;
use crate::project::{Project, ProjectChange};
use std::collections::VecDeque;

const MAX_UNDO_ENTRIES: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CursorPlace {
    pub(super) focus: Focus,
    pub(super) song: SongCursorPlace,
    pub(super) chain: ChainCursorPlace,
    pub(super) phrase: PhraseCursorPlace,
    pub(super) shown: Shown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Merge {
    Nudge,
    Digit,
    CartVisit(u32),
}

#[derive(Debug)]
struct UndoEntry {
    change: ProjectChange,
    place: CursorPlace,
}

#[derive(Debug)]
struct MergingRun {
    merge: Merge,
    before_run: Project,
    place: CursorPlace,
    left_at: CursorPlace,
    has_entry: bool,
}

#[derive(Debug, Default)]
pub(super) struct History {
    undo: VecDeque<UndoEntry>,
    redo: Vec<UndoEntry>,
    run: Option<MergingRun>,
    before_action: Option<(Project, CursorPlace)>,
}

impl History {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn begin(&mut self, project: &Project, place: CursorPlace) {
        self.before_action = Some((project.clone(), place));
    }

    pub(super) fn finish(&mut self, project: &Project, left_at: CursorPlace, merge: Option<Merge>) {
        let Some((before, place)) = self.before_action.take() else {
            return;
        };
        let run = self
            .run
            .take()
            .filter(|run| Some(run.merge) == merge && run.left_at == place);
        match run {
            Some(run) => self.continue_run(run, project, left_at),
            None => self.start_run(before, (place, left_at), project, merge),
        }
    }

    fn continue_run(&mut self, mut run: MergingRun, project: &Project, left_at: CursorPlace) {
        run.left_at = left_at;
        if run.has_entry {
            self.undo.pop_back();
        }
        let change = ProjectChange::between(&run.before_run, project);
        run.has_entry = !change.is_empty();
        if run.has_entry {
            self.push(change, run.place);
        }
        self.run = Some(run);
    }

    fn start_run(
        &mut self,
        before: Project,
        (place, left_at): (CursorPlace, CursorPlace),
        project: &Project,
        merge: Option<Merge>,
    ) {
        let change = ProjectChange::between(&before, project);
        let has_entry = !change.is_empty();
        if has_entry {
            self.push(change, place);
        }
        self.run = merge.map(|merge| MergingRun {
            merge,
            before_run: before,
            place,
            left_at,
            has_entry,
        });
    }

    fn push(&mut self, change: ProjectChange, place: CursorPlace) {
        self.redo.clear();
        self.undo.push_back(UndoEntry { change, place });
        if self.undo.len() > MAX_UNDO_ENTRIES {
            self.undo.pop_front();
        }
    }

    pub(super) fn undo(&mut self, project: &mut Project) -> Option<CursorPlace> {
        self.before_action = None;
        self.run = None;
        let entry = self.undo.pop_back()?;
        entry.change.undo(project);
        let place = entry.place;
        self.redo.push(entry);
        Some(place)
    }

    pub(super) fn redo(&mut self, project: &mut Project) -> Option<CursorPlace> {
        self.before_action = None;
        self.run = None;
        let entry = self.redo.pop()?;
        entry.change.redo(project);
        let place = entry.place;
        self.undo.push_back(entry);
        Some(place)
    }
}
