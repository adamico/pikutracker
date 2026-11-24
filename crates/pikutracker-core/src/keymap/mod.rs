mod hints;
mod key;
mod m8_style;
mod names;
mod pc_tracker;
mod shared;
mod trigger;

pub use hints::{HINT_GAP, KeyHint};
pub use key::{Key, KeyPress, Modifiers};
pub use names::named_actions;
pub use trigger::{Scope, Trigger};

use crate::app::{Action, KeyContext};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

pub const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub scope: Scope,
    pub trigger: Trigger,
    pub action: Action,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Preset {
    #[default]
    PcTracker,
    M8Style,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RebindProblem {
    UnknownAction(String),
    UnreadableKey { action: String, key: String },
}

#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: HashMap<(Scope, Trigger), Action>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::preset(Preset::default())
    }
}

impl Keymap {
    pub fn preset(preset: Preset) -> Self {
        let bindings = match preset {
            Preset::PcTracker => pc_tracker::bindings(),
            Preset::M8Style => m8_style::bindings(),
        };
        Self {
            bindings: bindings
                .into_iter()
                .map(|binding| ((binding.scope, binding.trigger), binding.action))
                .collect(),
        }
    }

    pub fn pc_tracker() -> Self {
        Self::preset(Preset::PcTracker)
    }

    pub fn m8_style() -> Self {
        Self::preset(Preset::M8Style)
    }

    pub fn rebound(preset: Preset, keys: &BTreeMap<String, String>) -> (Self, Vec<RebindProblem>) {
        let mut keymap = Self::preset(preset);
        let mut problems = Vec::new();
        for (name, text) in keys {
            let Some(action) = names::action_named(name) else {
                problems.push(RebindProblem::UnknownAction(name.clone()));
                continue;
            };
            let Some(trigger) = Trigger::parse(text) else {
                problems.push(RebindProblem::UnreadableKey {
                    action: name.clone(),
                    key: text.clone(),
                });
                continue;
            };
            keymap.rebind(action, trigger);
        }
        (keymap, problems)
    }

    fn rebind(&mut self, action: Action, trigger: Trigger) {
        let scopes: Vec<Scope> = self
            .bindings
            .iter()
            .filter(|(_, bound)| **bound == action)
            .map(|((scope, _), _)| *scope)
            .collect();
        let widest = scopes.iter().map(|scope| scope.width()).max();
        let replaced: Vec<Scope> = match widest {
            Some(widest) => scopes
                .into_iter()
                .filter(|scope| scope.width() == widest)
                .collect(),
            None => vec![Scope::Anywhere],
        };
        self.bindings
            .retain(|(scope, _), bound| !(*bound == action && replaced.contains(scope)));
        for scope in replaced {
            self.bindings.insert((scope, trigger), action);
        }
    }

    pub fn bindings(&self) -> Vec<Binding> {
        self.bindings
            .iter()
            .map(|(&(scope, trigger), &action)| Binding {
                scope,
                trigger,
                action,
            })
            .collect()
    }

    pub fn triggers_for(&self, action: Action, context: KeyContext) -> Vec<Trigger> {
        self.triggers_when(action, context, false)
    }

    pub fn triggers_when(
        &self,
        action: Action,
        context: KeyContext,
        selecting: bool,
    ) -> Vec<Trigger> {
        let scopes = scopes(context, selecting);
        let mut found: Vec<(Scope, Trigger)> = self
            .bindings
            .iter()
            .filter(|&(&(scope, trigger), &bound)| {
                bound == action
                    && scopes.contains(&scope)
                    && self.first_bound(trigger, &scopes) == Some(action)
                    && !(typing_a_name(context) && typed_by(trigger))
            })
            .map(|(&key, _)| key)
            .collect();
        found.sort_by_key(|&(scope, trigger)| (scope.width(), trigger.rank(), trigger.to_string()));
        found.into_iter().map(|(_, trigger)| trigger).collect()
    }

    fn first_bound(&self, trigger: Trigger, scopes: &[Scope]) -> Option<Action> {
        scopes
            .iter()
            .find_map(|&scope| self.bindings.get(&(scope, trigger)).copied())
    }

    pub fn action_for(&self, press: KeyPress) -> Option<Action> {
        let (_, action) = self.lookup(press, &[Scope::Anywhere], &[], false)?;
        repeats_or_new(press, action)
    }

    pub fn action_in(&self, press: KeyPress, context: KeyContext) -> Option<Action> {
        if typing_a_name(context)
            && let Some(action) = name_action(press)
        {
            return Some(action);
        }
        let (_, action) = self.lookup(press, &scopes(context, false), &[], false)?;
        repeats_or_new(press, action)
    }

    fn lookup(
        &self,
        press: KeyPress,
        scopes: &[Scope],
        held: &[Key],
        double: bool,
    ) -> Option<(Trigger, Action)> {
        let key = press.key.with_letter_lowercased();
        let mut triggers = Vec::new();
        if double {
            triggers.push(Trigger::DoubleTap(key));
        }
        if press.modifiers == Modifiers::NONE {
            triggers.extend(held.iter().rev().map(|&held| Trigger::Chord { held, key }));
        }
        triggers.push(Trigger::Press(key, press.modifiers));
        triggers.into_iter().find_map(|trigger| {
            scopes
                .iter()
                .find_map(|&scope| self.bindings.get(&(scope, trigger)))
                .map(|&action| (trigger, action))
        })
    }

    fn starts_chords(&self, key: Key, scopes: &[Scope]) -> bool {
        self.bindings.keys().any(|(scope, trigger)| {
            scopes.contains(scope) && matches!(trigger, Trigger::Chord { held, .. } if *held == key)
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Keyboard {
    keymap: Keymap,
    held: Vec<Key>,
    last_tap: Option<(Key, Duration)>,
    on_release: Option<(Key, Action)>,
}

impl Keyboard {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            ..Self::default()
        }
    }

    pub fn key_down(
        &mut self,
        press: KeyPress,
        context: KeyContext,
        selecting: bool,
        at: Duration,
    ) -> Option<Action> {
        if typing_a_name(context)
            && let Some(action) = name_action(press)
        {
            return Some(action);
        }
        let key = press.key.with_letter_lowercased();
        let mut double = false;
        if !press.repeat {
            let alone = press.modifiers == Modifiers::NONE;
            double = alone
                && self.last_tap.is_some_and(|(tapped, when)| {
                    tapped == key && at.saturating_sub(when) <= DOUBLE_TAP_WINDOW
                });
            self.last_tap = (alone && !double).then_some((key, at));
            self.on_release = None;
            if !self.held.contains(&key) {
                self.held.push(key);
            }
        }
        let scopes = scopes(context, selecting);
        let others: Vec<Key> = self.held.iter().copied().filter(|&k| k != key).collect();
        let (trigger, action) = self.keymap.lookup(press, &scopes, &others, double)?;
        let waits_for_chord = !press.repeat
            && matches!(trigger, Trigger::Press(_, Modifiers::NONE))
            && self.keymap.starts_chords(key, &scopes);
        if waits_for_chord {
            self.on_release = Some((key, action));
            return None;
        }
        repeats_or_new(press, action)
    }

    pub const fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn key_up(&mut self, key: Key) -> Option<Action> {
        let key = key.with_letter_lowercased();
        self.held.retain(|&held| held != key);
        self.on_release
            .take_if(|(waiting, _)| *waiting == key)
            .map(|(_, action)| action)
    }
}

fn scopes(context: KeyContext, selecting: bool) -> Vec<Scope> {
    let mut scopes = vec![Scope::Context(context)];
    if selecting {
        scopes.push(Scope::Selecting);
    }
    scopes.push(Scope::Anywhere);
    scopes
}

fn repeats_or_new(press: KeyPress, action: Action) -> Option<Action> {
    (!press.repeat || repeats_while_held(action)).then_some(action)
}

const fn repeats_while_held(action: Action) -> bool {
    matches!(
        action,
        Action::Move(_) | Action::Select(_) | Action::Nudge { .. }
    )
}

const fn typing_a_name(context: KeyContext) -> bool {
    matches!(context, KeyContext::FileName | KeyContext::InstrumentName)
}

fn typed_by(trigger: Trigger) -> bool {
    match trigger {
        Trigger::Press(key, modifiers) => name_action(KeyPress::new(key, modifiers)).is_some(),
        _ => false,
    }
}

fn name_action(press: KeyPress) -> Option<Action> {
    if press.modifiers.ctrl || press.modifiers.alt {
        return None;
    }
    match press.key {
        Key::Char(ch) => Some(Action::TypeChar(ch)),
        Key::Backspace => Some(Action::Backspace),
        _ => None,
    }
}
