use pikutracker_core::app::{Action, Direction, KeyContext};
use pikutracker_core::keymap::{
    Binding, Key, KeyPress, Keyboard, Keymap, Modifiers, Preset, Scope, Trigger, named_actions,
};
use std::time::Duration;

const PRESETS: [Preset; 2] = [Preset::PcTracker, Preset::M8Style];

fn press(keymap: &Keymap, binding: Binding) -> Vec<Action> {
    let mut keyboard = Keyboard::new(keymap.clone());
    let (context, selecting) = match binding.scope {
        Scope::Anywhere => (KeyContext::Navigation, false),
        Scope::Selecting => (KeyContext::Navigation, true),
        Scope::Context(context) => (context, false),
    };
    let down = |keyboard: &mut Keyboard, key, modifiers, at| {
        let press = KeyPress::new(key, modifiers);
        keyboard.key_down(press, context, selecting, Duration::from_millis(at))
    };
    let mut actions = Vec::new();
    match binding.trigger {
        Trigger::Press(key, modifiers) => {
            actions.extend(down(&mut keyboard, key, modifiers, 0));
            actions.extend(keyboard.key_up(key));
        }
        Trigger::Chord { held, key } => {
            actions.extend(down(&mut keyboard, held, Modifiers::NONE, 0));
            actions.extend(down(&mut keyboard, key, Modifiers::NONE, 10));
        }
        Trigger::DoubleTap(key) => {
            down(&mut keyboard, key, Modifiers::NONE, 0);
            keyboard.key_up(key);
            actions.extend(down(&mut keyboard, key, Modifiers::NONE, 100));
        }
    }
    actions
}

fn left_unbound(preset: Preset) -> Vec<Action> {
    match preset {
        Preset::PcTracker => vec![Action::StartSelection],
        Preset::M8Style => vec![
            Action::SelectJump(Direction::Up),
            Action::SelectJump(Direction::Down),
        ],
    }
}

fn typed_characters(preset: Preset) -> Vec<Action> {
    let semitones = match preset {
        Preset::PcTracker => 24,
        Preset::M8Style => 17,
    };
    let piano = (0..semitones).map(Action::PianoKey);
    let digits = (0..16).map(Action::HexDigit);
    piano.chain(digits).chain([Action::TypeChar('-')]).collect()
}

#[test]
fn every_binding_of_both_presets_reaches_its_action() {
    for preset in PRESETS {
        let keymap = Keymap::preset(preset);
        for binding in keymap.bindings() {
            assert_eq!(
                press(&keymap, binding),
                [binding.action],
                "{preset:?} {binding:?}"
            );
        }
    }
}

#[test]
fn both_presets_reach_every_named_action_and_typed_character_they_do_not_leave_unbound() {
    for preset in PRESETS {
        let bound: Vec<Action> = Keymap::preset(preset)
            .bindings()
            .iter()
            .map(|binding| binding.action)
            .collect();
        let unbound = left_unbound(preset);
        let named = named_actions().into_iter().map(|(_, action)| action);
        for action in named.chain(typed_characters(preset)) {
            assert_eq!(
                bound.contains(&action),
                !unbound.contains(&action),
                "{preset:?} {action:?}"
            );
        }
    }
}

#[test]
fn every_bound_action_is_named_or_typed() {
    let named: Vec<Action> = named_actions()
        .into_iter()
        .map(|(_, action)| action)
        .collect();
    for preset in PRESETS {
        for binding in Keymap::preset(preset).bindings() {
            let typed = matches!(
                binding.action,
                Action::PianoKey(_) | Action::HexDigit(_) | Action::TypeChar(_)
            );
            assert!(
                typed || named.contains(&binding.action),
                "{:?} has no name",
                binding.action
            );
        }
    }
}

#[test]
fn the_m8_style_preset_matches_the_keys_table() {
    let keymap = Keymap::m8_style();
    let z = Key::Char('z');
    let x = Key::Char('x');
    let anywhere = |trigger, action| Binding {
        scope: Scope::Anywhere,
        trigger,
        action,
    };
    let expected = [
        anywhere(Trigger::DoubleTap(z), Action::NewUnused),
        anywhere(Trigger::Press(x, Modifiers::SHIFT), Action::StartSelection),
        Binding {
            scope: Scope::Selecting,
            trigger: Trigger::Press(x, Modifiers::NONE),
            action: Action::Copy,
        },
        Binding {
            scope: Scope::Selecting,
            trigger: Trigger::Press(z, Modifiers::NONE),
            action: Action::Clone,
        },
        anywhere(Trigger::Chord { held: z, key: x }, Action::Cut),
        anywhere(Trigger::Press(z, Modifiers::SHIFT), Action::Paste),
        anywhere(
            Trigger::Chord {
                held: x,
                key: Key::Char('t'),
            },
            Action::ToggleCart,
        ),
        anywhere(
            Trigger::Chord {
                held: x,
                key: Key::Char('l'),
            },
            Action::ToggleLog,
        ),
        anywhere(
            Trigger::Chord {
                held: z,
                key: Key::Char('t'),
            },
            Action::ScaleSpeed,
        ),
        anywhere(
            Trigger::Press(Key::Char('d'), Modifiers::CTRL_SHIFT),
            Action::DeepClone,
        ),
        anywhere(
            Trigger::Press(Key::Char('s'), Modifiers::CTRL),
            Action::Save,
        ),
        anywhere(
            Trigger::Press(Key::Right, Modifiers::SHIFT),
            Action::FocusNext,
        ),
        anywhere(
            Trigger::Chord {
                held: z,
                key: Key::Up,
            },
            Action::Nudge {
                up: true,
                coarse: true,
            },
        ),
        Binding {
            scope: Scope::Selecting,
            trigger: Trigger::Press(Key::Down, Modifiers::NONE),
            action: Action::Select(Direction::Down),
        },
        Binding {
            scope: Scope::Selecting,
            trigger: Trigger::Chord {
                held: x,
                key: Key::Left,
            },
            action: Action::SelectJump(Direction::Left),
        },
    ];

    let bindings = keymap.bindings();
    for binding in expected {
        assert!(bindings.contains(&binding), "{binding:?}");
    }
}

#[test]
fn the_example_config_with_every_key_uncommented_keeps_the_pc_tracker_preset() {
    use pikutracker_core::settings::{SettingsRead, read_config};
    let example = include_str!("../../../config.example.ron");
    let uncommented: String = example
        .lines()
        .map(|line| match line.trim_start().strip_prefix("// \"") {
            Some(rest) => format!("\"{rest}\n"),
            None => format!("{line}\n"),
        })
        .collect();
    let SettingsRead::Loaded(config) = read_config(&uncommented) else {
        panic!("the example does not read: {uncommented}");
    };
    assert!(config.keys.len() > 20, "{:?}", config.keys);

    let (keymap, problems) = Keymap::rebound(config.preset, &config.keys);

    assert!(problems.is_empty(), "{problems:?}");
    let rebound = keymap.bindings();
    let preset = Keymap::pc_tracker().bindings();
    let missing: Vec<_> = preset.iter().filter(|b| !rebound.contains(b)).collect();
    let added: Vec<_> = rebound.iter().filter(|b| !preset.contains(b)).collect();
    assert_eq!((missing, added), (vec![], vec![]));
}

#[test]
fn x_with_the_arrows_jumps_in_the_m8_style_preset_and_alt_with_them_in_the_pc_tracker_preset() {
    let jumps = [
        (Key::Up, Direction::Up),
        (Key::Down, Direction::Down),
        (Key::Left, Direction::Left),
        (Key::Right, Direction::Right),
    ];
    let m8 = Keymap::m8_style().bindings();
    let pc = Keymap::pc_tracker().bindings();
    for (key, direction) in jumps {
        let action = Action::Jump(direction);
        let option = Trigger::Chord {
            held: Key::Char('x'),
            key,
        };
        let alt = Trigger::Press(key, Modifiers::ALT);
        for (bindings, trigger) in [(&m8, option), (&pc, alt)] {
            let binding = Binding {
                scope: Scope::Anywhere,
                trigger,
                action,
            };
            assert!(bindings.contains(&binding), "{binding:?}");
        }
    }
}
