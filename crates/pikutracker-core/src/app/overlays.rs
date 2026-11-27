use super::action::KeyContext;
use super::change_speed::{self, SpeedEntry};
use super::files::Question;
use super::focus::Focus;
use super::key_hints;
use super::layout::{self, Area};
use super::panes::cart_view::CartView;
use super::panes::file_list::FileList;
use super::panes::help::HelpList;
use super::panes::instrument::InstrumentView;
use super::panes::midi_dialog::MidiDialog;
use crate::keymap::Keymap;
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

    fn key_context(&self, project: &Project) -> KeyContext {
        match self {
            Overlay::SpeedEntry(_) => KeyContext::SpeedEntry,
            Overlay::Question(_) => KeyContext::Navigation,
            Overlay::MidiDialog(_) => KeyContext::MidiImport,
            Overlay::FileList(list) if list.purpose().takes_a_name() => KeyContext::FileName,
            Overlay::FileList(_) => KeyContext::Navigation,
            Overlay::InstrumentView(view) => view.key_context(project),
        }
    }

    fn help(&self, away_from_focus: Area) -> Option<(HelpList, Area)> {
        match self {
            Overlay::SpeedEntry(_) => Some((HelpList::SpeedEntry, away_from_focus)),
            Overlay::Question(_) | Overlay::InstrumentView(_) => None,
            Overlay::MidiDialog(dialog) => {
                let keys = dialog.help_list();
                let area = if keys == HelpList::MidiDrumKit {
                    layout::LEFT_AREA
                } else {
                    layout::RIGHT_SLOT
                };
                Some((keys, area))
            }
            Overlay::FileList(list) if list.purpose().takes_a_name() => {
                Some((HelpList::SaveList, layout::LEFT_AREA))
            }
            Overlay::FileList(_) => Some((HelpList::OpenList, layout::LEFT_AREA)),
        }
    }

    fn asked(&self, keymap: &Keymap) -> Option<String> {
        match self {
            Overlay::SpeedEntry(entry) => Some(key_hints::asked(
                keymap,
                KeyContext::SpeedEntry,
                entry.text(),
                change_speed::APPLY,
            )),
            Overlay::Question(question) => Some(key_hints::asked(
                keymap,
                KeyContext::Navigation,
                &question.text,
                question.answer,
            )),
            Overlay::MidiDialog(_) | Overlay::FileList(_) | Overlay::InstrumentView(_) => None,
        }
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

#[derive(Debug)]
enum SlotView {
    Log,
    CartView(CartView),
}

#[derive(Debug)]
struct OpenSlotView {
    view: SlotView,
    focus_returns_to: Focus,
}

#[derive(Debug, Default)]
struct SlotViews {
    open: Vec<OpenSlotView>,
}

impl SlotViews {
    fn top(&self) -> Option<&SlotView> {
        self.open.last().map(|open| &open.view)
    }

    fn log_on_top(&self) -> bool {
        matches!(self.top(), Some(SlotView::Log))
    }

    fn cart_view_on_top(&self) -> bool {
        matches!(self.top(), Some(SlotView::CartView(_)))
    }

    fn open(&mut self, view: SlotView, focus_returns_to: Focus) {
        let same_kind = std::mem::discriminant(&view);
        self.open
            .retain(|open| std::mem::discriminant(&open.view) != same_kind);
        self.open.push(OpenSlotView {
            view,
            focus_returns_to,
        });
    }

    fn close(&mut self, is_it: fn(&SlotView) -> bool, focus: Focus) -> Option<Focus> {
        let index = self.open.iter().position(|open| is_it(&open.view))?;
        let on_top = index + 1 == self.open.len();
        let closed = self.open.remove(index);
        (on_top && focus == Focus::RightSlot).then_some(closed.focus_returns_to)
    }

    fn cart_view(&mut self) -> Option<&mut CartView> {
        self.open.iter_mut().find_map(|open| match &mut open.view {
            SlotView::CartView(view) => Some(view),
            SlotView::Log => None,
        })
    }

    fn reset_for_new_project(&mut self) {
        self.open.retain(|open| matches!(open.view, SlotView::Log));
        for open in &mut self.open {
            open.focus_returns_to = Focus::Song;
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Overlays {
    stack: Vec<Overlay>,
    slot_views: SlotViews,
    help_open: bool,
}

macro_rules! overlay_access {
    ($variant:ident, $ty:ty, close: $close:ident $(, take: $take:ident)? $(, get: $get:ident)?) => {
        pub(super) fn $close(&mut self) {
            self.remove(|overlay| matches!(overlay, Overlay::$variant(_)));
        }

        $(
            pub(super) fn $take(&mut self) -> Option<$ty> {
                match self.remove(|overlay| matches!(overlay, Overlay::$variant(_)))? {
                    Overlay::$variant(inner) => Some(inner),
                    _ => None,
                }
            }
        )?

        $(
            pub(super) fn $get(&mut self) -> Option<&mut $ty> {
                self.stack.iter_mut().rev().find_map(|overlay| match overlay {
                    Overlay::$variant(inner) => Some(inner),
                    _ => None,
                })
            }
        )?
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

    pub(super) fn midi_dialog_on_top(&self) -> bool {
        matches!(self.top(), Some(Overlay::MidiDialog(_)))
    }

    fn remove(&mut self, is_it: fn(&Overlay) -> bool) -> Option<Overlay> {
        let index = self.stack.iter().rposition(is_it)?;
        Some(self.stack.remove(index))
    }

    overlay_access!(SpeedEntry, SpeedEntry, close: close_speed_entry, take: take_speed_entry, get: speed_entry);
    overlay_access!(Question, Question, close: close_question, take: take_question);
    overlay_access!(MidiDialog, Box<MidiDialog>, close: close_midi_dialog, take: take_midi_dialog, get: midi_dialog);
    overlay_access!(FileList, FileList, close: close_file_list, get: file_list);
    overlay_access!(InstrumentView, InstrumentView, close: close_instrument_view, get: instrument_view);

    pub(super) fn reset_for_new_project(&mut self) {
        self.stack.clear();
        self.slot_views.reset_for_new_project();
    }

    fn shown_file_list(&self) -> Option<&FileList> {
        self.stack.iter().rev().find_map(|overlay| match overlay {
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
        if let Some(list) = self.shown_file_list() {
            return RightSlotView::FileList(list);
        }
        match self.slot_views.top() {
            Some(SlotView::Log) => RightSlotView::Log,
            Some(SlotView::CartView(view)) => RightSlotView::CartView(view),
            None => RightSlotView::Multitrack,
        }
    }

    pub(super) fn panes_show_focus(&self) -> bool {
        self.shown_file_list().is_none()
    }

    pub(super) fn key_context(&self, project: &Project, focus: Focus) -> Option<KeyContext> {
        match self.top() {
            Some(overlay) => Some(overlay.key_context(project)),
            None if self.cart_view_has_focus(focus) => Some(KeyContext::ClaimText),
            None => None,
        }
    }

    pub(super) fn asked(&self, keymap: &Keymap) -> Option<String> {
        self.top()?.asked(keymap)
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
        if let Some(overlay) = self.top() {
            return overlay.help(away_from_focus);
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

    pub(super) fn log_on_top(&self) -> bool {
        self.slot_views.log_on_top()
    }

    pub(super) fn log_has_focus(&self, focus: Focus) -> bool {
        focus == Focus::RightSlot && self.slot_views.log_on_top()
    }

    pub(super) fn open_log(&mut self, focus_returns_to: Focus) {
        self.slot_views.open(SlotView::Log, focus_returns_to);
    }

    pub(super) fn open_log_for_a_report(&mut self, focus: Focus) {
        if !self.slot_views.log_on_top() {
            self.open_log(focus);
            self.help_open = false;
        }
    }

    pub(super) fn close_log(&mut self, focus: Focus) -> Option<Focus> {
        let is_log: fn(&SlotView) -> bool = |view| matches!(view, SlotView::Log);
        self.slot_views.close(is_log, focus)
    }

    pub(super) fn cart_view(&mut self) -> Option<&mut CartView> {
        self.slot_views.cart_view()
    }

    pub(super) fn cart_view_on_top(&self) -> bool {
        self.slot_views.cart_view_on_top()
    }

    pub(super) fn cart_view_has_focus(&self, focus: Focus) -> bool {
        focus == Focus::RightSlot && self.slot_views.cart_view_on_top()
    }

    pub(super) fn open_cart_view(&mut self, view: CartView, focus_returns_to: Focus) {
        self.slot_views
            .open(SlotView::CartView(view), focus_returns_to);
    }

    pub(super) fn close_cart_view(&mut self, focus: Focus) -> Option<Focus> {
        let is_cart_view: fn(&SlotView) -> bool = |view| matches!(view, SlotView::CartView(_));
        self.slot_views.close(is_cart_view, focus)
    }
}
