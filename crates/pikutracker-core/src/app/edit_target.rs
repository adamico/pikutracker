use super::App;
use super::focus::EditPane;
use super::selection::CellPosition;
use crate::project::{ChainId, PhraseId};

pub(super) const NO_CHAIN_HERE: &str = "NO CHAIN HERE";
pub(super) const NO_PHRASE_HERE: &str = "NO PHRASE HERE";
pub(super) const NO_STEP_HERE: &str = "NO STEP HERE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditTarget {
    Song,
    Chain(ChainId),
    Phrase(PhraseId),
}

impl App {
    pub(super) fn edit_target_or_remark(&mut self) -> Option<EditTarget> {
        let found = match self.focus.edit_pane()? {
            EditPane::Song => Ok(EditTarget::Song),
            EditPane::Chain => self
                .shown_chain_id()
                .map(EditTarget::Chain)
                .ok_or(NO_CHAIN_HERE),
            EditPane::Phrase => self
                .shown_phrase_id()
                .map(EditTarget::Phrase)
                .ok_or(NO_PHRASE_HERE),
        };
        found
            .map_err(|refusal| self.remark = Some(refusal.into()))
            .ok()
    }

    pub(super) fn phrase_step_or_remark(&mut self) -> Option<CellPosition> {
        let step = self.phrase.cell();
        if step.is_none() {
            self.remark = Some(NO_STEP_HERE.into());
        }
        step
    }
}
