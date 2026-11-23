use pikutracker_core::app::{Action, App, FileRequest, FileResponse};
use pikutracker_core::cart::Cart;
use pikutracker_core::export::{export, needs};
use pikutracker_core::project::{
    Chain, ChainEntry, ChainId, Claim, CustomInstrument, FIRST_CUSTOM_INSTRUMENT, InstrumentSound,
    PhraseId, Project, SfxInstrument, SongRowEnding, SongRowMarks, Step, TargetCart,
    WaveformInstrument,
};
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;
use std::path::Path;

const TEXT_CASES: u64 = 400;
const PNG_CASES: u64 = 40;

const PATTERNS_AT: usize = 0x3100;
const SFX_AT: usize = 0x3200;
const SHA1_AT: usize = 0x8006;
const PNG_WIDTH: u32 = 160;
const PNG_HEIGHT: u32 = 205;

struct Rng(u64);

impl Rng {
    fn new(case: u64) -> Self {
        Self(case.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, limit: usize) -> usize {
        ((self.next() >> 33) as usize) % limit
    }

    fn between(&mut self, low: usize, high: usize) -> usize {
        low + self.below(high - low + 1)
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn byte(&mut self) -> u8 {
        self.below(256) as u8
    }

    fn shuffled(&mut self, mut items: Vec<u8>) -> Vec<u8> {
        for index in (1..items.len()).rev() {
            items.swap(index, self.below(index + 1));
        }
        items
    }
}

struct Needs {
    sfx: usize,
    instruments: usize,
    patterns: usize,
}

fn random_project(rng: &mut Rng) -> (Project, Needs) {
    let mut project = Project::new();
    let instruments = rng.below(4);
    for id in 0..instruments {
        let sound = if rng.chance(50) {
            InstrumentSound::Waveform(WaveformInstrument {
                samples: std::array::from_fn(|_| rng.byte() as i8),
                bass: rng.chance(50),
                ..WaveformInstrument::default()
            })
        } else {
            InstrumentSound::Sfx(SfxInstrument::default())
        };
        let instrument = CustomInstrument { name: None, sound };
        project.set_custom_instrument(FIRST_CUSTOM_INSTRUMENT + id as u8, Some(instrument));
    }
    let tracks = rng.shuffled((1..=4).collect());
    let track_count = rng.between(1, 4);

    let song_rows = rng.between(1, 4);
    let (mut next_phrase, mut next_chain, mut patterns) = (0, 0, 0);
    for row in 0..song_rows {
        let chain_rows = rng.between(1, 3);
        patterns += chain_rows;
        let shapes: Vec<(i32, i32)> = (0..chain_rows)
            .map(|_| (rng.between(1, 32) as i32, rng.between(1, 255) as i32))
            .collect();
        for track in &tracks[..track_count] {
            let mut chain = Chain::default();
            for (k, (length, speed)) in shapes.iter().enumerate() {
                let id = PhraseId::new(next_phrase);
                next_phrase += 1;
                let phrase = project.phrase_mut(id);
                phrase.set_length(*length);
                phrase.set_speed(*speed);
                for step in 0..32 {
                    *phrase.step_mut(step) = Step::new(
                        rng.between(12, 50) as u8,
                        rng.below(8 + instruments) as u8,
                        rng.below(8) as u8,
                        rng.below(8) as u8,
                    )
                    .unwrap();
                }
                let mut entry = ChainEntry::new(id);
                entry.set_transpose(rng.between(0, 24) as i32 - 12);
                if rng.chance(25) {
                    let phrase = project.phrase_mut(id);
                    let end = phrase.length();
                    phrase.set_loop(rng.below(end) as i32, end as i32);
                }
                *chain.entry_mut(k) = Some(entry);
            }
            let id = ChainId::new(next_chain);
            next_chain += 1;
            *project.chain_mut(id) = chain;
            project.set_song_cell(row, usize::from(*track) - 1, Some(id));
        }
    }
    for row in 0..song_rows {
        let ending = [
            None,
            Some(SongRowEnding::LoopBack),
            Some(SongRowEnding::Stop),
        ];
        let marks = SongRowMarks {
            loop_start: rng.chance(25),
            ending: ending[rng.below(ending.len())],
        };
        project.set_song_row_marks(row, marks);
    }
    let needs = Needs {
        sfx: usize::from(next_phrase) + instruments,
        instruments,
        patterns,
    };
    (project, needs)
}

fn random_claim(rng: &mut Rng, needs: &Needs) -> Claim {
    let instrument_slots: Vec<u8> = (0..needs.instruments as u8).collect();
    let others = rng.shuffled((needs.instruments as u8..64).collect());
    let count = rng.between(needs.sfx, 64) - needs.instruments;
    let mut sfx = instrument_slots;
    sfx.extend(&others[..count]);
    let sfx = rng.shuffled(sfx);
    let first = rng.between(0, 64 - needs.patterns);
    let last = rng.between(first + needs.patterns - 1, 63);
    Claim {
        sfx,
        first_and_last_pattern: Some((first as u8, last as u8)),
    }
}

fn hex_text(rng: &mut Rng, digits: usize, upper: bool) -> String {
    let alphabet: &[u8] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    (0..digits)
        .map(|_| char::from(alphabet[rng.below(16)]))
        .collect()
}

fn random_p8(rng: &mut Rng) -> Vec<u8> {
    let eol = if rng.chance(30) { "\r\n" } else { "\n" };
    let upper = rng.chance(20);
    let mut lines: Vec<String> = vec![
        "pico-8 cartridge // http://www.pico-8.com".to_owned(),
        format!("version {}", rng.between(8, 43)),
    ];
    for name in ["__lua__", "__gfx__", "__label__", "__gff__", "__map__"] {
        if rng.chance(60) {
            lines.push(name.to_owned());
            for _ in 0..rng.below(4) {
                lines.push(format!("x{} = {}", rng.below(9), hex_text(rng, 6, false)));
            }
            if rng.chance(30) {
                lines.push(String::new());
            }
        }
    }
    if rng.chance(20) {
        lines.push("__meta:early__".to_owned());
        lines.push("before the music".to_owned());
    }
    if rng.chance(70) {
        lines.push("__sfx__".to_owned());
        for _ in 0..rng.below(65) {
            lines.push(hex_text(rng, 168, upper));
            if rng.chance(5) {
                lines.push(String::new());
            }
        }
    }
    if rng.chance(70) {
        lines.push("__music__".to_owned());
        for _ in 0..rng.below(65) {
            let flags = hex_text(rng, 2, upper);
            lines.push(format!("{flags} {}", hex_text(rng, 8, upper)));
        }
    }
    for index in 0..rng.below(3) {
        lines.push(format!("__meta:extra{index}__"));
        lines.push(format!("data = {}", hex_text(rng, 12, false)));
    }
    let mut text = lines.join(eol);
    if rng.chance(70) {
        text.push_str(eol);
    }
    if rng.chance(10) {
        text.insert(0, '\u{feff}');
    }
    text.into_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Mark,
    SfxRow(usize),
    MusicRow(usize),
    Other,
}

struct Line<'a> {
    whole: &'a [u8],
    body: &'a [u8],
    kind: Kind,
}

fn parse_lines(bytes: &[u8]) -> Vec<Line<'_>> {
    let (mut in_sfx, mut in_music) = (false, false);
    let (mut sfx_rows, mut music_rows) = (0, 0);
    bytes
        .split_inclusive(|byte| *byte == b'\n')
        .map(|whole| {
            let body = whole.strip_suffix(b"\n").unwrap_or(whole);
            let body = body.strip_suffix(b"\r").unwrap_or(body);
            let kind = if body.len() > 4 && body.starts_with(b"__") && body.ends_with(b"__") {
                in_sfx = body == b"__sfx__";
                in_music = body == b"__music__";
                Kind::Mark
            } else if body.is_empty() {
                Kind::Other
            } else if in_sfx {
                sfx_rows += 1;
                Kind::SfxRow(sfx_rows - 1)
            } else if in_music {
                music_rows += 1;
                Kind::MusicRow(music_rows - 1)
            } else {
                Kind::Other
            };
            Line { whole, body, kind }
        })
        .collect()
}

fn line_ending(line: &[u8]) -> &[u8] {
    if line.ends_with(b"\r\n") {
        b"\r\n"
    } else if line.ends_with(b"\n") {
        b"\n"
    } else {
        b""
    }
}

fn count_rows(lines: &[Line<'_>], row: fn(Kind) -> bool) -> usize {
    lines.iter().filter(|line| row(line.kind)).count()
}

fn has_mark(lines: &[Line<'_>], name: &[u8]) -> bool {
    lines.iter().any(|line| line.body == name)
}

fn mark_position(lines: &[Line<'_>], name: &[u8]) -> Option<usize> {
    lines.iter().position(|line| line.body == name)
}

fn assert_only_the_claim_changed(case: u64, before: &[u8], after: &[u8], claim: &Claim) {
    let claimed_sfx: BTreeSet<usize> = claim.sfx.iter().map(|slot| usize::from(*slot)).collect();
    let (first, last) = claim.first_and_last_pattern.unwrap();
    let claimed_music = usize::from(first)..=usize::from(last);
    let is_claimed = |kind: Kind| match kind {
        Kind::SfxRow(row) => claimed_sfx.contains(&row),
        Kind::MusicRow(row) => claimed_music.contains(&row),
        _ => false,
    };

    let old = parse_lines(before);
    let new = parse_lines(after);
    let old_sfx = count_rows(&old, |kind| matches!(kind, Kind::SfxRow(_)));
    let old_music = count_rows(&old, |kind| matches!(kind, Kind::MusicRow(_)));
    let had_sfx = has_mark(&old, b"__sfx__");
    let had_music = has_mark(&old, b"__music__");
    let is_new = |line: &Line<'_>| match line.kind {
        Kind::SfxRow(row) => row >= old_sfx,
        Kind::MusicRow(row) => row >= old_music,
        Kind::Mark => {
            (line.body == b"__sfx__" && !had_sfx) || (line.body == b"__music__" && !had_music)
        }
        Kind::Other => false,
    };

    let kept_old: Vec<&Line<'_>> = old.iter().filter(|line| !is_claimed(line.kind)).collect();
    let kept_new: Vec<&Line<'_>> = new
        .iter()
        .filter(|line| !is_claimed(line.kind) && !is_new(line))
        .collect();
    assert_eq!(kept_old.len(), kept_new.len(), "case {case}: kept lines");
    for (index, (old_line, new_line)) in kept_old.iter().zip(&kept_new).enumerate() {
        let was_open_ended = line_ending(old_line.whole).is_empty();
        if was_open_ended {
            assert_eq!(old_line.body, new_line.body, "case {case}: line {index}");
        } else {
            assert_eq!(old_line.whole, new_line.whole, "case {case}: line {index}");
        }
    }

    let file_ending = line_ending(old[0].whole);
    for line in new.iter().filter(|line| is_new(line)) {
        let ending = line_ending(line.whole);
        assert!(
            ending == file_ending || ending.is_empty(),
            "case {case}: line ending of a new line"
        );
    }

    for line in new
        .iter()
        .filter(|line| is_new(line) && !is_claimed(line.kind))
    {
        match line.kind {
            Kind::SfxRow(_) => {
                let default = format!("00100000{}", "0".repeat(160));
                assert_eq!(line.body, default.as_bytes(), "case {case}: gap row");
            }
            Kind::MusicRow(_) => {
                assert_eq!(line.body, b"00 41424344", "case {case}: gap row");
            }
            _ => {}
        }
    }

    for old_line in old.iter().filter(|line| is_claimed(line.kind)) {
        let new_line = new.iter().find(|line| line.kind == old_line.kind).unwrap();
        if !line_ending(old_line.whole).is_empty() {
            assert_eq!(
                line_ending(old_line.whole),
                line_ending(new_line.whole),
                "case {case}: line ending of a claimed row"
            );
        }
    }

    assert_new_sections_are_in_canonical_order(case, &new, had_sfx, had_music);
}

fn assert_new_sections_are_in_canonical_order(
    case: u64,
    new: &[Line<'_>],
    had_sfx: bool,
    had_music: bool,
) {
    let earlier: [&[u8]; 5] = [b"__lua__", b"__gfx__", b"__label__", b"__gff__", b"__map__"];
    if let (false, Some(sfx)) = (had_sfx, mark_position(new, b"__sfx__")) {
        for name in earlier {
            if let Some(position) = mark_position(new, name) {
                assert!(position < sfx, "case {case}: __sfx__ comes too early");
            }
        }
    }
    if let (false, Some(music), Some(sfx)) = (
        had_music,
        mark_position(new, b"__music__"),
        mark_position(new, b"__sfx__"),
    ) {
        assert!(sfx < music, "case {case}: __music__ comes before __sfx__");
    }
}

fn assert_slots_changed_only_inside_the_claim(
    case: u64,
    before: &[u8],
    after: &[u8],
    claim: &Claim,
) {
    let old = Cart::read(before).unwrap();
    let new = Cart::read(after).unwrap();
    let (first, last) = claim.first_and_last_pattern.unwrap();
    for slot in 0..64 {
        if !claim.sfx.contains(&slot) {
            assert_eq!(
                old.sfx_bytes(slot),
                new.sfx_bytes(slot),
                "case {case}: SFX {slot}"
            );
        }
        if !(first..=last).contains(&slot) {
            assert_eq!(
                old.pattern_bytes(slot),
                new.pattern_bytes(slot),
                "case {case}: Pattern {slot}"
            );
        }
    }
}

#[test]
fn a_p8_export_changes_no_byte_outside_the_claim() {
    for case in 0..TEXT_CASES {
        let mut rng = Rng::new(case + 1);
        let before = random_p8(&mut rng);
        let (project, needs) = random_project(&mut rng);
        let claim = random_claim(&mut rng, &needs);

        let exported = export(&project, &claim, &before)
            .unwrap_or_else(|failures| panic!("case {case}: {failures:?}"));

        assert_only_the_claim_changed(case, &before, &exported.cart_bytes, &claim);
        assert_slots_changed_only_inside_the_claim(case, &before, &exported.cart_bytes, &claim);
        let written = Cart::read(&exported.cart_bytes).unwrap();
        assert_eq!(
            written.claimed_bytes_hash(&claim),
            exported.claimed_bytes_hash,
            "case {case}: hash"
        );
    }
}

fn break_export(rng: &mut Rng, project: &mut Project, claim: &mut Claim) -> &'static str {
    let needed = needs(project);
    match rng.below(6) {
        5 if !needed.instrument_slots.is_empty() => {
            claim
                .sfx
                .retain(|slot| !needed.instrument_slots.contains(slot));
            "no SFX k for Custom Instrument 8 + k"
        }
        0 | 5 => {
            for row in 0..4 {
                for track in 0..4 {
                    project.set_song_cell(row, track, None);
                }
            }
            "an empty Song"
        }
        1 => {
            claim.sfx.sort_unstable();
            claim.sfx.dedup();
            claim.sfx.truncate(needed.sfx - 1);
            "too few SFX"
        }
        2 => {
            claim.first_and_last_pattern = claim
                .first_and_last_pattern
                .filter(|_| needed.patterns > 1)
                .map(|(first, _)| (first, first + needed.patterns as u8 - 2));
            "too few Patterns"
        }
        3 => {
            *claim = Claim::default();
            "an empty Claim"
        }
        _ => {
            let chain = (0..4)
                .find_map(|track| project.song_cell(0, track))
                .unwrap();
            let entry = project.chain_mut(chain).entry_mut(0).as_mut().unwrap();
            entry.set_transpose(1);
            let phrase = entry.phrase();
            *project.phrase_mut(phrase).step_mut(0) = Step::new(63, 0, 5, 0).unwrap();
            "an audible note out of range"
        }
    }
}

#[test]
fn a_failed_export_leaves_the_cart_byte_for_byte_unchanged() {
    let path = Path::new(".").join("game.p8");
    for case in 0..TEXT_CASES {
        let mut rng = Rng::new(case + 1);
        let before = if case % 10 == 0 {
            encode(&random_pixels(&mut rng))
        } else {
            random_p8(&mut rng)
        };
        let (mut project, needs) = random_project(&mut rng);
        let mut claim = random_claim(&mut rng, &needs);
        let broken = break_export(&mut rng, &mut project, &mut claim);
        project.set_target_cart(Some(TargetCart {
            path: "game.p8".to_owned(),
            claim,
            claimed_bytes_hash: None,
        }));
        let mut app = App::with_project(project);

        app.apply(Action::Export);
        app.take_file_requests();
        app.receive_file(FileResponse::BytesRead {
            path: path.clone(),
            contents: before,
        });

        let requests = app.take_file_requests();
        assert!(
            !requests
                .iter()
                .any(|request| matches!(request, FileRequest::WriteBytes { .. })),
            "case {case}, {broken}: the Cart was written"
        );
        let status = app.grid().text_dump().lines().last().unwrap().to_owned();
        assert!(
            status.contains("EXPORT FAILED"),
            "case {case}, {broken}: {status}"
        );
    }
}

fn random_pixels(rng: &mut Rng) -> Vec<u8> {
    (0..PNG_WIDTH * PNG_HEIGHT * 4)
        .map(|_| rng.byte())
        .collect()
}

fn encode(pixels: &[u8]) -> Vec<u8> {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, PNG_WIDTH, PNG_HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixels).unwrap();
    writer.finish().unwrap();
    file
}

fn decode(file: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(file))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    pixels
}

fn memory_byte(pixel: &[u8; 4]) -> u8 {
    (pixel[3] & 3) << 6 | (pixel[0] & 3) << 4 | (pixel[1] & 3) << 2 | (pixel[2] & 3)
}

fn claimed_addresses(claim: &Claim) -> BTreeSet<usize> {
    let mut addresses: BTreeSet<usize> = (SHA1_AT..SHA1_AT + 20).collect();
    for slot in &claim.sfx {
        let at = SFX_AT + usize::from(*slot) * 68;
        addresses.extend(at..at + 68);
    }
    let (first, last) = claim.first_and_last_pattern.unwrap();
    for slot in first..=last {
        let at = PATTERNS_AT + usize::from(slot) * 4;
        addresses.extend(at..at + 4);
    }
    addresses
}

#[test]
fn a_png_export_changes_no_pixel_outside_the_claim_and_the_sha1() {
    for case in 0..PNG_CASES {
        let mut rng = Rng::new(case + 1);
        let pixels = random_pixels(&mut rng);
        let before = encode(&pixels);
        let (project, needs) = random_project(&mut rng);
        let claim = random_claim(&mut rng, &needs);

        let exported = export(&project, &claim, &before)
            .unwrap_or_else(|failures| panic!("case {case}: {failures:?}"));

        let after = decode(&exported.cart_bytes);
        assert_eq!(pixels.len(), after.len(), "case {case}: image size");
        let may_change = claimed_addresses(&claim);
        let old_pixels = pixels.as_chunks::<4>().0.iter();
        for (address, (old, new)) in old_pixels.zip(after.as_chunks::<4>().0).enumerate() {
            if may_change.contains(&address) {
                for (old_channel, new_channel) in old.iter().zip(new) {
                    assert_eq!(
                        old_channel & 0xFC,
                        new_channel & 0xFC,
                        "case {case}: picture at {address:#x}"
                    );
                }
            } else {
                assert_eq!(old, new, "case {case}: pixel {address:#x}");
            }
        }

        let memory: Vec<u8> = after.as_chunks::<4>().0.iter().map(memory_byte).collect();
        let digest = Sha1::digest(&memory[..0x8000]);
        assert_eq!(
            memory[SHA1_AT..SHA1_AT + 20],
            *digest.as_slice(),
            "case {case}: SHA-1"
        );
        let written = Cart::read(&exported.cart_bytes).unwrap();
        assert_eq!(
            written.claimed_bytes_hash(&claim),
            exported.claimed_bytes_hash,
            "case {case}: hash"
        );
    }
}
