# Key hints name the active keys, and the idle status line names none

Every key a hint names is looked up in the active keymap, after the Preset and `config.ron` are applied. Hints are the answers to a question, the keys of a list or dialog, the Log pointer and the Cart view's checks. We had fixed text such as `^T EDIT CLAIM`. That text named the wrong key under the M8Style Preset (X+T) and never showed a key rebound in `config.ron`. The idle status line used to list global shortcuts. Now it shows only the Project name, with `*` for unsaved changes, because that list was noise. The Help view (#99) is the way to find keys.

A hint shows one key per Action. When an Action has several keys, the hint names the one bound in the narrowest scope. Within a scope, a plain press comes before a press with modifiers, then a chord, then a double-tap. So M8Style's Next pane shows as TAB rather than SHIFT+RIGHT. A key that a narrower scope gives to another Action doesn't count, and neither does a key that is typed into a name. An Action with no key left in the context drops out of the hint.

## Consequences

- Hints and the Help view share one lookup and one set of Action names, so a rebound key shows everywhere at once.
- A hint is worded once, as an Action and a label. Its key text is never written by hand.
- The Log pointer and the EDIT CLAIM hint are built when their message is reported. A Preset change applies to messages reported after it.
