# Cart bytes are read and written exactly as PICO-8 stores them

The Cart codec touches only the bytes it understands and writes them back in PICO-8's own layout, so that a Cart re-saved by PICO-8 after Export is unchanged and nothing outside the Claim moves.

## Consequences

- **`.p8.png`.** Cart bytes sit in the low 2 bits of each pixel's channels, alpha included. Pixels are read exactly as stored, and a write leaves the high bits (the picture) and the compressed code alone.
- **Rows PICO-8 leaves out.** PICO-8 omits trailing default rows from `__sfx__` and `__music__`. An omitted SFX row reads as Speed 16 with no notes, and an omitted Pattern as all channels disabled. The writer adds rows only up to the last claimed row that holds something.
- **Claimed-bytes hash.** One FNV-1a checksum per claimed slot, written `S03=xxxxxxxx` or `P18=xxxxxxxx`, so a later comparison can say which SFX and Patterns changed.
- **LOOP START.** Export always sets the LOOP START flag on the loop target, even at Pattern 0, because without it PICO-8 loops back to Pattern 0 while the Claim may start elsewhere. Stale flags in claimed Patterns are cleared.
- **Blank Steps.** Export blanks Steps past LEN, apart from pitches an arpeggio reads (ADR 0008), so that more SFX can share a slot.
- **New Cart.** A New Cart is written with only PICO-8's header (`version 43`) and an empty `__lua__` section, and no playback stub, because code outside the Claim would go stale. It is written to disk as soon as it is named, so that a missing Target Cart still means only that it moved.
- **Suggested Claim.** It takes the lowest free SFX, to leave the most room in 00–07 for Custom Instruments, and the lowest run of free Patterns long enough for the Song.
