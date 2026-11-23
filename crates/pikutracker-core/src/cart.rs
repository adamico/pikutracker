mod write;

use crate::project::Claim;
use std::fmt;
use std::io::Cursor;

pub const SLOTS: usize = 64;
pub const SFX_NOTES: usize = 32;
pub(crate) const SFX_BYTES: usize = 68;
const PATTERN_BYTES: usize = 4;

const PATTERNS_AT: usize = 0x3100;
const SFX_AT: usize = 0x3200;
const MUSIC_END: usize = SFX_AT + SLOTS * SFX_BYTES;

pub(crate) const FILTERS_BYTE: usize = 64;
pub(crate) const SPEED_BYTE: usize = 65;
pub(crate) const LOOP_START_BYTE: usize = 66;
pub(crate) const LOOP_END_BYTE: usize = 67;

pub(crate) const EDITOR_MODE_BIT: u8 = 0x01;
pub(crate) const WAVEFORM_INSTRUMENT_BIT: u8 = 0x80;

const CHANNELS: usize = 4;
const CHANNEL_DISABLED: u8 = 0x40;
const SFX_MASK: u8 = 0x3f;
const FLAG: u8 = 0x80;
const EMPTY_PATTERN: [u8; PATTERN_BYTES] = [0x41, 0x42, 0x43, 0x44];
const OMITTED_ROW_SPEED: u8 = 16;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const PNG_WIDTH: u32 = 160;
const PNG_HEIGHT: u32 = 205;
const PNG_CHANNELS: usize = 4;
const PIXEL_DATA_BITS: u8 = 0b11;

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const TEXT_HEADER: &[u8] = b"pico-8 cartridge";
const NEW_CART_HEADER_TAIL: &[u8] = b" // http://www.pico-8.com";
const NEW_CART_LINES: [&[u8]; 2] = [b"version 43", b"__lua__"];
const SFX_SECTION: &[u8] = b"__sfx__";
const MUSIC_SECTION: &[u8] = b"__music__";
const SECTION_MARK: &[u8] = b"__";
const SFX_HEADER_DIGITS: usize = 8;
const NOTE_DIGITS: usize = 5;
const SFX_LINE_DIGITS: usize = SFX_HEADER_DIGITS + SFX_NOTES * NOTE_DIGITS;
const MUSIC_LINE_LENGTH: usize = 11;
const MUSIC_FLAGS_DIGITS: usize = 2;

const FNV_OFFSET: u32 = 0x811C_9DC5;
const FNV_PRIME: u32 = 0x0100_0193;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CartError {
    NotACart,
    BadLine { line: usize },
}

impl fmt::Display for CartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotACart => write!(formatter, "NOT A PICO-8 CART"),
            Self::BadLine { line } => write!(formatter, "LINE {line} IS DAMAGED"),
        }
    }
}

impl std::error::Error for CartError {}

pub(crate) const fn note_word(
    pitch: u16,
    instrument: u16,
    volume: u16,
    effect: u16,
    custom: u16,
) -> u16 {
    pitch | instrument << 6 | volume << 9 | effect << 12 | custom << 15
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CartNote {
    pub pitch: u8,
    pub instrument: u8,
    pub volume: u8,
    pub effect: u8,
    pub custom: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sfx<'a> {
    bytes: &'a [u8; SFX_BYTES],
}

impl Sfx<'_> {
    pub fn note(self, index: usize) -> CartNote {
        let word = u16::from_le_bytes([self.bytes[index * 2], self.bytes[index * 2 + 1]]);
        CartNote {
            pitch: (word & 0x3f) as u8,
            instrument: (word >> 6 & 7) as u8,
            volume: (word >> 9 & 7) as u8,
            effect: (word >> 12 & 7) as u8,
            custom: word & 0x8000 != 0,
        }
    }

    pub const fn filters(self) -> u8 {
        self.bytes[FILTERS_BYTE]
    }

    pub const fn speed(self) -> u8 {
        self.bytes[SPEED_BYTE]
    }

    pub const fn loop_start(self) -> u8 {
        self.bytes[LOOP_START_BYTE]
    }

    pub const fn loop_end(self) -> u8 {
        self.bytes[LOOP_END_BYTE]
    }

    pub fn is_used(self) -> bool {
        self.bytes[..SFX_NOTES * 2].iter().any(|byte| *byte != 0)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slots {
    pub sfx: Vec<u8>,
    pub patterns: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pattern {
    bytes: [u8; PATTERN_BYTES],
}

impl Pattern {
    pub const fn channel(self, channel: usize) -> Option<u8> {
        let byte = self.bytes[channel];
        if byte & CHANNEL_DISABLED == 0 {
            Some(byte & SFX_MASK)
        } else {
            None
        }
    }

    pub fn is_empty(self) -> bool {
        (0..CHANNELS).all(|channel| self.channel(channel).is_none())
    }

    pub const fn loop_start(self) -> bool {
        self.bytes[0] & FLAG != 0
    }

    pub const fn loop_back(self) -> bool {
        self.bytes[1] & FLAG != 0
    }

    pub const fn stop(self) -> bool {
        self.bytes[2] & FLAG != 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cart {
    patterns: [[u8; PATTERN_BYTES]; SLOTS],
    sfx: [[u8; SFX_BYTES]; SLOTS],
}

impl Default for Cart {
    fn default() -> Self {
        let mut blank = [0; SFX_BYTES];
        blank[SPEED_BYTE] = OMITTED_ROW_SPEED;
        Self {
            patterns: [EMPTY_PATTERN; SLOTS],
            sfx: [blank; SLOTS],
        }
    }
}

impl Cart {
    pub fn read(bytes: &[u8]) -> Result<Self, CartError> {
        if bytes.starts_with(&PNG_SIGNATURE) {
            read_png(bytes)
        } else {
            read_text(bytes)
        }
    }

    pub fn new_cart_bytes() -> Vec<u8> {
        let header: &[u8] = &[TEXT_HEADER, NEW_CART_HEADER_TAIL].concat();
        std::iter::once(header)
            .chain(NEW_CART_LINES)
            .flat_map(|line| [line, b"\n"])
            .flatten()
            .copied()
            .collect()
    }

    pub fn sfx_bytes(&self, slot: u8) -> &[u8; 68] {
        &self.sfx[usize::from(slot)]
    }

    pub fn pattern_bytes(&self, slot: u8) -> [u8; 4] {
        self.patterns[usize::from(slot)]
    }

    pub fn set_sfx_bytes(&mut self, slot: u8, bytes: [u8; 68]) {
        self.sfx[usize::from(slot)] = bytes;
    }

    pub fn set_pattern_bytes(&mut self, slot: u8, bytes: [u8; 4]) {
        self.patterns[usize::from(slot)] = bytes;
    }

    pub fn sfx(&self, slot: u8) -> Sfx<'_> {
        Sfx {
            bytes: self.sfx_bytes(slot),
        }
    }

    pub fn pattern(&self, slot: u8) -> Pattern {
        Pattern {
            bytes: self.pattern_bytes(slot),
        }
    }

    pub fn claimed_bytes_hash(&self, claim: &Claim) -> String {
        let mut hash = String::new();
        for &slot in &claim.sfx {
            push_slot_hash(&mut hash, 'S', slot, self.sfx_bytes(slot));
        }
        if let Some((first, last)) = claim.first_and_last_pattern {
            for slot in first..=last {
                push_slot_hash(&mut hash, 'P', slot, &self.pattern_bytes(slot));
            }
        }
        hash
    }

    pub fn changed_since(&self, hash: &str) -> Slots {
        self.slots_in_hash(hash, false)
    }

    pub fn unchanged_since(&self, hash: &str) -> Slots {
        self.slots_in_hash(hash, true)
    }

    fn slots_in_hash(&self, hash: &str, unchanged: bool) -> Slots {
        let mut slots = Slots::default();
        for entry in hash.split_whitespace() {
            let Some((kind, slot, stored)) = slot_hash_entry(entry) else {
                continue;
            };
            let (bytes, list): (&[u8], _) = match kind {
                'S' => (self.sfx_bytes(slot), &mut slots.sfx),
                _ => (&self.pattern_bytes(slot), &mut slots.patterns),
            };
            if (slot_fingerprint(bytes) == stored) == unchanged {
                list.push(slot);
            }
        }
        slots.sfx.sort_unstable();
        slots.patterns.sort_unstable();
        slots
    }
}

fn slot_hash_entry(entry: &str) -> Option<(char, u8, u32)> {
    let (name, stored) = entry.split_once('=')?;
    let kind = name
        .chars()
        .next()
        .filter(|kind| matches!(kind, 'S' | 'P'))?;
    let slot = u8::from_str_radix(name.get(1..)?, 16)
        .ok()
        .filter(|slot| usize::from(*slot) < SLOTS)?;
    let stored = u32::from_str_radix(stored, 16).ok()?;
    Some((kind, slot, stored))
}

fn slot_fingerprint(bytes: &[u8]) -> u32 {
    bytes.iter().fold(FNV_OFFSET, |checksum, byte| {
        (checksum ^ u32::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn push_slot_hash(hash: &mut String, kind: char, slot: u8, bytes: &[u8]) {
    if !hash.is_empty() {
        hash.push(' ');
    }
    let fingerprint = slot_fingerprint(bytes);
    hash.push_str(&format!("{kind}{slot:02X}={fingerprint:08x}"));
}

fn read_png(bytes: &[u8]) -> Result<Cart, CartError> {
    let memory = decode_png(bytes).ok_or(CartError::NotACart)?;
    let mut cart = Cart::default();
    for (slot, pattern) in cart.patterns.iter_mut().enumerate() {
        let at = PATTERNS_AT + slot * PATTERN_BYTES;
        pattern.copy_from_slice(&memory[at..at + PATTERN_BYTES]);
    }
    for (slot, sfx) in cart.sfx.iter_mut().enumerate() {
        let at = SFX_AT + slot * SFX_BYTES;
        sfx.copy_from_slice(&memory[at..at + SFX_BYTES]);
    }
    Ok(cart)
}

fn decode_png(bytes: &[u8]) -> Option<Vec<u8>> {
    Some(
        png_pixels(bytes)?
            .as_chunks::<PNG_CHANNELS>()
            .0
            .iter()
            .take(MUSIC_END)
            .map(|pixel| pixel_byte(*pixel))
            .collect(),
    )
}

fn pixel_byte(pixel: [u8; PNG_CHANNELS]) -> u8 {
    let [r, g, b, a] = pixel.map(|bits| bits & PIXEL_DATA_BITS);
    a << 6 | r << 4 | g << 2 | b
}

fn png_pixels(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().ok()?;
    let info = reader.info();
    let is_cart = info.width == PNG_WIDTH
        && info.height == PNG_HEIGHT
        && info.color_type == png::ColorType::Rgba
        && info.bit_depth == png::BitDepth::Eight;
    if !is_cart {
        return None;
    }
    let mut pixels = vec![0; reader.output_buffer_size()?];
    reader.next_frame(&mut pixels).ok()?;
    Some(pixels)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Sfx,
    Music,
    Other,
}

fn read_text(bytes: &[u8]) -> Result<Cart, CartError> {
    let text = bytes.strip_prefix(BOM).unwrap_or(bytes);
    if !text.starts_with(TEXT_HEADER) {
        return Err(CartError::NotACart);
    }
    let mut cart = Cart::default();
    let mut section = Section::Other;
    let (mut sfx_rows, mut music_rows) = (0, 0);
    for (index, line) in text.split(|byte| *byte == b'\n').enumerate() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if is_section_mark(line) {
            section = match line {
                SFX_SECTION => Section::Sfx,
                MUSIC_SECTION => Section::Music,
                _ => Section::Other,
            };
            continue;
        }
        if line.is_empty() {
            continue;
        }
        let bad_line = CartError::BadLine { line: index + 1 };
        match section {
            Section::Sfx if sfx_rows < SLOTS => {
                cart.sfx[sfx_rows] = parse_sfx_line(line).ok_or(bad_line)?;
                sfx_rows += 1;
            }
            Section::Music if music_rows < SLOTS => {
                cart.patterns[music_rows] = parse_music_line(line).ok_or(bad_line)?;
                music_rows += 1;
            }
            _ => {}
        }
    }
    Ok(cart)
}

fn is_section_mark(line: &[u8]) -> bool {
    line.len() > 2 * SECTION_MARK.len()
        && line.starts_with(SECTION_MARK)
        && line.ends_with(SECTION_MARK)
}

fn hex_digits(text: &[u8]) -> Option<Vec<u8>> {
    text.iter()
        .map(|byte| char::from(*byte).to_digit(16).map(|digit| digit as u8))
        .collect()
}

fn parse_sfx_line(line: &[u8]) -> Option<[u8; SFX_BYTES]> {
    if line.len() != SFX_LINE_DIGITS {
        return None;
    }
    let digits = hex_digits(line)?;
    let mut sfx = [0; SFX_BYTES];
    for (index, byte) in sfx[FILTERS_BYTE..].iter_mut().enumerate() {
        *byte = digits[index * 2] << 4 | digits[index * 2 + 1];
    }
    let notes = digits[SFX_HEADER_DIGITS..].as_chunks::<NOTE_DIGITS>().0;
    for (index, note) in notes.iter().enumerate() {
        let pitch = u16::from(note[0] << 4 | note[1]) & 0x3f;
        let instrument = u16::from(note[2] & 7);
        let custom = u16::from(note[2] >> 3);
        let volume = u16::from(note[3] & 7);
        let effect = u16::from(note[4] & 7);
        let word = note_word(pitch, instrument, volume, effect, custom);
        sfx[index * 2..index * 2 + 2].copy_from_slice(&word.to_le_bytes());
    }
    Some(sfx)
}

fn parse_music_line(line: &[u8]) -> Option<[u8; PATTERN_BYTES]> {
    if line.len() != MUSIC_LINE_LENGTH || line[MUSIC_FLAGS_DIGITS] != b' ' {
        return None;
    }
    let flag_digits = hex_digits(&line[..MUSIC_FLAGS_DIGITS])?;
    let flags = flag_digits[0] << 4 | flag_digits[1];
    let digits = hex_digits(&line[MUSIC_FLAGS_DIGITS + 1..])?;
    let mut pattern = [0; PATTERN_BYTES];
    for (channel, byte) in pattern.iter_mut().enumerate() {
        let value = digits[channel * 2] << 4 | digits[channel * 2 + 1];
        *byte = value & !FLAG | (flags >> channel & 1) << 7;
    }
    Some(pattern)
}
