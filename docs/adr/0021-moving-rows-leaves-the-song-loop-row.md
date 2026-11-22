# Moving rows leaves the Song loop row where it is

While a selection is held, Move Rows Up / Down moves the selected Steps, Chain entries or Song cells one row, and the row they pass swaps to the other side. In the Song pane only the selected Tracks' cells move. The Song loop row keeps its row number.

Insert row and Delete row shift the Song loop row, because they shift every Track and so the music the loop marked moves as a whole. Moving rows can move some Tracks and leave others, so the Song row the loop marked is no longer one piece of music that could be followed. The owner chose to keep the Song loop row fixed rather than guess which Tracks it belongs to.

## Consequences

- Moving Song rows never changes where playback loops back to. To move the loop with the music, set it again with Toggle Song Loop Row.
- An edge press changes nothing and adds no undo entry, as [ADR 0017](0017-edits-and-their-undo.md) requires of every edit that changes nothing.
