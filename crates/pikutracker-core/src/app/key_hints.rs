use super::{Action, Direction, KeyContext};
use crate::keymap::{HINT_GAP, KeyHint, Keymap};

pub(super) const UP_DOWN: [Action; 2] =
    [Action::Move(Direction::Up), Action::Move(Direction::Down)];
pub(super) const CONFIRM: [Action; 1] = [Action::Confirm];
const CART: [Action; 1] = [Action::ToggleCart];
pub(super) const PICK: &str = "PICK";
pub(super) const CHOOSE: &str = "CHOOSE";
const TYPE_A_NAME: &str = "TYPE A NAME";
const EDIT_CLAIM: &str = "EDIT CLAIM";
const PICKS_ONE: &str = "PICKS ONE";

pub(super) fn asked(keymap: &Keymap, context: KeyContext, question: &str, answer: &str) -> String {
    format!("{question}{HINT_GAP}{}", answers(keymap, context, answer))
}

fn answers(keymap: &Keymap, context: KeyContext, answer: &str) -> String {
    keymap.hint_line(
        context,
        &[
            KeyHint::labelled(&CONFIRM, answer),
            KeyHint::named(&Action::Cancel),
        ],
    )
}

pub(super) fn open_list_keys(keymap: &Keymap) -> String {
    list_keys(keymap, KeyContext::Navigation)
}

pub(super) fn save_list_keys(keymap: &Keymap) -> String {
    format!(
        "{TYPE_A_NAME}{HINT_GAP}{}",
        list_keys(keymap, KeyContext::FileName)
    )
}

fn list_keys(keymap: &Keymap, context: KeyContext) -> String {
    format!(
        "{}{HINT_GAP}{}",
        keymap.hint_line(context, &[KeyHint::labelled(&UP_DOWN, PICK)]),
        answers(keymap, context, CHOOSE)
    )
}

pub(super) fn log_pointer(keymap: &Keymap) -> String {
    let hint = keymap.hint_line(
        KeyContext::Navigation,
        &[KeyHint::named(&Action::ToggleLog)],
    );
    format!("{HINT_GAP}{hint}")
}

pub(super) fn picks_one(keymap: &Keymap) -> String {
    keymap.hint_line(
        KeyContext::ClaimText,
        &[KeyHint::labelled(&CONFIRM, PICKS_ONE)],
    )
}

pub(super) fn edit_claim_hint(keymap: &Keymap) -> String {
    keymap.hint_line(
        KeyContext::Navigation,
        &[KeyHint::labelled(&CART, EDIT_CLAIM)],
    )
}

pub(super) fn help_hint(keymap: &Keymap) -> String {
    keymap.hint_line(
        KeyContext::Navigation,
        &[KeyHint::named(&Action::ToggleHelp)],
    )
}
