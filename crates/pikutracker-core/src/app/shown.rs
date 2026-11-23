use crate::project::{ChainEntry, ChainId, PhraseId, Project, Track};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Shown {
    chain: Option<ChainId>,
    phrase: Option<PhraseId>,
    song_cursor_last_seen: Option<SongCursorPlaceSeen>,
    chain_cursor_last_seen: Option<ChainCursorPlaceSeen>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CursorPlaces {
    pub(super) song_row: usize,
    pub(super) track: Track,
    pub(super) chain_entry: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SongCursorPlaceSeen {
    row: usize,
    track: Track,
    chain: Option<ChainId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChainCursorPlaceSeen {
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

    pub(super) fn follow_cursors(&mut self, project: &Project, cursors: CursorPlaces) {
        let CursorPlaces {
            song_row,
            track,
            chain_entry,
        } = cursors;
        let song_cursor = SongCursorPlaceSeen {
            row: song_row,
            track,
            chain: project.song_cell(song_row, track.index()),
        };
        if self.song_cursor_last_seen != Some(song_cursor) {
            self.song_cursor_last_seen = Some(song_cursor);
            self.chain = song_cursor.chain;
        }
        let chain_cursor = ChainCursorPlaceSeen {
            chain: self.chain,
            entry: chain_entry,
            phrase: self
                .chain
                .and_then(|chain| project.chain(chain).entry(chain_entry))
                .map(ChainEntry::phrase),
        };
        if self.chain_cursor_last_seen != Some(chain_cursor) {
            self.chain_cursor_last_seen = Some(chain_cursor);
            self.phrase = chain_cursor.phrase;
        }
    }
}
