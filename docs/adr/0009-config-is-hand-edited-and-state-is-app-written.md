# The user edits config.ron and the app writes only state.ron

App settings live in two RON files with an integer format version. `config.ron`, under `$XDG_CONFIG_HOME/pikutracker/`, holds the active key preset and only the keys the user changed. The user edits it by hand, and the app reads it at launch and never writes it. `state.ron`, under `$XDG_STATE_HOME/pikutracker/`, holds everything the app writes: Follow, the Pen, changed MIDI table rows and the recent Projects. We first put Follow, the octave and the MIDI tables in `config.ron` too. We moved them so that the app never writes the file users edit, and the user's comments and edits survive while the app runs.

## Consequences

- There is no settings screen. Users switch presets and rebind keys by editing `config.ron`, starting from `config.example.ron`.
- `config.ron` stores only changed keys, so keys a later version adds reach existing users.
- A damaged file is renamed with a `.bad` suffix and the app starts on defaults and says so. A file from a newer version is left alone, and the app never writes it that session.
- A damaged `state.ron` is written again only once its rename to `.bad` has succeeded. A settings file that cannot be read is left alone like a newer one, and `state.ron` is not written that session.
- When the platform has no folder for a file, defaults apply and nothing is remembered that session.
