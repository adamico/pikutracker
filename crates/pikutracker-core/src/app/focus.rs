#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Song,
    Chain,
    Phrase,
    RightSlot,
}

impl Focus {
    pub const fn next(self) -> Self {
        match self {
            Focus::Phrase => Focus::Chain,
            Focus::Chain => Focus::Song,
            Focus::Song => Focus::RightSlot,
            Focus::RightSlot => Focus::Phrase,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Focus::Phrase => Focus::RightSlot,
            Focus::Chain => Focus::Phrase,
            Focus::Song => Focus::Chain,
            Focus::RightSlot => Focus::Song,
        }
    }
}
