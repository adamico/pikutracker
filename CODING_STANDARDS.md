# Coding standards

Review applies every rule here. CI enforces the mechanical rules (`.github/workflows/ci.yml`), so a review spends its attention on the judgement calls below.

## Code carries no comments

A name, a type or a test says what code does, and an ADR in `docs/adr/` says why. `scripts/check-no-comments.py` fails CI on any comment, so a review spends its attention on where the meaning went instead.

- Flag a name that needs a comment to make sense. Rename it until it reads on its own.
- Give a primitive its own small type when its name cannot say its range or unit, such as a Track numbered 1 to 8.
- Flag a commented-looking block left as one long function: extract the block into a function named for what it does.
- Flag a reason with no home: a deviation from the zepto8 port, a PICO-8 measurement or an owner's decision that the change relies on, with no ADR recording it.
- Flag prose moved into strings: an `expect` message, a test name or a constant's text that reads as an explanation rather than a fact.

## Logic lives in the App core

The Window shell (`crates/pikutracker`) only connects the App core to the window, the audio device and the disk. Every decision lives in the App core, where the main test seam reaches it. Flag a decision made in the shell, such as a branch on the Project or the wording of a message.

Every key reaches the App core as a named `Action` from the keymap. Flag a key handled anywhere else.

## Glossary terms

`GLOSSARY.md` is the vocabulary for identifiers, messages, tests and docs.

- A defined term means exactly its definition. A different idea gets a different word: an undo entry is not a Step.
- An avoided word stays out of new code. PICO-8's and MIDI's own names are exempt where they mean the PICO-8 or MIDI thing, such as `music()` or a MIDI sequence.

## Renames are complete

One idea has one name. A rename covers every identifier, field, helper, test name, message and doc that uses the old word. Reread each touched message as a sentence: a search-and-replace leaves phrases like "every cell in a cells".

## One home per fact

Each constant, format string and helper is defined once and imported where needed. Flag a second definition of the same value (a pitch limit, the SFX layout) or the same small logic (plural wording, a warning's place prefix).

## Tests

Tests exercise behaviour through a seam, in the domain's language: actions, Projects, Carts, warnings and samples. Refactoring without a behaviour change keeps every test green.

New tests use the highest seam that can express them:

1. **Headless App core**, the main seam: named actions and file contents in; the Project, the grid's text dump, the status line and Log, samples and written bytes out.
2. **Cart codec**: `.p8` and `.p8.png` bytes to and from Cart data.
3. **Music compilers**: a Cart or MIDI file to a Project and warnings, and a Project, Cart and Claim to a new Cart or failures.
4. **Offline synth**: Cart data or a Project to samples.
5. **Project file**: `.ptrk` text to and from a Project.

Expected data comes from real PICO-8 (saved Carts, measurements) or from a decision in an ADR, never from the code under test. A test that reads source text breaks this rule.

### Proof per area

A change to an area carries that area's proof. Flag a change that has none.

| Area | Proof |
|---|---|
| Cart reading and writing | Carts saved by real PICO-8 with expected bytes, plus read-write round trips |
| Cart Import | Expected `.ptrk` snapshots and warning lists. An Import with no warnings renders the same samples as the original Cart through our synth |
| MIDI Import | Small MIDI files built in test code, with expected Projects |
| Export | The Exported Cart renders the same samples as the Project's 4 Tracks through our synth, Track n on channel n − 1. A property test keeps every byte outside the Claim unchanged |
| Playback | Comparison with stored measurements of real PICO-8, plus exact snapshots of our synth's output |
| Keyboard UI | Actions drive the headless App core. Tests assert on the Project and the grid's text, plus one pixel snapshot of the font and palette |
| `.ptrk` files | One fixture per released format version loads to the expected Project, and saving and reloading gives an identical Project |

The Cart Import and Export comparisons go through our synth by design. A test that builds their expected samples with `Voice` or `Player` follows the expected-data rule.

### Fingerprint limits

Loosening a limit in a comparison with real PICO-8 measurements needs an A/B listening check by the owner, noted in the commit. Flag a loosened limit whose commit has no such note.
