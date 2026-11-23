# A Project holds at most 8 Custom Instruments, each tied to its SFX slot

A Step's instrument is one hex digit: 0–7 are the Built-in Instruments and 8–F are the Custom Instruments. Custom Instrument 8 + k is always Exported to SFX k, as PICO-8 plays it. We chose this over unlimited Custom Instruments with two-digit numbers, each given a free claimed slot in SFX 0–7 at Export ([ADR 0004](0004-custom-instruments-assigned-at-export.md)), for three reasons. A Cart can only play 8, so any more could never be Exported. A fixed slot means the digit in the tracker is the digit PICO-8 shows, and a Cart Import → Export round trip keeps every instrument where it was. And one digit frees a column in the Phrase pane for the Pen.

## Consequences

- Export fails when the Claim doesn't hold SFX k for a Custom Instrument 8 + k in use, and the error names both. A New Cart claims every slot, so this only happens with a game Cart whose low SFX hold its sound effects.
- Export no longer rewrites instrument digits or picks slots for Custom Instruments. The Phrases' SFX still fill the remaining claimed slots in order of first use.
- A Step that plays a Custom Instrument the Project doesn't define still needs its slot, which Export writes as an empty SFX, as [ADR 0004](0004-custom-instruments-assigned-at-export.md) decided.
- Cart Import turns SFX k, used as an instrument, into Custom Instrument 8 + k.
- Saved Projects with two-digit instruments are not migrated. The app is unpublished and no such Project exists.
