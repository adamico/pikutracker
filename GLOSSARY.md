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

**Song loop row**:
The Song row that playback jumps back to once the Song reaches its first empty row. A Song with no Song loop row stops there.
_Avoid_: Loop start (that is PICO-8's flag), loop point

**Track**:
One of the Song's 8 vertical lanes, played by one voice. Export picks 4 of them for PICO-8's channels.
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
_Avoid_: Fill, loop start (that is PICO-8's flag), loop point

**Loose Phrase**:
A Phrase that holds something but that no Chain uses. Cart Import keeps a Cart's other SFX as loose Phrases, outside the Claim.
_Avoid_: Orphan Phrase

**Step**:
One row of a Phrase, equivalent to one PICO-8 note: a pitch with its instrument, volume and effect. A Step at volume 0 is silent and is shown as empty; a note is held by repeating it in the following Steps.
_Avoid_: Row, note (when meaning the whole row)

**Instrument**:
What a Step plays: one of the 8 Built-in Instruments or one of the Project's Custom Instruments.
_Avoid_: Waveform (ambiguous), patch

**Built-in Instrument**:
One of PICO-8's 8 fixed sounds (triangle, tilted saw, saw, square, pulse, organ, noise, phaser). They have no settings.
_Avoid_: Waveform

**Custom Instrument**:
An Instrument defined in the Project, either an SFX Instrument or a Waveform Instrument. A Project can hold many; at most 8 fit in a Cart.
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

**Follow**:
A mode in which the panes track the Playhead. Moving the cursor by hand turns it off.
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

**Export Track**:
A Track assigned to one of PICO-8's 4 channels for Export. A Project has up to 4, and a Track can be on only one channel.
_Avoid_: Output track, exported channel

**Target Cart**:
The one Cart a Project Exports into, if any. A Project can have no Target Cart, for example after a MIDI Import.
_Avoid_: Linked cart, output cart

**Claim**:
The SFX slots and the range of Patterns in the Project's Target Cart that the Project owns. The SFX part is any set of slots; the Pattern part is one contiguous range. Export writes only inside the Claim.
_Avoid_: Reservation, allocation

**Claimed-bytes hash**:
A fingerprint of the Target Cart's bytes inside the Claim, taken at Cart Import and after each Export. When the Cart no longer matches it, those bytes were changed outside the tracker.
_Avoid_: Checksum

**Import**:
Reconstructing a Project from outside material: a Cart's SFX and Patterns (Cart Import) or a Standard MIDI File (MIDI Import).
_Avoid_: Load (loading reads a Project)
