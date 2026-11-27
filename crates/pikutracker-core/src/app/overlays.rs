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

    fn sits_on_the_status_line(&self) -> bool {
        matches!(self, Overlay::SpeedEntry(_) | Overlay::Question(_))
    }
}

pub(super) enum Screen<'a> {
    Panes,
    MidiDialog(&'a MidiDialog),
    InstrumentView(&'a InstrumentView),
}

pub(super) enum RightSlotView<'a> {
    FileList(&'a FileList),
    Log,
    CartView(&'a CartView),
    Multitrack,
}

#[derive(Debug, Default)]
pub(super) struct Overlays {
    stack: Vec<Overlay>,
    log_returns_to: Option<Focus>,
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

    pub(super) fn reset_for_new_project(&mut self) {
        self.stack.clear();
        self.cart_view = None;
        if let Some(return_to) = self.log_returns_to.as_mut() {
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
            .find(|overlay| !overlay.sits_on_the_status_line());
        match beneath_the_status_line {
            Some(Overlay::MidiDialog(dialog)) => Screen::MidiDialog(dialog),
            Some(Overlay::InstrumentView(view)) => Screen::InstrumentView(view),
            _ => Screen::Panes,
        }
    }

    pub(super) fn right_slot(&self) -> RightSlotView<'_> {
        if let Some(list) = self.open_file_list() {
            RightSlotView::FileList(list)
        } else if self.log_returns_to.is_some() {
            RightSlotView::Log
        } else if let Some(view) = &self.cart_view {
            RightSlotView::CartView(view)
        } else {
            RightSlotView::Multitrack
        }
    }

    pub(super) fn panes_show_focus(&self) -> bool {
        self.open_file_list().is_none()
    }

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
        self.log_returns_to.is_some()
    }

    pub(super) fn log_has_focus(&self, focus: Focus) -> bool {
        self.log_returns_to.is_some() && focus == Focus::RightSlot
    }

    pub(super) fn open_log(&mut self, focus: Focus) {
        if self.log_returns_to.is_none() {
            self.log_returns_to = Some(focus);
        }
    }

    pub(super) fn open_log_for_a_report(&mut self, focus: Focus) {
        if self.log_returns_to.is_none() {
            self.log_returns_to = Some(focus);
            self.help_open = false;
        }
    }

    pub(super) fn close_log(&mut self) -> Option<Focus> {
        self.log_returns_to.take()
    }

    pub(super) fn cart_view(&mut self) -> Option<&mut CartView> {
        self.cart_view.as_mut()
    }

    pub(super) fn cart_view_open(&self) -> bool {
        self.cart_view.is_some()
    }

    pub(super) fn cart_view_has_focus(&self, focus: Focus) -> bool {
        focus == Focus::RightSlot && self.log_returns_to.is_none() && self.cart_view.is_some()
    }

    pub(super) fn open_cart_view(&mut self, view: CartView) {
        self.log_returns_to = None;
        self.cart_view = Some(view);
    }

    pub(super) fn take_cart_view(&mut self) -> Option<CartView> {
        self.cart_view.take()
    }
}
