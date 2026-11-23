# The Song mirrors the Cart's Pattern list

A Project has one Song, and it holds all of a Cart's music the way PICO-8 does: one flat list of Song rows, where a row can carry a Loop start, a Loop back or a Stop. The Project doesn't name the songs inside it. A game starts each one with `music(n)`, and the marks decide where it loops or ends. We chose this over one Project per song, picked at Cart Import, because composers write a Cart's music as sections of one Pattern list, and the tracker should show it the same way. The Song picker goes.

## Consequences

- Cart Import brings in every Pattern from the first non-empty one to the last, one Song row each ([ADR 0006](0006-import-rebuilds-what-plays.md)), and the Claim's Pattern range spans them. Each flow flag becomes the mark of the same name. Every SFX those Patterns play is claimed, and every other non-empty SFX becomes a loose Phrase. This replaces choosing a song by its flow flags ([ADR 0016](0016-cart-import-finds-songs-by-flow-flags.md)) and claiming only the chosen song's SFX ([ADR 0006](0006-import-rebuilds-what-plays.md)).
- Loop back returns to the nearest Loop start at or before it, or to Song row 00 if there is none, as PICO-8 does with Pattern 0.
- Loop start can share a row with either of the others, and with Loop back it makes a one-row loop. Loop back and Stop exclude each other, because a row can't both repeat and end. Setting one clears the other.
- Export puts the Loop start flag on the first Pattern of its Song row, and the Loop back and Stop flags on the last. A Song row with several Chain rows takes several Patterns, so Song row n need not be Pattern n.
- Export writes every Song row from 00 to the last one that plays. An empty Song row between them becomes one Pattern with every channel disabled, carrying its marks, so the Cart ends there as playback does. Marks after the last Song row that plays are not exported. Flags come only from marks: with no Stop mark, the last Pattern gets no Stop flag, and the Cart ends at the next empty Pattern.
- When a Loop back has no Loop start at or before it and the Song's first Pattern is not Pattern 0, Export also puts the Loop start flag on that first Pattern. PICO-8 would otherwise return to Pattern 0, outside the Song.
- The tracker neither reads nor writes `music(n)` calls. When an Export moves Patterns, the user updates the game's code.
- Play starts at the cursor's Song row and follows the marks as PICO-8 would.
- The Multitrack lays out the Song rows from 00 to the last one that plays, in row order as Export does, and shows the marks in its gutter rather than following them. An empty Song row lasts no Ticks, so it takes no rows there.
- Insert row and Delete row shift the marks with the rows, and moving rows leaves them where they are, as [ADR 0021](0021-moving-rows-leaves-the-song-loop-row.md) decided for the Song loop row, which they replace.
