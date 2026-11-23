use crate::project::{ChainEntry, ChainId, PhraseId, Project};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Shown {
    chain: Option<ChainId>,
    phrase: Option<PhraseId>,
    song_cursor_saw: Option<SongCursorSight>,
    chain_cursor_saw: Option<ChainCursorSight>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SongCursorSight {
    row: usize,
    track: usize,
    chain: Option<ChainId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChainCursorSight {
    chain: Option<ChainId>,
    entry: usize,
    phrase: Option<PhraseId>,
}

impl Shown {
    pub(super) fn chain(&self) -> Option<ChainId> {
        self.chain
    }

    pub(super) fn phrase(&self) -> Option<PhraseId> {
        self.phrase
    }

    pub(super) fn open_chain(&mut self, chain: ChainId) {
        self.chain = Some(chain);
    }

    pub(super) fn open_phrase(&mut self, phrase: PhraseId) {
        self.phrase = Some(phrase);
    }

    pub(super) fn follow_cursors(
        &mut self,
        project: &Project,
        (row, track): (usize, usize),
        entry: usize,
    ) {
        let song_sight = SongCursorSight {
            row,
            track,
            chain: project.song_cell(row, track),
        };
        if self.song_cursor_saw != Some(song_sight) {
            self.song_cursor_saw = Some(song_sight);
            self.chain = song_sight.chain;
        }
        let chain_sight = ChainCursorSight {
            chain: self.chain,
            entry,
            phrase: self
                .chain
                .and_then(|chain| project.chain(chain).entry(entry))
                .map(ChainEntry::phrase),
        };
        if self.chain_cursor_saw != Some(chain_sight) {
            self.chain_cursor_saw = Some(chain_sight);
            self.phrase = chain_sight.phrase;
        }
    }
}
