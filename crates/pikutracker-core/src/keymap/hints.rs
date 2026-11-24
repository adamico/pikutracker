use super::Keymap;
use crate::app::{Action, KeyContext};
use std::borrow::Cow;

const KEY_GAP: &str = "/";
pub const HINT_GAP: &str = "  ";

#[derive(Clone, Copy, Debug)]
pub struct KeyHint<'a> {
    actions: &'a [Action],
    label: Option<&'a str>,
}

impl<'a> KeyHint<'a> {
    pub const fn named(action: &'a Action) -> Self {
        Self {
            actions: std::slice::from_ref(action),
            label: None,
        }
    }

    pub const fn labelled(actions: &'a [Action], label: &'a str) -> Self {
        Self {
            actions,
            label: Some(label),
        }
    }

    fn label(&self) -> Cow<'a, str> {
        match (self.label, self.actions.first()) {
            (Some(label), _) => label.into(),
            (None, Some(action)) => action.name(),
            (None, None) => "".into(),
        }
    }
}

impl Keymap {
    pub fn hint_line(&self, context: KeyContext, hints: &[KeyHint<'_>]) -> String {
        hints
            .iter()
            .filter_map(|hint| {
                let keys: Vec<String> = hint
                    .actions
                    .iter()
                    .filter_map(|&action| self.triggers_for(action, context).first().copied())
                    .map(|trigger| trigger.to_string())
                    .collect();
                (!keys.is_empty()).then(|| format!("{} {}", keys.join(KEY_GAP), hint.label()))
            })
            .collect::<Vec<_>>()
            .join(HINT_GAP)
    }
}
