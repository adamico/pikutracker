# pikutracker

A music tracker for composing PICO-8 music with a Song → Chain → Phrase workflow modelled on the Dirtywave M8, exporting to PICO-8 carts.

## Language

### Our side

**Project**:
The tracker's own saved work: the full Song, its Chains, Phrases and instruments. Richer than what a Cart can hold.
_Avoid_: Song file, module

**Song**:
The top-level arrangement of a Project: rows of Chains, one per Track.
_Avoid_: Arrangement, sequence

**Song row**:
One row of the Song: a Chain, or nothing, for each Track. Its Chains play together, one Chain row at a time.
_Avoid_: Song position, Song line

**Loop start**:
A mark on a Song row: the row that a later Loop back returns to. It becomes PICO-8's flag of the same name on the first Pattern of that row.
_Avoid_: Song loop row, loop point

**Loop back**:
A mark on a Song row that makes playback return, after that row, to the nearest Loop start at or before it, or to Song row 00 if there is none. It becomes PICO-8's flag of the same name on the last Pattern of that row.
_Avoid_: Loop end

**Stop**:
A mark on a Song row: playback stops after that row, even when the row also has a Loop back. It becomes PICO-8's flag of the same name on the last Pattern of that row.

**Track**:
One of the Song's 4 vertical lanes, played by one voice. Track 1 plays on PICO-8's first channel, Track 2 on the second, and so on.
_Avoid_: Channel (that is PICO-8's word)

**Chain**:
An ordered list of entries, each either a Phrase (with an optional transpose) or empty. An empty entry leaves its Track silent for that Chain row.
_Avoid_: Block, sequence

**Chain row**:
The k-th entry of every Track's Chain within one Song row. They play together, and the Chain row lasts as long as its longest Phrase without a Phrase loop. If every Phrase in it loops, the longest plays once through and sets the length. Exported as one Pattern.
_Avoid_: Position, bar

**Phrase**:
32 Steps, of which the first 1 to 32 play (its length, 16 by default); the unit where notes are written. Steps past the length keep their notes and are shown, but don't play.
_Avoid_: Pattern (that is PICO-8's word), SFX

**Phrase loop**:
The Steps from a chosen Step to the end of a Phrase's length, repeated until the Chain row ends. Steps before it play once as a lead-in.
_Avoid_: Fill, loop start (that marks a Song row), loop point

**Loose Phrase**:
A Phrase that holds something but that no Chain uses. Cart Import keeps a Cart's other SFX as loose Phrases, outside the Claim.
_Avoid_: Orphan Phrase

**Step**:
One row of a Phrase, equivalent to one PICO-8 note: a pitch with its instrument, volume and effect. A Step at volume 0 is silent and is shown as empty; a note is held by repeating it in the following Steps.
_Avoid_: Row, note (when meaning the whole row)

**Pen**:
The octave, instrument, volume and effect that a newly entered note takes, overwriting all four values of its Step. Typing an instrument, volume or effect into a Step also sets the Pen. It belongs to the user's session, not to the Project.
_Avoid_: Defaults, current instrument

**Instrument**:
What a Step plays: one of the 8 Built-in Instruments or one of the Project's Custom Instruments.
_Avoid_: Waveform (ambiguous), patch

**Built-in Instrument**:
One of PICO-8's 8 fixed sounds (triangle, tilted saw, saw, square, pulse, organ, noise, phaser). They have no settings.
_Avoid_: Waveform

**Custom Instrument**:
An Instrument defined in the Project, either an SFX Instrument or a Waveform Instrument. A Project holds at most 8, numbered 8 to F after the Built-in Instruments' 0 to 7, as a Cart does.
_Avoid_: Sample

**SFX Instrument**:
A Custom Instrument made of a short run of Steps with its own Speed, filters and loop, played relative to C2 by each note that uses it.
_Avoid_: Macro

**Waveform Instrument**:
A Custom Instrument made of one 64-sample looping wave, with its own DETUNE, REVERB and DAMPEN, and a Bass switch that plays it one octave lower.
_Avoid_: Sample, wavetable

**Speed**:
How many Ticks each Step of a Phrase lasts (1–255). Each Phrase has its own Speed; the tempo in BPM is only a readout of it.
_Avoid_: Tempo, BPM (when meaning the stored value)

**Change Speed**:
The command that gives every Phrase in its scope the same typed Speed. Its scope is the selected Song rows or Chain entries, or else the whole Song.

**Scale Speed**:
The command that multiplies the Speed of every Phrase in its scope by the typed Speed over the Speed of the Phrase on show, so Phrases at different Speeds keep their timing against each other. It has the same scope as Change Speed.
_Avoid_: Set Speed

**Tick**:
PICO-8's unit of sound time, about 1/120 of a second.
_Avoid_: Frame

**Clone**:
A new, unused copy of a Chain or Phrase. A cloned Chain still points to the same Phrases.
_Avoid_: Duplicate

**Deep clone**:
A Clone of a Chain that also gets its own copy of each distinct Phrase it uses.
_Avoid_: Full copy

**Playhead**:
The position currently being played. It is separate from the cursor, which is where the user is editing.
_Avoid_: Play cursor

**Selection**:
A rectangle of cells in the focused pane, from the cell where it was started to the cursor. Copy, Cut, nudges, moving rows, Clone and Change Speed act on it instead of the cell under the cursor. Changing pane clears it.
_Avoid_: Block, mark

**Multitrack**:
The view of the Song as it plays: each Track's Steps in time order with their transposes applied, spaced by Ticks so that Steps side by side start together. The cursor moves only from Step to Step.
_Avoid_: Pattern view

**Follow**:
A mode in which the Chain and Phrase panes track the Playhead. Moving the cursor by hand turns it off. The Song cursor and the Multitrack track the Playhead whenever the Song plays, with or without Follow.
_Avoid_: Auto-scroll

**Mute**:
Silencing a Track without changing timing. A muted Track can still set the length of a Chain row.
_Avoid_: Disable

**Solo**:
Hearing only the marked Tracks, whatever is muted. Several Tracks can be soloed at once, and timing does not change.

### PICO-8 side

**Cart**:
A PICO-8 cartridge file, either `.p8` text or `.p8.png` image.
_Avoid_: Cartridge file, ROM

**SFX**:
One of the Cart's 64 sound slots: up to 32 notes with a speed, loop/length and filter switches. Used for both sound effects and music.
_Avoid_: Sample, sound

**Pattern**:
One of the Cart's 64 music entries: up to 4 SFX played together on PICO-8's channels, plus loop-start, loop-back and stop flags.
_Avoid_: Music (ambiguous), frame

### Between them

**Export**:
Compiling a Project into a Cart's SFX and Patterns. May lose information the Cart cannot hold.
_Avoid_: Save (saving writes the Project)

**Target Cart**:
The one Cart a Project Exports into, if any. A Project can have no Target Cart, for example after a MIDI Import.
_Avoid_: Linked cart, output cart

**New Cart**:
A `.p8` Cart the tracker made itself to be a Project's Target Cart, rather than one the user picked. Once made, it is an ordinary Target Cart.
_Avoid_: Blank cart, empty cart

**Claim**:
The SFX slots and the range of Patterns in the Project's Target Cart that the Project owns. The SFX part is any set of slots; the Pattern part is one contiguous range. Export writes only inside the Claim.
_Avoid_: Reservation, allocation

**Claimed-bytes hash**:
A fingerprint of the Target Cart's bytes inside the Claim, taken at Cart Import, when a New Cart is made, and after each Export. When the Cart no longer matches it, those bytes were changed outside the tracker.
_Avoid_: Checksum

**Import**:
Reconstructing a Project from outside material: a Cart's SFX and Patterns (Cart Import) or a Standard MIDI File (MIDI Import).
_Avoid_: Load (loading reads a Project)
