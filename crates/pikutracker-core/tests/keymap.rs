use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};

const NONE: Modifiers = Modifiers::NONE;
const CTRL: Modifiers = Modifiers::CTRL;
const SHIFT: Modifiers = Modifiers::SHIFT;
const CTRL_SHIFT: Modifiers = Modifiers::CTRL_SHIFT;
const ALT: Modifiers = Modifiers::ALT;

fn action(key: Key, modifiers: Modifiers) -> Option<Action> {
    Keymap::pc_tracker().action_for(KeyPress::new(key, modifiers))
}

#[test]
fn arrows_and_tab_map_to_movement_and_focus() {
    let expected = [
        (Key::Up, NONE, Action::Move(Direction::Up)),
        (Key::Down, NONE, Action::Move(Direction::Down)),
        (Key::Left, NONE, Action::Move(Direction::Left)),
        (Key::Right, NONE, Action::Move(Direction::Right)),
        (Key::Tab, NONE, Action::FocusNext),
        (Key::Tab, SHIFT, Action::FocusPrevious),
    ];

    for (key, modifiers, want) in expected {
        assert_eq!(action(key, modifiers), Some(want), "{key:?} {modifiers:?}");
    }
}

#[test]
fn the_pc_tracker_preset_matches_the_keys_table() {
    let expected = [
        (Key::Char('n'), CTRL, Action::NewProject),
        (Key::Char('o'), CTRL, Action::OpenProject),
        (Key::Char('s'), CTRL, Action::Save),
        (Key::Char('s'), CTRL_SHIFT, Action::SaveAs),
        (Key::Char('i'), CTRL, Action::Import),
        (Key::Char('e'), CTRL, Action::Export),
        (Key::Char('n'), CTRL_SHIFT, Action::NewUnused),
        (Key::Char('b'), CTRL, Action::StartSelection),
        (Key::Char('c'), CTRL, Action::Copy),
        (Key::Char('x'), CTRL, Action::Cut),
        (Key::Char('v'), CTRL, Action::Paste),
        (Key::Char('d'), CTRL, Action::Clone),
        (Key::Char('d'), CTRL_SHIFT, Action::DeepClone),
        (Key::Char('z'), CTRL, Action::Undo),
        (Key::Char('y'), CTRL, Action::Redo),
        (Key::Insert, NONE, Action::InsertRow),
        (Key::Delete, CTRL, Action::DeleteRow),
        (Key::Char('t'), CTRL_SHIFT, Action::ChangeSpeed),
        (Key::Char('t'), ALT, Action::ScaleSpeed),
        (Key::Char('p'), CTRL_SHIFT, Action::PurgeUnused),
        (Key::Char('t'), CTRL, Action::ToggleCart),
        (Key::Char('l'), CTRL, Action::ToggleLog),
        (Key::Space, NONE, Action::PlayFocusedPane),
        (Key::Space, SHIFT, Action::PlaySongFromCursor),
        (Key::F(9), NONE, Action::ClearMutesAndSolos),
        (Key::F(10), NONE, Action::ToggleFollow),
    ];

    for (key, modifiers, want) in expected {
        assert_eq!(action(key, modifiers), Some(want), "{key:?} {modifiers:?}");
    }
}

#[test]
fn function_keys_mute_and_solo_tracks_one_to_eight() {
    for track in 1..=8 {
        assert_eq!(
            action(Key::F(track), NONE),
            Some(Action::Mute {
                track_number: track
            })
        );
        assert_eq!(
            action(Key::F(track), SHIFT),
            Some(Action::Solo {
                track_number: track
            })
        );
    }
}

#[test]
fn a_shifted_letter_finds_the_same_binding_as_its_lowercase_key() {
    assert_eq!(action(Key::Char('S'), CTRL_SHIFT), Some(Action::SaveAs));
}

#[test]
fn an_unbound_key_maps_to_no_action() {
    assert_eq!(action(Key::Char('g'), CTRL), None);
    assert_eq!(action(Key::Insert, Modifiers::ALT), None);
}

#[test]
fn only_movement_repeats_while_a_key_is_held() {
    let keymap = Keymap::pc_tracker();

    assert_eq!(
        keymap.action_for(KeyPress::repeated(Key::Down, NONE)),
        Some(Action::Move(Direction::Down))
    );
    assert_eq!(keymap.action_for(KeyPress::repeated(Key::Tab, NONE)), None);
    assert_eq!(
        keymap.action_for(KeyPress::repeated(Key::Space, NONE)),
        None
    );
}

#[test]
fn key_presses_drive_the_headless_app_through_the_preset() {
    let keymap = Keymap::pc_tracker();
    let mut app = App::new();

    for key in [Key::Tab, Key::Down] {
        let action = keymap.action_for(KeyPress::new(key, NONE)).unwrap();
        app.apply(action);
    }

    let dump = app.grid().text_dump();
    let chain_entry_one: String = dump
        .lines()
        .nth(3)
        .unwrap()
        .chars()
        .skip(29)
        .take(13)
        .collect();
    assert_eq!(chain_entry_one, "1[--]---     ");
}
