use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, Step};
use pikutracker_core::screen::font::{GLYPH_HEIGHT, GLYPH_WIDTH, glyph};
use pikutracker_core::screen::frame::{Frame, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use pikutracker_core::screen::grid::{GRID_COLUMNS, GRID_ROWS, Grid, Style};
use pikutracker_core::screen::palette::Colour;
use pikutracker_core::screen::viewport::integer_scale;

const SNAPSHOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/snapshots/font_and_palette.png"
);
const README_SCREENSHOT_AT_DOUBLE_SIZE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/images/song-chain-phrase.png"
);

const PICO8_PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [29, 43, 83],
    [126, 37, 83],
    [0, 135, 81],
    [171, 82, 54],
    [95, 87, 79],
    [194, 195, 199],
    [255, 241, 232],
    [255, 0, 77],
    [255, 163, 0],
    [255, 236, 39],
    [0, 228, 54],
    [41, 173, 255],
    [131, 118, 156],
    [255, 119, 168],
    [255, 204, 170],
];

fn pixel(frame: &Frame, x: usize, y: usize) -> [u8; 4] {
    let offset = (y * LOGICAL_WIDTH as usize + x) * 4;
    frame.rgba()[offset..offset + 4].try_into().unwrap()
}

fn font_and_palette_grid() -> Grid {
    let mut grid = Grid::new();
    let glyphs: Vec<char> = (' '..='_').chain(['|', '~', '│']).collect();
    for (index, colour) in Colour::ALL.into_iter().enumerate() {
        let line: String = glyphs.iter().collect();
        grid.put(0, index, &line, Style::fg(colour));
        let background = Style {
            fg: Colour::Black,
            bg: colour,
        };
        grid.put(0, 20 + index, "PIKUTRACKER 0123456789", background);
    }
    grid
}

#[test]
fn the_palette_is_pico8s() {
    let colours: Vec<[u8; 4]> = Colour::ALL.into_iter().map(Colour::rgba).collect();
    let expected: Vec<[u8; 4]> = PICO8_PALETTE
        .into_iter()
        .map(|[r, g, b]| [r, g, b, 255])
        .collect();

    assert_eq!(colours, expected);
}

#[test]
fn the_frame_is_640_by_360_and_holds_106_by_45_cells_of_6_by_8() {
    assert_eq!((LOGICAL_WIDTH, LOGICAL_HEIGHT), (640, 360));
    assert_eq!((GRID_COLUMNS, GRID_ROWS), (106, 45));
    assert_eq!(Frame::from_grid(&Grid::new()).rgba().len(), 640 * 360 * 4);
}

#[test]
fn a_glyph_is_drawn_5_by_7_from_the_top_left_of_its_cell() {
    let mut grid = Grid::new();
    grid.put(2, 1, "A", Style::fg(Colour::White));
    let frame = Frame::from_grid(&grid);

    let expected = [
        ".###..", "#...#.", "#...#.", "#####.", "#...#.", "#...#.", "#...#.", "......",
    ];
    for (y, row) in expected.iter().enumerate() {
        for (x, mark) in row.chars().enumerate() {
            let want = if mark == '#' {
                Colour::White.rgba()
            } else {
                Colour::Black.rgba()
            };
            assert_eq!(pixel(&frame, 12 + x, 8 + y), want, "pixel {x},{y}");
        }
    }
}

#[test]
fn a_background_fills_the_whole_cell() {
    let mut grid = Grid::new();
    let style = Style {
        fg: Colour::White,
        bg: Colour::Red,
    };
    grid.put(0, 0, " ", style);
    let frame = Frame::from_grid(&grid);

    assert_eq!(pixel(&frame, 5, 7), Colour::Red.rgba());
    assert_eq!(pixel(&frame, 6, 7), Colour::Black.rgba());
    assert_eq!(pixel(&frame, 5, 8), Colour::Black.rgba());
}

#[test]
fn a_progress_bar_fills_pixels_from_the_left_under_the_character() {
    let mut grid = Grid::new();
    grid.put(0, 0, "00", Style::fg(Colour::White));
    grid.put_bar(0, 0, 2, (3, 4), Colour::DarkGreen);
    let frame = Frame::from_grid(&grid);

    let ink = glyph('0');
    for x in 0..12 {
        for y in 0..8 {
            let inked = x % 6 < GLYPH_WIDTH
                && y < GLYPH_HEIGHT
                && ink[y] & (1 << (GLYPH_WIDTH - 1 - x % 6)) != 0;
            let expected = if inked {
                Colour::White
            } else if x < 9 {
                Colour::DarkGreen
            } else {
                Colour::Black
            };
            assert_eq!(pixel(&frame, x, y), expected.rgba(), "pixel {x},{y}");
        }
    }
}

#[test]
fn a_separator_is_an_unbroken_line_down_the_cell() {
    let mut grid = Grid::new();
    grid.put(0, 0, "│", Style::fg(Colour::White));
    let frame = Frame::from_grid(&grid);

    for y in 0..8 {
        assert_eq!(pixel(&frame, 2, y), Colour::White.rgba(), "row {y}");
    }
}

fn assert_matches_snapshot(frame: &Frame, scale: usize, path: &str) {
    let width = LOGICAL_WIDTH as usize * scale;
    let height = LOGICAL_HEIGHT as usize * scale;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            pixels.extend(pixel(frame, x / scale, y / scale));
        }
    }

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = png::Encoder::new(file, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&pixels).unwrap();
    }

    let file = std::io::BufReader::new(std::fs::File::open(path).unwrap());
    let mut reader = png::Decoder::new(file).read_info().unwrap();
    let mut snapshot = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut snapshot).unwrap();

    assert!(snapshot == pixels, "pixels differ from {path}");
}

#[test]
fn the_font_and_palette_match_the_pixel_snapshot() {
    assert_matches_snapshot(&Frame::from_grid(&font_and_palette_grid()), 1, SNAPSHOT);
}

const SQUARE: u8 = 3;
const TRIANGLE: u8 = 0;

fn notes(project: &mut Project, phrase: u8, instrument: u8, pitches: &[u8]) -> PhraseId {
    let id = PhraseId::new(phrase);
    let phrase = project.phrase_mut(id);
    phrase.set_length(pitches.len() as i32);
    for (index, pitch) in pitches.iter().enumerate() {
        *phrase.step_mut(index) = Step::new(*pitch, instrument, 5, 0).unwrap();
    }
    id
}

fn chain(project: &mut Project, chain: u8, phrases: &[PhraseId]) -> ChainId {
    let id = ChainId::new(chain);
    for (index, phrase) in phrases.iter().enumerate() {
        *project.chain_mut(id).entry_mut(index) = Some(ChainEntry::new(*phrase));
    }
    id
}

fn readme_project() -> Project {
    let mut project = Project::new();
    let verse = notes(
        &mut project,
        0x00,
        SQUARE,
        &[24, 28, 31, 28, 24, 28, 31, 36],
    );
    let turn = notes(
        &mut project,
        0x01,
        SQUARE,
        &[33, 31, 29, 28, 26, 28, 29, 31],
    );
    let ending = notes(&mut project, 0x02, SQUARE, &[24, 24, 24, 24, 0, 0, 0, 0]);
    let bass = notes(
        &mut project,
        0x03,
        TRIANGLE,
        &[12, 12, 19, 19, 17, 17, 19, 19],
    );
    let melody = chain(&mut project, 0x00, &[verse, verse, turn]);
    let melody_end = chain(&mut project, 0x01, &[turn, ending]);
    let bass_line = chain(&mut project, 0x02, &[bass, bass, bass]);
    let bass_end = chain(&mut project, 0x03, &[bass, bass]);
    project.set_song_cell(0, 0, Some(melody));
    project.set_song_cell(0, 1, Some(bass_line));
    project.set_song_cell(1, 0, Some(melody_end));
    project.set_song_cell(1, 1, Some(bass_end));
    project
}

#[test]
fn the_readme_screenshot_matches_the_app() {
    let mut app = App::with_project(readme_project());
    for action in [
        Action::FocusNext,
        Action::Move(Direction::Down),
        Action::Move(Direction::Down),
    ] {
        app.apply(action);
    }

    assert_matches_snapshot(
        &Frame::from_grid(&app.grid()),
        2,
        README_SCREENSHOT_AT_DOUBLE_SIZE,
    );
}

#[test]
fn the_window_scales_by_the_largest_whole_number_that_fits() {
    let decided = [
        (1366, 768, 2),
        (1920, 1080, 3),
        (2560, 1440, 4),
        (3840, 2160, 6),
    ];

    for (width, height, scale) in decided {
        assert_eq!(integer_scale(width, height), scale, "{width}x{height}");
    }
}

#[test]
fn a_window_too_small_for_times_two_falls_back_to_times_one() {
    for (width, height) in [(960, 1050), (1279, 720), (640, 360), (320, 200), (0, 0)] {
        assert_eq!(integer_scale(width, height), 1, "{width}x{height}");
    }
}
