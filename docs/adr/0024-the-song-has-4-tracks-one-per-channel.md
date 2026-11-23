# The Song has 4 Tracks, one per PICO-8 channel

The Song has exactly 4 Tracks, and Track n always plays on PICO-8's channel n − 1. We chose this over 8 Tracks of which Export picks 4 ([ADR 0001](0001-project-is-source-of-truth.md)) because the Song now mirrors a Cart's Pattern list ([ADR 0023](0023-the-song-mirrors-the-carts-pattern-list.md)), and with 8 Tracks the tracker could play music that the Exported Cart would not. Choosing the Export Tracks was also the only everyday use of the Overview, and dropping it frees that room for the Multitrack.

## Consequences

- Export Tracks are gone, and so is the Overview that set them. What plays in the tracker is what Export writes.
- Mute and Solo only change what the tracker plays, never what Export writes.
- The Song pane is narrower, which leaves room for the Chain pane under it.
- MIDI Import gives Tracks 1–4 to the first 4 ticked voices, in order, and drops the rest. Its dialog warns when more than 4 are ticked.
- Saved Projects with 8 Tracks are not migrated. The app is unpublished and no such Project exists.
