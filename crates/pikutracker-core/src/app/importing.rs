use super::files::{folder_of, shown_name};
use super::messages::{LogItem, Report, Severity};
use super::panes::midi_dialog::{MidiDialog, Outcome};
use super::{Action, App};
use crate::cart::Cart;
use crate::cart_import;
use crate::midi_import;
use crate::project::SongPlace;
use crate::wording::counted;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImportKind {
    Cart,
    Midi,
}

pub(super) const TEXT_CART_EXTENSION: &str = ".p8";
pub(super) const PNG_CART_EXTENSION: &str = ".p8.png";

pub(super) fn import_kind(file_name: &str) -> Option<ImportKind> {
    let name = file_name.to_lowercase();
    if name.ends_with(TEXT_CART_EXTENSION) || name.ends_with(PNG_CART_EXTENSION) {
        Some(ImportKind::Cart)
    } else if name.ends_with(".mid") || name.ends_with(".midi") {
        Some(ImportKind::Midi)
    } else {
        None
    }
}

fn warning_item(text: String, place: Option<SongPlace>) -> LogItem {
    let item = LogItem::new(Severity::Warning, text);
    match place {
        Some(place) => item.at(place),
        None => item,
    }
}

fn import_report(source: &str, warnings: Vec<LogItem>) -> Report {
    if warnings.is_empty() {
        return Report::info(format!("IMPORTED {source}"));
    }
    let count = counted(warnings.len(), "WARNING");
    Report::warning(format!("IMPORTED {source} WITH {count}")).with_items(warnings)
}

impl App {
    pub(super) fn finish_importing(&mut self, path: PathBuf, contents: &[u8]) {
        self.file_list = None;
        let file_name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        match import_kind(&file_name) {
            Some(ImportKind::Cart) => self.import_cart(path, &file_name, contents),
            Some(ImportKind::Midi) => self.start_midi_import(path, contents),
            None => {}
        }
    }

    pub fn start_midi_import(&mut self, path: impl AsRef<Path>, contents: &[u8]) {
        let path = path.as_ref();
        let name = shown_name(path);
        match midi_import::analyse(contents) {
            Ok(analysis) => {
                let table = self.midi_table.clone();
                let dialog = MidiDialog::new(name, folder_of(path), analysis, table);
                self.midi_dialog = Some(Box::new(dialog));
            }
            Err(error) => self.report(Report::failure(format!("CANNOT IMPORT {name}: {error}"))),
        }
    }

    pub(super) fn use_midi_dialog(&mut self, action: Action) {
        let Some(dialog) = self.midi_dialog.as_mut() else {
            return;
        };
        match dialog.apply(action) {
            Outcome::Stay => {}
            Outcome::Cancel => self.midi_dialog = None,
            Outcome::Import => {
                if let Some(dialog) = self.midi_dialog.take() {
                    let (name, folder, imported, table) = dialog.finish();
                    self.midi_table = table;
                    self.directory = folder;
                    self.replace_project(imported.project, None);
                    let warnings = imported.warnings.into_iter();
                    let items = warnings.map(|w| warning_item(w.text, w.place)).collect();
                    self.report(import_report(&name, items));
                }
            }
        }
    }

    fn import_cart(&mut self, path: PathBuf, file_name: &str, contents: &[u8]) {
        let name = shown_name(&path);
        let cart = match Cart::read(contents) {
            Ok(cart) => cart,
            Err(error) => {
                self.report(Report::failure(format!("CANNOT IMPORT {name}: {error}")));
                return;
            }
        };
        let imported = cart_import::import(&cart, file_name);
        self.directory = folder_of(&path);
        self.replace_project(imported.project, None);
        let warnings = imported.warnings.into_iter();
        let items = warnings.map(|w| warning_item(w.text, w.place)).collect();
        self.report(import_report(&name, items));
    }
}
