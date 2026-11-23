# The Song cursor and the Multitrack always track Song playback

While the Song plays, the Song cursor and the Multitrack always move with the Playhead, whether Follow is on or off. Follow now only decides whether the Chain and Phrase panes move too. We had all panes behind Follow, but then the Multitrack, which exists to show the Song as it plays, sat still by default, and the Song cursor gave no sense of where playback was. The Chain and Phrase panes stay behind Follow, so a Phrase can be edited while the Song plays without the pane switching to whatever is playing.

Outside playback, moving the Song cursor scrolls the Multitrack to that Song row, so the two always show the same place. The Multitrack cursor already drives the Song cursor, so either one can be used to find a place in the Song.

## Consequences

- While the Song plays, a Song cursor moved by hand goes back to the Playhead's Song row at the next Step. Editing the Song is easiest with playback stopped.
- With Follow off, the Chain and Phrase on show stay put while the Song cursor moves with the Playhead. They change only when a cursor is moved by hand.
- Playing a Phrase or a Chain on its own moves neither the Song cursor nor the Multitrack, because it isn't Song playback.
