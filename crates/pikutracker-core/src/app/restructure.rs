use super::edit_target::{EditTarget, NO_CHAIN_HERE, NO_PHRASE_HERE};
use super::files::Question;
use super::focus::EditPane;
use super::messages::Report;
use super::overlays::Overlay;
use super::selection::CellRange;
use super::{App, CHAIN_ENTRIES, Chain, Phrase, Project};
use crate::project::{ChainEntry, ChainId, PhraseId, UnusedItems};
use crate::wording::counted;
use std::collections::HashMap;

const NO_FREE_CHAIN: &str = "NO UNUSED CHAIN LEFT";
const PURGE: &str = "PURGE";
const NO_FREE_PHRASE: &str = "NO UNUSED PHRASE LEFT";
const NEEDS_A_REFERENCE: &str = "USE THE SONG OR CHAIN PANE";
const NOTHING_TO_PURGE: &str = "NOTHING TO PURGE";

fn counted_items(items: &UnusedItems) -> String {
    format!(
        "{} AND {}",
        counted(items.chains.len(), "CHAIN"),
        counted(items.phrases.len(), "PHRASE"),
    )
}

impl App {
    pub(super) fn clone_items(&mut self, deep: bool) {
        let Some(cells) = self.target_cells() else {
            return;
        };
        match self.focus.edit_pane() {
            Some(EditPane::Song) => self.clone_chains(cells, deep),
            Some(EditPane::Chain) => self.clone_phrases(cells),
            Some(EditPane::Phrase) => self.remark = Some(NEEDS_A_REFERENCE.into()),
            None => {}
        }
    }

    fn clone_chains(&mut self, cells: CellRange, deep: bool) {
        let chains = self.chains_in(cells);
        if chains.is_empty() {
            self.remark = Some(NO_CHAIN_HERE.into());
            return;
        }
        let phrases = if deep {
            self.phrases_of(&chains)
        } else {
            Vec::new()
        };
        let Some(new_chains) = self.free_chain_slots(chains.len()) else {
            return;
        };
        let Some(new_phrases) = self.free_phrase_slots(phrases.len()) else {
            return;
        };
        let phrase_copies = self.copy_phrases(&phrases, &new_phrases);
        let mut chain_copies = HashMap::new();
        for (chain, copy) in chains.into_iter().zip(new_chains) {
            let mut copied = *self.project.chain(chain);
            for index in 0..CHAIN_ENTRIES {
                if let Some(entry) = copied.entry_mut(index) {
                    let phrase = entry.phrase();
                    entry.set_phrase(*phrase_copies.get(&phrase).unwrap_or(&phrase));
                }
            }
            *self.project.chain_mut(copy) = copied;
            chain_copies.insert(chain, copy);
        }
        for row in cells.rows() {
            for track in cells.columns() {
                if let Some(copy) = self
                    .project
                    .song_cell(row, track)
                    .and_then(|chain| chain_copies.get(&chain))
                {
                    self.project.set_song_cell(row, track, Some(*copy));
                }
            }
        }
        self.selection_start = None;
    }

    fn clone_phrases(&mut self, cells: CellRange) {
        let Some(EditTarget::Chain(chain)) = self.edit_target_or_remark() else {
            return;
        };
        let mut phrases = Vec::new();
        for row in cells.rows() {
            if let Some(entry) = self.project.chain(chain).entry(row)
                && !phrases.contains(&entry.phrase())
            {
                phrases.push(entry.phrase());
            }
        }
        if phrases.is_empty() {
            self.remark = Some(NO_PHRASE_HERE.into());
            return;
        }
        let Some(new_phrases) = self.free_phrase_slots(phrases.len()) else {
            return;
        };
        let copies = self.copy_phrases(&phrases, &new_phrases);
        for row in cells.rows() {
            if let Some(entry) = self.project.chain_mut(chain).entry_mut(row) {
                entry.set_phrase(copies[&entry.phrase()]);
            }
        }
        self.selection_start = None;
    }

    pub(super) fn chains_in(&self, cells: CellRange) -> Vec<ChainId> {
        let mut chains = Vec::new();
        for row in cells.rows() {
            for track in cells.columns() {
                if let Some(chain) = self.project.song_cell(row, track)
                    && !chains.contains(&chain)
                {
                    chains.push(chain);
                }
            }
        }
        chains
    }

    pub(super) fn phrases_of(&self, chains: &[ChainId]) -> Vec<PhraseId> {
        let mut phrases = Vec::new();
        for chain in chains {
            for phrase in self.project.chain(*chain).phrases() {
                if !phrases.contains(&phrase) {
                    phrases.push(phrase);
                }
            }
        }
        phrases
    }

    fn copy_phrases(
        &mut self,
        phrases: &[PhraseId],
        new_phrases: &[PhraseId],
    ) -> HashMap<PhraseId, PhraseId> {
        let mut copies = HashMap::new();
        for (phrase, copy) in phrases.iter().zip(new_phrases) {
            *self.project.phrase_mut(*copy) = *self.project.phrase(*phrase);
            copies.insert(*phrase, *copy);
        }
        copies
    }

    fn free_chain_slots(&mut self, count: usize) -> Option<Vec<ChainId>> {
        let free = self.project.free_chains();
        if free.len() < count {
            self.report(Report::failure(NO_FREE_CHAIN));
            return None;
        }
        Some(free[..count].to_vec())
    }

    fn free_phrase_slots(&mut self, count: usize) -> Option<Vec<PhraseId>> {
        let free = self.project.free_phrases();
        if free.len() < count {
            self.report(Report::failure(NO_FREE_PHRASE));
            return None;
        }
        Some(free[..count].to_vec())
    }

    pub(super) fn insert_row(&mut self) {
        self.shift_rows(
            Project::insert_song_row,
            Chain::insert_entry,
            Phrase::insert_step,
        );
    }

    pub(super) fn delete_row(&mut self) {
        self.shift_rows(
            Project::delete_song_row,
            Chain::delete_entry,
            Phrase::delete_step,
        );
    }

    fn shift_rows(
        &mut self,
        shift_song: fn(&mut Project, usize),
        shift_chain: fn(&mut Chain, usize),
        shift_phrase: fn(&mut Phrase, usize),
    ) {
        match self.edit_target_or_remark() {
            Some(EditTarget::Song) => shift_song(&mut self.project, self.song.row()),
            Some(EditTarget::Chain(chain)) => {
                shift_chain(self.project.chain_mut(chain), self.chain.entry());
            }
            Some(EditTarget::Phrase(phrase)) => {
                let Some(step) = self.phrase_step_or_remark() else {
                    return;
                };
                shift_phrase(self.project.phrase_mut(phrase), step.row);
            }
            None => return,
        }
        self.selection_start = None;
    }

    pub(super) fn ask_to_purge(&mut self) {
        let unused = self.project.unused_items();
        if unused.is_empty() {
            self.remark = Some(NOTHING_TO_PURGE.into());
            return;
        }
        let loose = match unused.loose_phrases {
            0 => String::new(),
            count => format!(" ({count} LOOSE)"),
        };
        self.selection_start = None;
        self.overlays.open(Overlay::Question(Question::purge_unused(
            format!("PURGE {}{loose}", counted_items(&unused)),
            PURGE,
        )));
    }

    pub(super) fn purge_unused(&mut self) {
        let unused = self.project.unused_items();
        self.project.purge(&unused);
        self.report(Report::info(format!("PURGED {}", counted_items(&unused))));
    }

    pub(super) fn new_unused(&mut self) {
        match self.focus.edit_pane() {
            Some(EditPane::Song) => {
                if let Some(slots) = self.free_chain_slots(1) {
                    let (row, track) = (self.song.row(), self.song.track());
                    self.project.set_song_cell(row, track, Some(slots[0]));
                }
            }
            Some(EditPane::Chain) => self.new_unused_phrase(),
            Some(EditPane::Phrase) => self.remark = Some(NEEDS_A_REFERENCE.into()),
            None => {}
        }
    }

    fn new_unused_phrase(&mut self) {
        let Some(EditTarget::Chain(chain)) = self.edit_target_or_remark() else {
            return;
        };
        let Some(slots) = self.free_phrase_slots(1) else {
            return;
        };
        *self.project.phrase_mut(slots[0]) = self.project.new_phrase();
        let index = self.chain.entry();
        let entry = self.project.chain_mut(chain).entry_mut(index);
        match entry {
            Some(entry) => entry.set_phrase(slots[0]),
            None => *entry = Some(ChainEntry::new(slots[0])),
        }
    }
}
