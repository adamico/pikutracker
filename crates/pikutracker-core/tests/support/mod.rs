#![allow(dead_code)]

use pikutracker_core::app::layout::Area;
use pikutracker_core::app::{Action, App, Launch, SettingsContents};
use pikutracker_core::cart::Cart;
use pikutracker_core::player::{AudioRequest, PlayId, PlayTarget, Player};
use pikutracker_core::project::Project;
use pikutracker_core::synth::{
    Instrument, Note, SAMPLES_PER_TICK, Sound, Voice, Waveform, packed_filters,
};
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub const HEADER: &str = "pico-8 cartridge // http://www.pico-8.com\nversion 43\n";

pub const PATTERNS_AT: usize = 0x3100;
pub const SFX_AT: usize = 0x3200;
const CART_BYTES: usize = 0x8020;
const PNG_WIDTH: u32 = 160;
const PNG_HEIGHT: u32 = 205;

pub const M8_STYLE: &str = "(version: 1, preset: M8Style)";

pub fn launched(mut app: App, config: &str) -> App {
    app.launch(Launch {
        config_file: Some(PathBuf::from("/config/pikutracker/config.ron")),
        config: SettingsContents::Bytes(config.as_bytes().to_vec()),
        state_file: None,
        state: SettingsContents::Missing,
        directory: PathBuf::from("."),
    });
    app
}

pub fn let_the_notice_fade(app: &mut App) {
    app.advance_time(Duration::from_secs(3));
}

pub fn apply(app: &mut App, actions: &[Action]) {
    for action in actions {
        app.apply(*action);
    }
}

pub fn pane_text(app: &App, area: Area, line: usize) -> String {
    let text: String = app
        .grid()
        .text_dump()
        .lines()
        .nth(area.row + line)
        .unwrap_or_default()
        .chars()
        .chain(std::iter::repeat(' '))
        .skip(area.column)
        .take(area.width)
        .collect();
    text.trim_end().to_owned()
}

pub fn status_line(app: &App) -> String {
    app.grid().text_dump().lines().last().unwrap().to_owned()
}

#[derive(Clone, Copy, Debug)]
pub struct TestNote {
    pub pitch: u8,
    pub instrument: u8,
    pub volume: u8,
    pub effect: u8,
    pub custom: bool,
}

pub const fn note(pitch: u8) -> TestNote {
    TestNote {
        pitch,
        instrument: 0,
        volume: 3,
        effect: 0,
        custom: false,
    }
}

const REST: TestNote = TestNote {
    pitch: 0,
    instrument: 0,
    volume: 0,
    effect: 0,
    custom: false,
};

#[derive(Clone, Copy, Debug)]
pub struct TestSfx {
    pub editor_and_filters: u8,
    pub speed: u8,
    pub loop_start: u8,
    pub loop_end: u8,
    pub notes: [TestNote; 32],
}

impl TestSfx {
    pub fn with_pitches(speed: u8, pitches: &[u8]) -> Self {
        let mut notes = [REST; 32];
        for (slot, pitch) in notes.iter_mut().zip(pitches) {
            *slot = note(*pitch);
        }
        Self {
            editor_and_filters: 0,
            speed,
            loop_start: 0,
            loop_end: 0,
            notes,
        }
    }

    pub fn line(&self) -> String {
        let mut line = format!(
            "{:02x}{:02x}{:02x}{:02x}",
            self.editor_and_filters, self.speed, self.loop_start, self.loop_end
        );
        for note in &self.notes {
            let instrument = note.instrument + if note.custom { 8 } else { 0 };
            write!(
                line,
                "{:02x}{:x}{:x}{:x}",
                note.pitch, instrument, note.volume, note.effect
            )
            .unwrap();
        }
        line
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(68);
        for note in &self.notes {
            let word = u16::from(note.pitch)
                | u16::from(note.instrument) << 6
                | u16::from(note.volume) << 9
                | u16::from(note.effect) << 12
                | u16::from(note.custom) << 15;
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend([
            self.editor_and_filters,
            self.speed,
            self.loop_start,
            self.loop_end,
        ]);
        bytes
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TestPattern {
    pub channels: [Option<u8>; 4],
    pub loop_start: bool,
    pub loop_back: bool,
    pub stop: bool,
}

impl TestPattern {
    pub const fn playing(channels: [Option<u8>; 4]) -> Self {
        Self {
            channels,
            loop_start: false,
            loop_back: false,
            stop: false,
        }
    }

    pub fn bytes(&self) -> [u8; 4] {
        let flags = [self.loop_start, self.loop_back, self.stop, false];
        let mut bytes = [0; 4];
        for (channel, byte) in bytes.iter_mut().enumerate() {
            let sfx = self.channels[channel].unwrap_or(0x41 + channel as u8);
            *byte = sfx | u8::from(flags[channel]) << 7;
        }
        bytes
    }

    pub fn line(&self) -> String {
        let flags =
            u8::from(self.loop_start) | u8::from(self.loop_back) << 1 | u8::from(self.stop) << 2;
        let [a, b, c, d] = self.bytes().map(|byte| byte & 0x7f);
        format!("{flags:02x} {a:02x}{b:02x}{c:02x}{d:02x}")
    }
}

#[derive(Clone, Debug, Default)]
pub struct TestCart {
    pub sfx: Vec<TestSfx>,
    pub patterns: Vec<TestPattern>,
}

impl TestCart {
    pub fn p8(&self) -> Vec<u8> {
        let mut text = format!("{HEADER}__lua__\nprint(\"hi\")\n");
        if !self.sfx.is_empty() {
            text.push_str("__sfx__\n");
            for sfx in &self.sfx {
                text.push_str(&sfx.line());
                text.push('\n');
            }
        }
        if !self.patterns.is_empty() {
            text.push_str("__music__\n");
            for pattern in &self.patterns {
                text.push_str(&pattern.line());
                text.push('\n');
            }
        }
        text.into_bytes()
    }

    pub fn memory(&self) -> Vec<u8> {
        let mut memory = vec![0; CART_BYTES];
        let blank = TestSfx::with_pitches(16, &[]);
        for slot in 0..64 {
            let sfx = self.sfx.get(slot).unwrap_or(&blank);
            memory[SFX_AT + slot * 68..SFX_AT + (slot + 1) * 68].copy_from_slice(&sfx.bytes());
            let pattern = self.patterns.get(slot).copied().unwrap_or_default();
            memory[PATTERNS_AT + slot * 4..PATTERNS_AT + (slot + 1) * 4]
                .copy_from_slice(&pattern.bytes());
        }
        memory
    }

    pub fn png(&self) -> Vec<u8> {
        png_of(&self.memory(), PNG_WIDTH, PNG_HEIGHT)
    }
}

pub fn sound_of(cart: &Cart, slot: u8) -> Sound {
    let sfx = cart.sfx(slot);
    let mut notes = [Note::default(); 32];
    for (index, note) in notes.iter_mut().enumerate() {
        let stored = sfx.note(index);
        *note = Note {
            pitch: i16::from(stored.pitch),
            instrument: stored.instrument,
            custom: stored.custom,
            volume: stored.volume,
            effect: stored.effect,
        };
    }
    let byte = sfx.filters();
    Sound {
        notes,
        speed: sfx.speed(),
        filters: packed_filters(
            byte & 2 != 0,
            byte & 4 != 0,
            (byte / 8) % 3,
            (byte / 24) % 3,
            (byte / 72) % 3,
        ),
        loop_start: 0,
        loop_end: 32,
    }
}

pub fn render_notes(sound: Sound, notes: &[usize]) -> Vec<f32> {
    render_sounds(&[(sound, notes)])
}

pub fn render_sounds(sounds: &[(Sound, &[usize])]) -> Vec<f32> {
    let mut voice = Voice::new();
    let mut samples = Vec::new();
    for (sound, notes) in sounds {
        let step_samples = usize::from(sound.speed.max(1)) * SAMPLES_PER_TICK as usize;
        for (step, note) in notes.iter().enumerate() {
            if step == 0 {
                voice.play_restarting_if_silent(*sound, *note);
            } else {
                voice.continue_at(*sound, *note);
            }
            samples.extend((0..step_samples).map(|_| voice.next_sample()));
        }
    }
    samples
}

pub fn play_along(app: &mut App, player: &mut Player, samples: usize) -> Vec<f32> {
    for request in app.take_audio_requests() {
        player.handle(request);
    }
    let rendered = player.render(samples);
    app.show_playback(&player.report());
    rendered
}

pub fn render_song(project: Project, samples: usize) -> Vec<f32> {
    let mut player = Player::new();
    player.handle(AudioRequest::Play {
        play: PlayId::new(1),
        project: Arc::new(project),
        target: PlayTarget::Song { from_row: 0 },
    });
    player.render(samples)
}

pub fn assert_same_samples(rendered: &[f32], expected: &[f32]) {
    assert_eq!(rendered.len(), expected.len());
    assert!(expected.iter().any(|sample| sample.abs() > 0.01));
    let first_difference = expected
        .iter()
        .zip(rendered)
        .position(|(expected, rendered)| expected != rendered);
    assert_eq!(first_difference, None);
}

pub fn png_of(memory: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for index in 0..(width * height) as usize {
        let byte = memory.get(index).copied().unwrap_or(0);
        pixels.extend([
            0x80 | (byte >> 4 & 3),
            0x40 | (byte >> 2 & 3),
            0xc0 | (byte & 3),
            0xfc | (byte >> 6),
        ]);
    }
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    file
}

pub fn state_ron(recent_projects: &[&str]) -> String {
    let paths: Vec<String> = recent_projects
        .iter()
        .map(|path| format!("\"{path}\""))
        .collect();
    format!("(version: 1, recent_projects: [{}])", paths.join(", "))
}

const SFX_NOTES: usize = 32;
const EDITOR_MODE_BIT: u8 = 0x01;
const WAVEFORM_INSTRUMENT_BIT: u8 = 0x80;
const BASS_BIT: u8 = 0x01;
const INSTRUMENT_SLOTS: u8 = 8;
const SLOT_COUNT: u8 = 64;

pub fn stored_sound(cart: &Cart, slot: u8) -> Sound {
    let sfx = cart.sfx(slot);
    Sound {
        notes: std::array::from_fn(|index| {
            let stored = sfx.note(index);
            Note {
                pitch: i16::from(stored.pitch),
                instrument: stored.instrument,
                custom: stored.custom,
                volume: stored.volume,
                effect: stored.effect,
            }
        }),
        speed: sfx.speed(),
        filters: sfx.filters() & !EDITOR_MODE_BIT,
        loop_start: sfx.loop_start(),
        loop_end: sfx.loop_end(),
    }
}

pub fn stored_instrument(cart: &Cart, slot: u8) -> Instrument {
    let sfx = cart.sfx(slot);
    if sfx.loop_start() & WAVEFORM_INSTRUMENT_BIT == 0 {
        return Instrument::Sfx(stored_sound(cart, slot));
    }
    let bytes = cart.sfx_bytes(slot);
    Instrument::Waveform(Waveform {
        samples: std::array::from_fn(|index| bytes[index] as i8),
        bass: sfx.speed() & BASS_BIT != 0,
        filters: sfx.filters() & !EDITOR_MODE_BIT,
    })
}

const SATURATION_KNEE: f32 = 0.75;
const SATURATION_SLOPE: f32 = 1.0 / 25.0;

fn pico8_mix(voices: &mut [Voice]) -> f32 {
    let mix: f32 = voices.iter_mut().map(Voice::next_sample).sum();
    let past_knee = mix.abs() - SATURATION_KNEE;
    if past_knee > 0.0 {
        mix.signum() * (SATURATION_KNEE + past_knee * SATURATION_SLOPE)
    } else {
        mix
    }
}

struct MusicChannel {
    sound: Sound,
    note: usize,
    ended: bool,
}

impl MusicChannel {
    fn voice_sound(&self) -> Sound {
        if self.loops() {
            self.sound
        } else {
            Sound {
                loop_start: 0,
                loop_end: SFX_NOTES as u8,
                ..self.sound
            }
        }
    }

    fn loops(&self) -> bool {
        self.sound.loop_end > self.sound.loop_start
    }

    fn length(&self) -> usize {
        match (self.sound.loop_start, self.sound.loop_end) {
            (start, 0) if start > 0 => usize::from(start),
            _ => SFX_NOTES,
        }
    }
}

fn pattern_ticks(channels: &[Option<MusicChannel>]) -> usize {
    let playing = || channels.iter().flatten();
    playing().find(|channel| !channel.loops()).map_or_else(
        || {
            playing()
                .map(|channel| SFX_NOTES * usize::from(channel.sound.speed))
                .max()
                .unwrap_or(0)
        },
        |channel| channel.length() * usize::from(channel.sound.speed),
    )
}

fn next_pattern(cart: &Cart, slot: u8) -> Option<u8> {
    let pattern = cart.pattern(slot);
    if pattern.stop() || slot + 1 == SLOT_COUNT {
        None
    } else if pattern.loop_back() {
        let loop_start = (0..=slot)
            .rev()
            .find(|&slot| cart.pattern(slot).loop_start());
        Some(loop_start.unwrap_or(0))
    } else {
        Some(slot + 1)
    }
}

fn music_channels(cart: &Cart, slot: u8) -> Option<Vec<Option<MusicChannel>>> {
    let pattern = cart.pattern(slot);
    let channels: Vec<Option<MusicChannel>> = (0..4)
        .map(|channel| {
            pattern.channel(channel).map(|sfx| MusicChannel {
                sound: stored_sound(cart, sfx),
                note: 0,
                ended: false,
            })
        })
        .collect();
    channels.iter().any(Option::is_some).then_some(channels)
}

pub fn music_samples(cart: &Cart, first_pattern: u8) -> usize {
    let mut ticks = 0;
    let mut next = Some(first_pattern);
    while let Some(slot) = next {
        let Some(channels) = music_channels(cart, slot) else {
            break;
        };
        ticks += pattern_ticks(&channels);
        next = next_pattern(cart, slot);
    }
    ticks * SAMPLES_PER_TICK as usize
}

pub fn render_music(cart: &Cart, first_pattern: u8, samples: usize) -> Vec<f32> {
    let mut voices = vec![Voice::new(); 4];
    for voice in &mut voices {
        for slot in 0..INSTRUMENT_SLOTS {
            voice.set_instrument(slot, stored_instrument(cart, slot));
        }
    }
    let mut output = Vec::with_capacity(samples);
    let mut next = Some(first_pattern);
    while output.len() < samples {
        let Some(mut channels) = next.and_then(|slot| music_channels(cart, slot)) else {
            for voice in &mut voices {
                voice.play(Sound::default(), 0);
            }
            while output.len() < samples {
                output.push(pico8_mix(&mut voices));
            }
            break;
        };
        for (voice, channel) in voices.iter_mut().zip(&channels) {
            let sound = channel
                .as_ref()
                .map_or_else(Sound::default, |channel| channel.sound);
            voice.play_restarting_if_silent(sound, 0);
        }
        let ticks = pattern_ticks(&channels);
        for tick in 1..=ticks {
            for _ in 0..SAMPLES_PER_TICK {
                output.push(pico8_mix(&mut voices));
            }
            if tick == ticks {
                break;
            }
            for (voice, channel) in voices.iter_mut().zip(&mut channels) {
                let Some(channel) = channel.as_mut().filter(|channel| !channel.ended) else {
                    continue;
                };
                if tick % usize::from(channel.sound.speed) != 0 {
                    continue;
                }
                channel.note += 1;
                if channel.loops() && channel.note >= usize::from(channel.sound.loop_end) {
                    channel.note = usize::from(channel.sound.loop_start);
                }
                if !channel.loops() && channel.note >= channel.length() {
                    channel.ended = true;
                    voice.rest();
                } else if channel.note >= SFX_NOTES {
                    let silence = Sound {
                        notes: [Note::default(); SFX_NOTES],
                        ..channel.sound
                    };
                    voice.continue_at(silence, 0);
                } else {
                    voice.continue_at(channel.voice_sound(), channel.note);
                }
            }
        }
        next = next.and_then(|slot| next_pattern(cart, slot));
    }
    output.truncate(samples);
    output
}
