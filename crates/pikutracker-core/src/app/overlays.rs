//! What sits over the panes, and so what is on top: who gets the key, which
//! Help list shows, what fills the right-hand slot and where focus returns.
//!
//! Overlays that take every key stack up newest on top, so closing one shows
//! the one it opened over. The Log and the Cart view only fill the right-hand
//! slot; the Log covers the Cart view while both are open.

use super::action::KeyContext;
use super::change_speed::SpeedEntry;
use super::files::Question;
use super::focus::Focus;
use super::layout::{self, Area};
use super::panes::cart_view::CartView;
use super::panes::file_list::FileList;
use super::panes::help::HelpList;
use super::panes::instrument::InstrumentView;
use super::panes::midi_dialog::MidiDialog;
use crate::project::Project;

/// An overlay that takes every key while it is on top.
#[derive(Debug)]
pub(super) enum Overlay {
    SpeedEntry(SpeedEntry),
    Question(Question),
    MidiDialog(Box<MidiDialog>),
    FileList(FileList),
    InstrumentView(InstrumentView),
}

impl Overlay {
    fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// Speed entry and questions sit on the status line and leave the screen
    /// beneath them showing.
    fn on_the_status_line(&self) -> bool {
        matches!(self, Overlay::SpeedEntry(_) | Overlay::Question(_))
    }
}

/// What fills the whole grid.
pub(super) enum Screen<'a> {
    Panes,
    MidiDialog(&'a MidiDialog),
    InstrumentView(&'a InstrumentView),
}

/// What fills the right-hand slot when the panes show, Help aside.
pub(super) enum RightSlotView<'a> {
    FileList(&'a FileList),
    Log,
    CartView(&'a CartView),
    Multitrack,
}

#[derive(Debug, Default)]
pub(super) struct Overlays {
    stack: Vec<Overlay>,
    /// Where focus returns when the open Log closes.
    log: Option<Focus>,
    cart_view: Option<CartView>,
    help_open: bool,
}

macro_rules! overlay_access {
    ($variant:ident, $ty:ty, $take:ident $(, $get:ident)?) => {
        $(
            pub(super) fn $get(&mut self) -> Option<&mut $ty> {
                self.stack.iter_mut().rev().find_map(|overlay| match overlay {
                    Overlay::$variant(it) => Some(it),
                    _ => None,
                })
            }
        )?

        pub(super) fn $take(&mut self) -> Option<$ty> {
            let index = self
                .stack
                .iter()
                .rposition(|overlay| matches!(overlay, Overlay::$variant(_)))?;
            match self.stack.remove(index) {
                Overlay::$variant(it) => Some(it),
                _ => None,
            }
        }
    };
}

impl Overlays {
    /// Puts `overlay` on top, in place of any open overlay of its kind.
    pub(super) fn open(&mut self, overlay: Overlay) {
        self.stack.retain(|open| !open.same_kind(&overlay));
        self.stack.push(overlay);
    }

    pub(super) fn top(&self) -> Option<&Overlay> {
        self.stack.last()
    }

    overlay_access!(SpeedEntry, SpeedEntry, take_speed_entry, speed_entry);
    overlay_access!(Question, Question, take_question);
    overlay_access!(MidiDialog, Box<MidiDialog>, take_midi_dialog, midi_dialog);
    overlay_access!(FileList, FileList, take_file_list, file_list);
    overlay_access!(
        InstrumentView,
        InstrumentView,
        take_instrument_view,
        instrument_view
    );

    /// Opening another Project closes every overlay and the Cart view, and a
    /// Log left open returns focus to the Song pane.
    pub(super) fn reset_for_new_project(&mut self) {
        self.stack.clear();
        self.cart_view = None;
        if let Some(return_to) = self.log.as_mut() {
            *return_to = Focus::Song;
        }
    }

    fn open_file_list(&self) -> Option<&FileList> {
        self.stack.iter().find_map(|overlay| match overlay {
            Overlay::FileList(list) => Some(list),
            _ => None,
        })
    }

    pub(super) fn screen(&self) -> Screen<'_> {
        let beneath_the_status_line = self
            .stack
            .iter()
            .rev()
            .find(|overlay| !overlay.on_the_status_line());
        match beneath_the_status_line {
            Some(Overlay::MidiDialog(dialog)) => Screen::MidiDialog(dialog),
            Some(Overlay::InstrumentView(view)) => Screen::InstrumentView(view),
            _ => Screen::Panes,
        }
    }

    pub(super) fn right_slot(&self) -> RightSlotView<'_> {
        if let Some(list) = self.open_file_list() {
            RightSlotView::FileList(list)
        } else if self.log.is_some() {
            RightSlotView::Log
        } else if let Some(view) = &self.cart_view {
            RightSlotView::CartView(view)
        } else {
            RightSlotView::Multitrack
        }
    }

    /// Whether the focused pane draws as focused: not while the File list
    /// takes the keys.
    pub(super) fn panes_have_focus(&self) -> bool {
        self.open_file_list().is_none()
    }

    /// The keys an overlay takes, or None when the panes take them.
    pub(super) fn key_context(&self, project: &Project, focus: Focus) -> Option<KeyContext> {
        match self.top() {
            Some(Overlay::SpeedEntry(_)) => Some(KeyContext::SpeedEntry),
            Some(Overlay::Question(_)) => Some(KeyContext::Navigation),
            Some(Overlay::MidiDialog(_)) => Some(KeyContext::MidiImport),
            Some(Overlay::FileList(list)) if list.purpose().takes_a_name() => {
                Some(KeyContext::FileName)
            }
            Some(Overlay::FileList(_)) => Some(KeyContext::Navigation),
            Some(Overlay::InstrumentView(view)) => Some(view.key_context(project)),
            None if self.cart_view_has_focus(focus) => Some(KeyContext::ClaimText),
            None => None,
        }
    }

    pub(super) fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
    }

    /// The Help list to draw and where, if Help is open and has one here.
    pub(super) fn help(&self, focus: Focus) -> Option<(HelpList, Area)> {
        if !self.help_open {
            return None;
        }
        let away_from_focus = if focus == Focus::RightSlot {
            layout::LEFT_AREA
        } else {
            layout::RIGHT_SLOT
        };
        match self.top() {
            Some(Overlay::SpeedEntry(_)) => return Some((HelpList::SpeedEntry, away_from_focus)),
            Some(Overlay::Question(_) | Overlay::InstrumentView(_)) => return None,
            Some(Overlay::MidiDialog(dialog)) => {
                let keys = dialog.help_list();
                let area = if keys == HelpList::MidiDrumKit {
                    layout::LEFT_AREA
                } else {
                    layout::RIGHT_SLOT
                };
                return Some((keys, area));
            }
            Some(Overlay::FileList(list)) => {
                let keys = if list.purpose().takes_a_name() {
                    HelpList::SaveList
                } else {
                    HelpList::OpenList
                };
                return Some((keys, layout::LEFT_AREA));
            }
            None => {}
        }
        let keys = match (focus, self.right_slot()) {
            (Focus::Phrase, _) => HelpList::Phrase,
            (Focus::Chain, _) => HelpList::Chain,
            (Focus::Song, _) => HelpList::Song,
            (Focus::RightSlot, RightSlotView::Log) => HelpList::Log,
            (Focus::RightSlot, RightSlotView::CartView(_)) => HelpList::CartView,
            (Focus::RightSlot, RightSlotView::Multitrack | RightSlotView::FileList(_)) => {
                return None;
            }
        };
        Some((keys, away_from_focus))
    }

    pub(super) fn log_open(&self) -> bool {
        self.log.is_some()
    }

    pub(super) fn log_has_focus(&self, focus: Focus) -> bool {
        self.log.is_some() && focus == Focus::RightSlot
    }

    /// Opens the Log over the right-hand slot, remembering `focus` to return to.
    pub(super) fn open_log(&mut self, focus: Focus) {
        if self.log.is_none() {
            self.log = Some(focus);
        }
    }

    /// A result with items opens the Log the same way and closes Help.
    pub(super) fn open_log_for_a_report(&mut self, focus: Focus) {
        if self.log.is_none() {
            self.log = Some(focus);
            self.help_open = false;
        }
    }

    /// Closes the Log, giving the focus it was opened from.
    pub(super) fn close_log(&mut self) -> Option<Focus> {
        self.log.take()
    }

    pub(super) fn cart_view(&mut self) -> Option<&mut CartView> {
        self.cart_view.as_mut()
    }

    pub(super) fn cart_view_open(&self) -> bool {
        self.cart_view.is_some()
    }

    pub(super) fn cart_view_has_focus(&self, focus: Focus) -> bool {
        focus == Focus::RightSlot && self.log.is_none() && self.cart_view.is_some()
    }

    /// Shows the Cart view in the right-hand slot, closing the Log.
    pub(super) fn open_cart_view(&mut self, view: CartView) {
        self.log = None;
        self.cart_view = Some(view);
    }

    pub(super) fn take_cart_view(&mut self) -> Option<CartView> {
        self.cart_view.take()
    }
}
