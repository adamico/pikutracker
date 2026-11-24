use pikutracker_core::app::{Action, Direction, KeyContext};
use pikutracker_core::keymap::{Key, Keymap, Modifiers, Preset, Trigger};
use std::collections::BTreeMap;

const NAVIGATION: KeyContext = KeyContext::Navigation;

fn key_names(keymap: &Keymap, action: Action, context: KeyContext) -> Vec<String> {
    keymap
        .triggers_for(action, context)
        .iter()
        .map(Trigger::to_string)
        .collect()
}

fn rebound(keys: &[(&str, &str)]) -> Keymap {
    let keys: BTreeMap<String, String> = keys
        .iter()
        .map(|&(action, key)| (action.to_owned(), key.to_owned()))
        .collect();
    let (keymap, problems) = Keymap::rebound(Preset::PcTracker, &keys);
    assert_eq!(problems, []);
    keymap
}

#[test]
fn the_cart_view_is_ctrl_t_in_pc_tracker_and_x_plus_t_in_m8_style() {
    assert_eq!(
        key_names(&Keymap::pc_tracker(), Action::ToggleCart, NAVIGATION),
        ["^T"]
    );
    assert_eq!(
        key_names(&Keymap::m8_style(), Action::ToggleCart, NAVIGATION),
        ["X+T"]
    );
}

#[test]
fn a_key_changed_in_config_ron_replaces_the_presets_key() {
    let keymap = rebound(&[("ToggleCart", "Ctrl+K"), ("Cancel", "Backspace")]);

    assert_eq!(key_names(&keymap, Action::ToggleCart, NAVIGATION), ["^K"]);
    assert_eq!(
        key_names(&keymap, Action::Cancel, NAVIGATION),
        ["BACKSPACE"]
    );
}

#[test]
fn an_action_with_several_keys_lists_the_plain_key_first() {
    assert_eq!(
        key_names(&Keymap::m8_style(), Action::FocusNext, NAVIGATION),
        ["TAB", "SHIFT+DOWN", "SHIFT+RIGHT"]
    );
}

#[test]
fn a_key_taken_by_the_context_does_not_count_for_the_wider_action() {
    let keymap = Keymap::pc_tracker();

    assert_eq!(key_names(&keymap, Action::FocusNext, NAVIGATION), ["TAB"]);
    assert_eq!(
        key_names(&keymap, Action::FocusNext, KeyContext::MidiImport),
        Vec::<String>::new()
    );
}

#[test]
fn keys_are_named_as_the_status_line_shows_them() {
    let names = [
        (Trigger::Press(Key::Enter, Modifiers::NONE), "ENTER"),
        (Trigger::Press(Key::Escape, Modifiers::NONE), "ESC"),
        (Trigger::Press(Key::Char('r'), Modifiers::SHIFT), "SHIFT+R"),
        (
            Trigger::Press(Key::Char('s'), Modifiers::CTRL_SHIFT),
            "^SHIFT+S",
        ),
        (
            Trigger::Press(Key::Up, Modifiers::ALT_SHIFT),
            "ALT+SHIFT+UP",
        ),
        (Trigger::Press(Key::Delete, Modifiers::CTRL), "^DEL"),
        (Trigger::Press(Key::Insert, Modifiers::NONE), "INS"),
        (Trigger::Press(Key::F(9), Modifiers::NONE), "F9"),
        (Trigger::Press(Key::Char('['), Modifiers::NONE), "["),
        (
            Trigger::Chord {
                held: Key::Char('z'),
                key: Key::Down,
            },
            "Z+DOWN",
        ),
        (Trigger::DoubleTap(Key::Char('z')), "DOUBLE-TAP Z"),
    ];

    for (trigger, name) in names {
        assert_eq!(trigger.to_string(), name, "{trigger:?}");
    }
}

#[test]
fn actions_are_named_in_the_glossarys_words() {
    let names = [
        (Action::Cancel, "CANCEL"),
        (Action::ToggleCart, "CART"),
        (Action::ToggleLoopStart, "LOOP START"),
        (Action::DeepClone, "DEEP CLONE"),
        (Action::Move(Direction::Up), "UP"),
        (Action::Mute { track_number: 2 }, "MUTE TRACK 2"),
    ];

    for (action, name) in names {
        assert_eq!(action.name(), name, "{action:?}");
    }
}

#[test]
fn help_is_ctrl_h_in_pc_tracker_and_x_plus_h_in_m8_style() {
    assert_eq!(
        key_names(&Keymap::pc_tracker(), Action::ToggleHelp, NAVIGATION),
        ["^H"]
    );
    assert_eq!(
        key_names(&Keymap::m8_style(), Action::ToggleHelp, NAVIGATION),
        ["X+H"]
    );
}

#[test]
fn help_is_named_in_config_ron_and_can_be_rebound() {
    assert!(
        pikutracker_core::keymap::named_actions()
            .contains(&("ToggleHelp".to_owned(), Action::ToggleHelp))
    );
    let keymap = rebound(&[("ToggleHelp", "F12")]);

    assert_eq!(key_names(&keymap, Action::ToggleHelp, NAVIGATION), ["F12"]);
}
