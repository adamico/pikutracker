# What counts as an edit

Only a change the user makes to the Project is an edit: it marks the Project changed and becomes an undo entry. Anything the app does on its own, or that changes nothing, leaves the Project untouched. We chose this so that undo walks back only what the user did, and opening a view or exporting never creates work to save.

## Consequences

- Undo leaves out the Claimed-bytes hash, because Export sets it and Export is not an edit. Undo or redo that restores a Target Cart with the same path keeps the hash Export last stored.
- Opening the Instrument view never defines a Custom Instrument. A new SFX Instrument exists only from its first real edit.
- A Phrase keeps all 32 Steps whatever its LEN, and the `.ptrk` file writes the Steps past LEN too, so shortening LEN and lengthening it again loses nothing.
- Change Speed and Scale Speed decide their scope (which Phrases, and how many are used elsewhere) when their question opens, so nothing the Playhead does while the user types can change it.
- Opening or importing another Project stops playback, and Mute and Solo do not carry over.
