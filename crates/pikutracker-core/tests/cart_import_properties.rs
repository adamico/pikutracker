mod support;

use pikutracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use pikutracker_core::cart::Cart;
use pikutracker_core::cart_import::{ImportedCart, import};
use pikutracker_core::project::{CHAIN_ENTRIES, MAX_STEPS, Project};
use std::path::PathBuf;
use support::{TestCart, TestNote, TestPattern, TestSfx, png_of};

const RANDOM_CARTS: u64 = 60;
const DAMAGED_CARTS: u64 = 120;
const CART_BYTES: usize = 160 * 205;
const SONG_ROWS_CHECKED: usize = 64;

const LOOP_BYTES: [u8; 14] = [0, 1, 2, 4, 8, 31, 32, 33, 40, 64, 127, 128, 200, 255];
const SPEEDS: [u8; 9] = [0, 1, 2, 3, 5, 8, 16, 100, 255];

struct Random(u64);

impl Random {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, limit: usize) -> usize {
        (self.next() % limit as u64) as usize
    }

    fn byte(&mut self) -> u8 {
        self.next() as u8
    }

    fn one_of(&mut self, values: &[u8]) -> u8 {
        values[self.below(values.len())]
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

fn random_sfx(random: &mut Random) -> TestSfx {
    let mut sfx = TestSfx::with_pitches(random.one_of(&SPEEDS), &[]);
    sfx.editor_and_filters = random.byte();
    sfx.loop_start = random.one_of(&LOOP_BYTES);
    sfx.loop_end = random.one_of(&LOOP_BYTES);
    for note in &mut sfx.notes {
        *note = TestNote {
            pitch: random.byte() & 0x3f,
            instrument: random.byte() & 7,
            volume: random.byte() & 7,
            effect: random.byte() & 7,
            custom: random.chance(10),
        };
    }
    sfx
}

fn random_pattern(random: &mut Random) -> TestPattern {
    let mut channels = [None; 4];
    for channel in &mut channels {
        if random.chance(70) {
            *channel = Some(random.byte() & 0x3f);
        }
    }
    TestPattern {
        channels,
        loop_start: random.chance(10),
        loop_back: random.chance(5),
        stop: random.chance(5),
    }
}

fn random_cart(random: &mut Random) -> TestCart {
    TestCart {
        sfx: (0..64).map(|_| random_sfx(random)).collect(),
        patterns: (0..64).map(|_| random_pattern(random)).collect(),
    }
}

fn expected_ticks(cart: &TestCart, pattern: &TestPattern) -> u32 {
    let playing: Vec<&TestSfx> = pattern
        .channels
        .iter()
        .flatten()
        .map(|slot| &cart.sfx[usize::from(*slot)])
        .collect();
    let speed = |sfx: &TestSfx| u32::from(sfx.speed.max(1));
    let bit_7 = |sfx: &TestSfx| sfx.loop_start >= 128;
    let loops = |sfx: &TestSfx| !bit_7(sfx) && sfx.loop_start < sfx.loop_end;
    if let Some(sfx) = playing.iter().find(|sfx| !bit_7(sfx) && !loops(sfx)) {
        let has_len = sfx.loop_end == 0 && sfx.loop_start > 0;
        let notes = if has_len { sfx.loop_start.into() } else { 32 };
        return notes * speed(sfx);
    }
    let slowest = |sfx: Vec<&&TestSfx>| sfx.iter().map(|sfx| speed(sfx)).max();
    let looping = slowest(playing.iter().filter(|sfx| loops(sfx)).collect());
    32 * looping.or(slowest(playing.iter().collect())).unwrap_or(0)
}

fn song_row_ticks(project: &Project, row: usize) -> u32 {
    (0..CHAIN_ENTRIES)
        .map(|k| project.chain_row_ticks(row, k))
        .sum()
}

fn first_song(cart: &TestCart) -> Vec<&TestPattern> {
    let is_empty = |pattern: &TestPattern| pattern.channels == [None; 4];
    let mut song = Vec::new();
    for pattern in cart.patterns.iter().skip_while(|pattern| is_empty(pattern)) {
        if is_empty(pattern) {
            break;
        }
        song.push(pattern);
        if pattern.stop || pattern.loop_back {
            break;
        }
    }
    song
}

fn assert_project_is_sound(imported: &ImportedCart) {
    let project = &imported.project;
    for row in 0..SONG_ROWS_CHECKED {
        for track in 0..4 {
            for k in 0..CHAIN_ENTRIES {
                let Some(entry) = project.chain_row_entry(row, track, k) else {
                    continue;
                };
                let phrase = project.phrase(entry.phrase());
                assert!((1..=MAX_STEPS).contains(&phrase.length()));
                assert!(phrase.speed() >= 1);
            }
        }
    }
    for warning in &imported.warnings {
        let Some(place) = warning.place else { continue };
        assert!(place.chain_row.song_row < SONG_ROWS_CHECKED, "{warning:?}");
        assert!(place.track < 4, "{warning:?}");
        assert!(place.chain_row.entry < CHAIN_ENTRIES, "{warning:?}");
        assert!(place.step_index < MAX_STEPS, "{warning:?}");
    }
}

#[test]
fn random_carts_import_with_the_length_pico_8_plays() {
    for seed in 1..=RANDOM_CARTS {
        let cart = random_cart(&mut Random::new(seed));

        let imported = import(&Cart::read(&cart.p8()).unwrap(), "random.p8");

        assert_project_is_sound(&imported);
        let out_of_phrases = imported
            .warnings
            .iter()
            .any(|warning| warning.text.contains("MORE THAN 256 PHRASES"));
        if out_of_phrases {
            continue;
        }
        for (row, pattern) in first_song(&cart).into_iter().enumerate() {
            assert_eq!(
                song_row_ticks(&imported.project, row),
                expected_ticks(&cart, pattern),
                "seed {seed}, Song row {row}"
            );
        }
    }
}

#[test]
fn carts_of_random_bytes_never_crash_import() {
    for seed in 1..=RANDOM_CARTS {
        let mut random = Random::new(seed);
        let memory: Vec<u8> = (0..CART_BYTES).map(|_| random.byte()).collect();

        let cart = Cart::read(&png_of(&memory, 160, 205)).unwrap();

        assert_project_is_sound(&import(&cart, "random.p8.png"));
    }
}

fn damage(bytes: &mut Vec<u8>, random: &mut Random) {
    for _ in 0..=random.below(8) {
        let at = random.below(bytes.len().max(1));
        match random.below(4) {
            0 if at < bytes.len() => bytes[at] = random.byte(),
            1 if at < bytes.len() => {
                bytes.remove(at);
            }
            2 => bytes.insert(at, random.byte()),
            _ => bytes.truncate(bytes.len().saturating_sub(random.below(64))),
        }
    }
}

#[test]
fn damaged_carts_are_refused_or_imported_but_never_crash() {
    for seed in 1..=DAMAGED_CARTS {
        let mut random = Random::new(seed);
        let cart = random_cart(&mut random);
        let mut bytes = if seed % 10 == 0 {
            cart.png()
        } else {
            cart.p8()
        };

        damage(&mut bytes, &mut random);

        if let Ok(cart) = Cart::read(&bytes) {
            assert_project_is_sound(&import(&cart, "damaged.p8"));
        }
    }
}

#[test]
fn the_app_shows_the_warnings_of_random_carts_without_crashing() {
    for seed in 1..=RANDOM_CARTS / 5 {
        let cart = random_cart(&mut Random::new(seed));
        let mut app = App::new();
        app.apply(Action::Import);
        app.take_file_requests();
        app.receive_file(FileResponse::Listed {
            directory: PathBuf::from("."),
            entries: vec![DirectoryEntry {
                name: "random.p8".to_owned(),
                is_directory: false,
            }],
        });
        app.apply(Action::Move(Direction::Down));
        app.apply(Action::Confirm);
        let requests = app.take_file_requests();
        let [FileRequest::ReadBytes { path }] = requests.as_slice() else {
            panic!("expected the Cart to be read, got {requests:?}");
        };
        app.receive_file(FileResponse::BytesRead {
            path: path.clone(),
            contents: cart.p8(),
        });

        jump_to_the_place_of_every_warning_in_the_log(&mut app);
    }
}

fn jump_to_the_place_of_every_warning_in_the_log(app: &mut App) {
    app.apply(Action::FocusNext);
    for _ in 0..8 {
        app.apply(Action::Confirm);
        app.apply(Action::Move(Direction::Down));
        assert!(!app.grid().text_dump().is_empty());
    }
}
