# Cart Import finds songs by their flow flags
_Partly superseded by [ADR 0023](0023-the-song-mirrors-the-carts-pattern-list.md): Cart Import no longer chooses a song. The flow flags become Song row marks, and Loop back follows the same rule._

A Cart holds no list of songs, so Cart Import reads them from the Patterns' flow flags. A song starts at Pattern 0, or after a STOP, a LOOP BACK or an empty Pattern, and it runs until a STOP, a LOOP BACK or an empty Pattern. LOOP BACK jumps to the nearest LOOP START at or before it, or to Pattern 0; a jump outside the song means the song does not loop.

## Consequences

- A note marked as an SFX Instrument inside an SFX Instrument is Imported as a Built-in Step, because PICO-8 plays it as its Built-in Instrument (ADR 0012).
- An arpeggio's timing follows its place in the SFX. When Import moves an arpeggio, it keeps its sound only if all 4 pitches of its group are equal, and otherwise it warns.
- A `custom` note plays a Waveform Instrument when bit 7 of byte 66 of the SFX is set, with Bass in bit 0 of byte 65; otherwise it plays the SFX itself.
