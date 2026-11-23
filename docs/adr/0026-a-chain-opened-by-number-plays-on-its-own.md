# A Chain opened by number plays on its own

The Chain and Phrase panes each keep their own number, so a Chain can be on show without sitting in the Song cell under the Song cursor. Play in the Chain pane then has no Song row to take its Chain row timing from. We chose to play such a Chain on its own, on one voice: each entry lasts as long as its own Phrase, by the same rules a Chain row uses, and after the last entry it starts over, as Chain-pane playback already does. When the Chain on show is the one in the Song cell under the Song cursor, Play keeps the lockstep timing of its Song row ([ADR 0005](0005-chain-rows-play-in-lockstep.md)). We chose this over borrowing the Song row under the cursor, whose other Tracks have nothing to do with the Chain on show, and over refusing to play, which would defeat writing bottom-up.

## Consequences

- A Chain heard on its own can sound shorter than it will in a Song row, where a longer Phrase on another Track sets the Chain row's length.
- The Playhead reports the Chain playing on its own and its entry, so the Chain pane marks that entry and Follow moves the Chain cursor to it.
- Mute and Solo don't apply, as with any playback outside the Song.
