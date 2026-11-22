# Change Speed sets one Speed, Scale Speed keeps the ratios

Change Speed gives every Phrase in its scope the Speed the user types. Scale Speed multiplies each Phrase's Speed by the typed Speed over the Speed of the Phrase on show. Both have the same scope (the selected Song rows or Chain entries, or else the whole Song), and their question names it.

Change Speed used to scale. The owner found it surprising: a command named for changing a Speed should set it. Scaling is still needed, because Phrases in one Chain row can have different Speeds on purpose ([ADR 0003](0003-speed-in-ticks-per-phrase.md)), and Chain rows are timed in Ticks ([ADR 0005](0005-chain-rows-play-in-lockstep.md)). Setting double-time drums and a melody to one Speed changes the rhythm, not just the tempo. So both commands exist, and the familiar key goes to the one that sets.

## Consequences

- Change Speed keeps Ctrl+Shift+T in both presets. Scale Speed is Alt+T in the PcTracker preset and Z+T in the M8-style preset.
- Only Scale Speed can round, so only its report counts inexact Phrases.
