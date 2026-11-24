# pikutracker

A keyboard-driven desktop tracker for writing PICO-8 music. You compose in a Project with more room than a Cart has, then Export the music into a PICO-8 Cart.

What pikutracker does:

- Song, Chain and Phrase panes on one screen, with 4 Tracks, one per PICO-8 channel.
- Export writes only the SFX and Patterns the Project claims, so the Cart's code, graphics, map and other sounds stay as they are.
- Import the music of a `.p8` or `.p8.png` Cart, or a Standard MIDI File.
- Playback emulates PICO-8's sound chip and is checked against recordings of real PICO-8.

## Install

Releases are for Linux x86_64 and need ALSA (`libasound`). Download the archive from [GitHub Releases](https://github.com/adamico/pikutracker/releases), then:

```sh
tar -xzf pikutracker-X.Y.Z-x86_64-linux.tar.gz
./pikutracker-X.Y.Z-x86_64-linux/pikutracker
```

To build from source, run `cargo run --release -p pikutracker`.

## How a Song is built

![The pikutracker screen. On the left, the Phrase pane shows the 8 Steps of Phrase 01, from A-2 down to D-2 and back up to G-2, under its settings and the NOTE, INST, VOL and FX column headers. In the middle, the Chain pane shows Chain 00, which lists Phrases 00, 00 and 01; below it, the Song pane: in row 00, Track 1 plays Chain 00 and Track 2 plays Chain 02; in row 01 they play Chains 01 and 03. On the right, the Multitrack shows the Song as it plays: Track 1's and Track 2's Steps in time order, with Song rows 00 and 01 numbered in its gutter.](docs/images/song-chain-phrase.png)

The screen has three sections: the Phrase pane on the left, the Chain pane over the Song pane in the middle, and the Multitrack on the right, where the file list, the Log and the Cart view open over it. The panes are:

- **Multitrack**: the Song as it plays. Its columns, TRACK 1 to TRACK 4, show each Track's Steps in time order with their transposes applied, spaced by Ticks so that Steps side by side start together; a Step that lasts longer than its neighbours has a tail (`|`). Phrase loops repeat until their Chain row ends. Song row numbers and marks run down its gutter, and its title shows the Song row in view. While the Song plays it tracks the Playhead, and moving the Song cursor scrolls it to that Song row. Focus it to browse with a cursor that moves from Step to Step: Up and Down go to a Track's previous or next Step, Left and Right to the Step sounding at that time on the next Track over. The Song, Chain and Phrase panes show where the cursor is, and Tab takes you to that Step in the Phrase pane to edit it.
- **Song**: the whole piece, read from top to bottom. Each Song row names one Chain for each Track, or none. A Song has up to 256 rows.
  - **Loop start, Loop back and Stop**: marks on a Song row, shown beside its number, which become PICO-8's flags of the same name. After a Loop back row (`<`), playback returns to the nearest Loop start (`>`) at or before it, or to row 00 if there is none. Playback ends after a Stop row (`|`), at an empty row, or after the last row. A row can carry any mix of the three: Loop start and Loop back show `=`, Loop start and Stop `/`, Loop back and Stop `[`, and all three `#`. When a row has both Loop back and Stop, Stop wins.
  - **Track**: one of the Song's 4 columns (T1 to T4), played by one voice. Track n Exports to PICO-8's channel n − 1.
- **Chain**: a list of up to 16 Phrases, played in order. The same Phrase can appear more than once, and each entry can transpose it.
  - **Chain row**: entry K of every Track's Chain in the same Song row. They start together and become one PICO-8 Pattern.
- **Phrase**: 1 to 32 Steps (16 by default) with its own speed. This is where you write notes. A Phrase becomes roughly one PICO-8 SFX; when a Chain transposes it, the transposed copy is exported as its own SFX.
  - **Phrase loop**: set with the LOOP start and end beside the speed, read exactly as PICO-8 reads an SFX's. With end 00, the start is the length and nothing loops. With start below end, Steps start to end − 1 repeat until the Chain row ends, after the Steps before them play once. Any other pair plays all 32 Steps. A `<` beside the row label marks where the loop starts.
  - **Step**: one row of a Phrase: a note, its instrument, volume and effect. It is the same as one note of a PICO-8 SFX.

In the screenshot, Song row 00 plays Chain 00 on Track 1 and Chain 02 on Track 2. Chain 00 plays Phrase 00 twice, then Phrase 01. The cursor is on that Phrase 01, so the Phrase pane shows its 8 Steps.

To write bottom-up, move the cursor onto the number in the Chain or Phrase pane's title and type another: the pane shows that Chain or Phrase without placing it anywhere. Moving the cursor in the Song or Chain pane shows what it points at again.

The workflow and the optional M8-style keys come from the Dirtywave M8; its [operation manual](https://dirtywave.com/pages/resources-downloads) explains them further. Some things work differently from the M8:

- The Song, Chain and Phrase panes are all on one screen.
- Each Chain row starts on every Track at once, because PICO-8 plays its 4 channels that way.
- There is no note-off or hold marker. To hold a note, repeat it on the next Steps, as in PICO-8.

## Getting started

When nothing else is shown, the status line at the bottom shows the Project's name, with `*` while it has unsaved changes. Questions, lists and dialogs on the status line name the keys that answer them, as bound under the active Preset and `config.ron`. Ctrl+S saves the Project as one `.ptrk` file. Ctrl+E Exports it into a Cart. The first Export opens the Cart view: press Enter on its Cart field, then pick a Cart or type a name to create a New Cart, an empty `.p8` file next to the Project.

## Default shortcuts

These are the keys of the default PcTracker preset. The M8-style column applies only when you switch presets under [Settings](#settings). "X+R" means hold X, then press R. Keys marked "same" are the same in both presets.

### Project

| Action | PcTracker | M8-style |
| --- | --- | --- |
| New Project | Ctrl+N | same |
| Open Project | Ctrl+O | same |
| Save | Ctrl+S | same |
| Save as | Ctrl+Shift+S | same |
| Import a Cart or MIDI file | Ctrl+I | same |
| Export into a Cart | Ctrl+E | same |
| Quit | Ctrl+Q | same |

### Moving around

| Action | PcTracker | M8-style |
| --- | --- | --- |
| Move the cursor | Arrows | same |
| Next / previous pane (Phrase, Chain, Song, Multitrack) | Tab / Shift+Tab | same, or Shift+Arrows |
| Jump 16 Song rows, or to the next filled Chain or Phrase | Alt+Arrows | X+Arrows |
| Open the instrument under the cursor (Phrase pane) | Enter | same |
| Cancel | Esc | same |

### Editing

| Action | PcTracker | M8-style |
| --- | --- | --- |
| Enter a note (Phrase pane, note column) | Z–M lower octave, Q–U upper octave | Q–P |
| Octave down / up | [ / ] | same |
| Type a value | 0–9, A–F | same |
| Change the value by 1 | Ctrl+Up / Ctrl+Down | Z+Right / Z+Left |
| Change the value by a larger step (an octave for notes) | Ctrl+Shift+Up / Ctrl+Shift+Down | Z+Up / Z+Down |
| Clear | Delete | same |
| Insert row | Insert | same |
| Delete row | Ctrl+Delete | same |
| Move the selected rows up / down | Alt+Up / Alt+Down (while selecting) | X+Up / X+Down (while selecting) |
| Undo / redo | Ctrl+Z / Ctrl+Y | same |
| Select | Shift+Arrows | X+Shift, then Arrows |
| Select by a jump | Shift+Alt+Arrows | X+Left / X+Right (while selecting) |
| Drop the selection | Esc, or an arrow without Shift | Esc |
| Copy | Ctrl+C | X (while selecting) |
| Cut | Ctrl+X | Z+X |
| Paste | Ctrl+V | Z+Shift |
| New unused Chain or Phrase under the cursor | Ctrl+Shift+N | Double-tap Z |
| Clone the Chain or Phrase under the cursor | Ctrl+D | Z (while selecting) |
| Clone Chains together with their Phrases | Ctrl+Shift+D | same |
| Set or clear Loop start on the Song row | Ctrl+R | X+R |
| Set or clear Loop back on the Song row | Ctrl+Shift+R | X+B |
| Set or clear Stop on the Song row | Ctrl+P | X+P |
| Set one Speed on every Phrase in the selection, or else the Song | Ctrl+Shift+T | same |
| Scale Speeds by a ratio, keeping Phrases in time | Alt+T | Z+T |
| Purge unused Chains and Phrases | Ctrl+Shift+P | same |

### Playback

| Action | PcTracker | M8-style |
| --- | --- | --- |
| Play / stop the focused pane | Space | same |
| Play the Song from the cursor | Shift+Space | same |
| Mute Track 1–4 | F1–F4 | same |
| Solo Track 1–4 | Shift+F1–F4 | same |
| Clear mutes and solos | F9 | same |
| Toggle Follow | F10 | same |

### Views

| Action | PcTracker | M8-style |
| --- | --- | --- |
| Show / hide the Cart | Ctrl+T | X+T |
| Show / hide the log | Ctrl+L | X+L |

## Settings

No settings file is needed. To use the M8-style key preset or rebind keys, copy `config.example.ron` to `~/.config/pikutracker/config.ron` (or `$XDG_CONFIG_HOME`) and edit the copy. The file explains each option.

pikutracker keeps Follow, the Pen (octave, instrument, volume and effect for new notes), MIDI table edits and your recent Projects in `~/.local/state/pikutracker/state.ron` (or `$XDG_STATE_HOME`).

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE); you may use it under either. Third-party notices are in `THIRD_PARTY_LICENSES`, and `pikutracker --licenses` prints them.

pikutracker is an independent project. It is not affiliated with or endorsed by Lexaloffle or Dirtywave, and it includes no PICO-8 code.
