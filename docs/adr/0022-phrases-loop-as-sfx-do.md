# Phrases loop as SFX do

A Phrase holds a loop start and a loop end, read exactly as PICO-8 reads an SFX's. When the end is 0, the start is the Phrase's length and nothing loops. When the start is below the end, the Steps from start to end − 1 form the **Phrase loop** and the end is the length. Any other pair plays all 32 Steps. All 32 Steps keep their notes, and the Phrase pane shows the Steps past the length dimmed and still editable. This replaces Fill, the mark on a Chain entry that repeated its whole Phrase ([ADR 0005](0005-chain-rows-play-in-lockstep.md)).

Fill left out something PICO-8 has: a lead-in before a loop. Writing one took two Phrases in two Chain rows and another Track to set the length. Cart Import had to unroll such loops and lost the lead-in when the result didn't fit in 32 Steps. Shortening a Phrase also hid its later notes, though they were still saved. Putting the loop on the Phrase makes Phrase and SFX match one-to-one, so Export and Cart Import stop working around the difference. The cost: a Phrase now loops the same way in every Chain that uses it. To loop it in one place and not another, Clone it.

## Consequences

- The Chain row timing of ADR 0005 stays the same, with "Phrase with a Phrase loop" in place of "entry marked Fill". A looping Phrase never sets its Chain row's length. When every Phrase in a Chain row loops, the longest plays once through and sets the length.
- Cart Import copies an SFX's loop into the Phrase loop. It no longer unrolls loops or warns about a lost lead-in, and Loose Phrases keep their loop. This replaces the loop rules of [ADR 0006](0006-import-rebuilds-what-plays.md).
- Projects saved with Fill aren't converted. There are no Projects that need to be kept.
