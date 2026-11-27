# Overlays stack newest on top

Speed entry, a question, the MIDI Import dialog, the File list and the Instrument view each take every key while open. They stack, newest on top: the top one gets the key, names the Key context and the Help list, and closing it shows the one it opened over. Opening an overlay of a kind already open replaces that one. We used to keep one flag per overlay and decide which came first in five places, each in its own order. Quit closed every overlay before it asked about unsaved changes, while opening another Project closed only Speed entry and the Cart view. Overlays already opened over one another, such as the File list over the Instrument view and a question over the File list, so we chose to let the order they opened in decide, in one place.

## Consequences

- Quit no longer closes what is open. Its question goes on top, and Esc returns to the overlay, and any Selection, beneath it.
- While an overlay is on top, Esc goes to it. It clears a Selection only when no overlay is open.
- A question that arrives late, such as an Export's NOT FOUND, goes on top of whatever is open, Speed entry included. Over the MIDI Import dialog it takes the status line from the dialog's key hints.
- Opening another Project closes every overlay and the Cart view.
- The Log and the Cart view are not overlays: they fill the right-hand slot and leave the keys to the panes until focused. The Log covers the Cart view while both are open.
