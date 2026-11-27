use crate::project::SongPlace;
use std::time::Duration;

const INFO_LIFETIME: Duration = Duration::from_secs(3);
const INFO_BRIGHT: Duration = Duration::from_secs(2);
const ATTENTION_MARK: &str = "! ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Failure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Step(SongPlace),
    SongCell { song_row: usize, track: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogItem {
    pub(super) severity: Severity,
    pub(super) text: String,
    pub(super) hint: Option<String>,
    pub(super) location: Option<Location>,
}

impl LogItem {
    pub fn new(severity: Severity, text: impl Into<String>) -> Self {
        Self {
            severity,
            text: text.into(),
            hint: None,
            location: None,
        }
    }

    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    #[must_use]
    pub fn at(self, step: SongPlace) -> Self {
        self.in_place(Location::Step(step))
    }

    #[must_use]
    pub fn in_place(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub(super) severity: Severity,
    pub(super) summary: String,
    pub(super) items: Vec<LogItem>,
}

impl Report {
    fn new(severity: Severity, summary: impl Into<String>) -> Self {
        Self {
            severity,
            summary: summary.into(),
            items: Vec::new(),
        }
    }

    pub fn info(summary: impl Into<String>) -> Self {
        Self::new(Severity::Info, summary)
    }

    pub fn warning(summary: impl Into<String>) -> Self {
        Self::new(Severity::Warning, summary)
    }

    pub fn failure(summary: impl Into<String>) -> Self {
        Self::new(Severity::Failure, summary)
    }

    #[must_use]
    pub fn with_items(mut self, items: Vec<LogItem>) -> Self {
        self.items = items;
        self
    }

    pub(super) fn status_text(&self, log_pointer: &str) -> String {
        let mark = if self.severity == Severity::Info {
            ""
        } else {
            ATTENTION_MARK
        };
        let pointer = if self.items.is_empty() {
            ""
        } else {
            log_pointer
        };
        format!("{mark}{}{pointer}", self.summary)
    }

    pub(super) fn into_log_items(self) -> Vec<LogItem> {
        if self.items.is_empty() {
            vec![LogItem::new(self.severity, self.summary)]
        } else {
            self.items
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Look {
    Info,
    FadingInfo,
    Warning,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Notice {
    severity: Severity,
    text: String,
    age: Duration,
}

impl Notice {
    pub(super) fn new(severity: Severity, text: String) -> Self {
        Self {
            severity,
            text,
            age: Duration::ZERO,
        }
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn look(&self) -> Look {
        match self.severity {
            Severity::Info if self.age < INFO_BRIGHT => Look::Info,
            Severity::Info => Look::FadingInfo,
            Severity::Warning => Look::Warning,
            Severity::Failure => Look::Failure,
        }
    }

    pub(super) fn is_persistent(&self) -> bool {
        self.severity != Severity::Info
    }

    pub(super) fn grow_older(&mut self, elapsed: Duration) -> bool {
        self.age += elapsed;
        !self.is_persistent() && self.age >= INFO_LIFETIME
    }

    pub(super) fn time_until_change(&self) -> Option<Duration> {
        if self.is_persistent() {
            None
        } else if self.age < INFO_BRIGHT {
            Some(INFO_BRIGHT - self.age)
        } else {
            Some(INFO_LIFETIME.saturating_sub(self.age))
        }
    }
}
