use super::{CHAIN_ENTRIES, Project, SONG_ROWS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongRowEnding {
    LoopBack,
    Stop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SongRowMarks {
    pub loop_start: bool,
    pub ending: Option<SongRowEnding>,
}

impl SongRowMarks {
    pub fn toggle_loop_start(&mut self) {
        self.loop_start = !self.loop_start;
    }

    pub fn toggle_ending(&mut self, ending: SongRowEnding) {
        self.ending = (self.ending != Some(ending)).then_some(ending);
    }

    pub fn loops_back(self) -> bool {
        self.ending == Some(SongRowEnding::LoopBack)
    }

    pub fn stops(self) -> bool {
        self.ending == Some(SongRowEnding::Stop)
    }
}

impl Project {
    pub fn song_row_marks(&self, row: usize) -> SongRowMarks {
        self.song_marks[row]
    }

    pub fn set_song_row_marks(&mut self, row: usize, marks: SongRowMarks) {
        self.song_marks[row] = marks;
    }

    pub fn played_chain_rows(&self, row: usize) -> impl Iterator<Item = usize> + '_ {
        (0..CHAIN_ENTRIES).filter(move |&chain_row| self.chain_row_ticks(row, chain_row) > 0)
    }

    pub fn song_row_after(&self, row: usize) -> Option<usize> {
        let marks = self.song_row_marks(row);
        let next = if marks.stops() {
            None
        } else if marks.loops_back() {
            Some(self.loop_start_at_or_before(row).unwrap_or(0))
        } else {
            Some(row + 1).filter(|&next| next < SONG_ROWS)
        };
        next.filter(|&next| !self.song_row_is_empty(next))
    }

    pub fn loop_start_at_or_before(&self, row: usize) -> Option<usize> {
        (0..=row)
            .rev()
            .find(|&row| self.song_row_marks(row).loop_start)
    }

    pub fn last_played_song_row(&self) -> Option<usize> {
        (0..SONG_ROWS)
            .rev()
            .find(|&row| !self.song_row_is_empty(row))
    }
}
