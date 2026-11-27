use super::App;
use super::claim_text::named_slots;
use super::file_conversations::Asked;
use super::files::{Accepted, FileRequest, OVERWRITE, shown_name};
use super::key_hints::{ExportFailureHints, export_failure_hints};
use super::messages::{Location, LogItem, Report, Severity};
use crate::cart::{Cart, Slots};
use crate::export::{self, ExportFailure, Exported};
use crate::project::{ChainRowPlace, Claim, SongPlace};
use crate::wording::counted;
use std::path::{Path, PathBuf};

const LOCATE: &str = "LOCATE";

#[derive(Debug)]
pub(super) struct FinishedExport {
    claimed_bytes_hash: String,
    report: Report,
}

const fn failure_location(failure: ExportFailure) -> Option<Location> {
    match failure {
        ExportFailure::NoteOutOfRange {
            step,
            track,
            song_row,
            chain_row,
            ..
        } => Some(Location::Step(SongPlace {
            chain_row: ChainRowPlace {
                song_row,
                entry: chain_row,
            },
            track,
            step_index: step,
        })),
        ExportFailure::EmptySong => Some(Location::SongCell {
            song_row: 0,
            track: 0,
        }),
        _ => None,
    }
}

fn failure_hint(failure: ExportFailure, hints: &ExportFailureHints) -> Option<&str> {
    match failure {
        ExportFailure::EmptyClaim
        | ExportFailure::BadClaim
        | ExportFailure::TooFewSfx { .. }
        | ExportFailure::TooFewPatterns { .. }
        | ExportFailure::MissingInstrumentSlot { .. } => Some(&hints.edit_claim),
        ExportFailure::NoteOutOfRange { .. } => Some(&hints.go_to_note),
        ExportFailure::EmptySong => Some(&hints.go_to_song),
        ExportFailure::UnreadableCart(_) => None,
    }
}

fn failure_item(failure: ExportFailure, hints: &ExportFailureHints) -> LogItem {
    let mut item = LogItem::new(Severity::Failure, failure.to_string());
    if let Some(hint) = failure_hint(failure, hints) {
        item = item.with_hint(hint);
    }
    match failure_location(failure) {
        Some(location) => item.in_place(location),
        None => item,
    }
}

fn still_claimed(slots: Slots, claim: &Claim) -> Slots {
    let in_pattern_range = |slot: &u8| {
        claim
            .first_and_last_pattern
            .is_some_and(|(first, last)| (first..=last).contains(slot))
    };
    Slots {
        sfx: slots
            .sfx
            .into_iter()
            .filter(|slot| claim.sfx.contains(slot))
            .collect(),
        patterns: slots
            .patterns
            .into_iter()
            .filter(in_pattern_range)
            .collect(),
    }
}

fn changed_slots_text(changed: &Slots) -> String {
    [
        named_slots(&changed.sfx, "SFX", "SFX"),
        named_slots(&changed.patterns, "PATTERN", "PATTERNS"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" AND ")
}

fn failure_report(
    cart_name: &str,
    failures: &[ExportFailure],
    hints: &ExportFailureHints,
) -> Report {
    let summary = if failures.contains(&ExportFailure::EmptyClaim) {
        format!("EXPORT FAILED: NO CLAIM IN {cart_name}")
    } else {
        format!("EXPORT FAILED: {}", counted(failures.len(), "PROBLEM"))
    };
    let items = failures
        .iter()
        .map(|&failure| failure_item(failure, hints))
        .collect();
    Report::failure(summary).with_items(items)
}

impl App {
    pub(super) fn target_cart_folder(&self) -> &Path {
        self.project_path
            .as_deref()
            .and_then(Path::parent)
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(&self.directory)
    }

    pub(super) fn target_cart_path(&self, cart: &str) -> PathBuf {
        self.target_cart_folder().join(cart)
    }

    pub(super) fn export(&mut self) {
        self.close_cart_view();
        let Some(target) = self.project.target_cart() else {
            self.open_cart_view();
            return;
        };
        let path = self.target_cart_path(&target.path);
        self.files
            .ask(FileRequest::ReadBytes { path }, Asked::ExportReading);
    }

    pub(super) fn finish_reading_target(&mut self, path: PathBuf, original: &[u8]) {
        let cart_name = shown_name(&path);
        let Some(target) = self.project.target_cart() else {
            return;
        };
        let changed = target
            .claimed_bytes_hash
            .as_deref()
            .zip(Cart::read(original).ok())
            .map(|(hash, cart)| still_claimed(cart.changed_since(hash), &target.claim))
            .unwrap_or_default();
        match export::export(&self.project, &target.claim, original) {
            Ok(exported) if changed == Slots::default() => self.write_target(path, exported),
            Ok(exported) => {
                let text = format!("{} CHANGED IN {cart_name}", changed_slots_text(&changed));
                self.ask(
                    text,
                    OVERWRITE,
                    Accepted::OverwriteCart(path, Box::new(exported)),
                );
            }
            Err(failures) => {
                let hints = export_failure_hints(self.keyboard.keymap());
                self.report(failure_report(&cart_name, &failures, &hints));
            }
        }
    }

    pub(super) fn ask_to_locate(&mut self, path: &Path) {
        let text = format!("{} NOT FOUND", shown_name(path));
        self.ask(text, LOCATE, Accepted::LocateCart);
    }

    pub(super) fn locate_target_cart(&mut self, path: &Path) {
        self.overlays.close_file_list();
        let Some(mut target) = self.project.target_cart().cloned() else {
            return;
        };
        target.path = self.path_from_project(path);
        self.project.set_target_cart(Some(target));
        self.export();
    }

    pub(super) fn write_target(&mut self, path: PathBuf, exported: Exported) {
        let cart_name = shown_name(&path);
        let report = Report::info(format!(
            "EXPORTED TO {cart_name}: {} SFX, {} PATTERNS.",
            exported.sfx_count, exported.pattern_count,
        ));
        let finished = FinishedExport {
            claimed_bytes_hash: exported.claimed_bytes_hash,
            report,
        };
        self.files.ask(
            FileRequest::WriteBytes {
                path,
                contents: exported.cart_bytes,
            },
            Asked::ExportWriting(finished),
        );
    }

    pub(super) fn finish_writing_target(&mut self, finished: FinishedExport) {
        self.record_claimed_bytes_hash(finished.claimed_bytes_hash);
        self.report(finished.report);
    }

    pub(super) fn record_claimed_bytes_hash(&mut self, hash: String) {
        if let Some(target) = self.project.target_cart() {
            let mut target = target.clone();
            target.claimed_bytes_hash = Some(hash);
            self.project.set_target_cart(Some(target));
        }
    }
}
