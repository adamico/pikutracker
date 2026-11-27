mod support;

use pikutracker_core::cart::Cart;
use pikutracker_core::cart_import::import;
use pikutracker_core::player::{AudioRequest, PlayId, PlayTarget, Player, TrackMix};
use pikutracker_core::project::Project;
use pikutracker_core::project_file;
use std::sync::Arc;
use support::{assert_same_samples, render_sounds, sound_of};

const P8: &[u8] = include_bytes!("fixtures/pico8/advent26.p8");
const PNG: &[u8] = include_bytes!("fixtures/pico8/advent26.p8.png");

const LEN: usize = 18;
const SPEED: u8 = 20;
const PATTERNS: usize = 7;

fn p8() -> Cart {
    Cart::read(P8).unwrap()
}

fn png() -> Cart {
    Cart::read(PNG).unwrap()
}

#[test]
fn both_formats_hold_the_same_sfx_and_patterns() {
    let (p8, png) = (p8(), png());

    for slot in 0..64 {
        assert_eq!(p8.sfx_bytes(slot), png.sfx_bytes(slot), "SFX {slot:02X}");
        assert_eq!(
            p8.pattern_bytes(slot),
            png.pattern_bytes(slot),
            "Pattern {slot:02X}"
        );
    }
}

#[test]
fn the_patterns_hold_the_bytes_pico8_saved() {
    for cart in [p8(), png()] {
        let patterns: Vec<[u8; 4]> = (0..8).map(|slot| cart.pattern_bytes(slot)).collect();

        let loop_start_with_disabled_channel_2_naming_sfx_0b = [0x81, 0x07, 0x4b, 0x44];
        let loop_back_in_bit_7_of_the_second_byte = [0x05, 0x8a, 0x43, 0x44];
        let row_the_p8_leaves_out = [0x41, 0x42, 0x43, 0x44];
        assert_eq!(
            patterns,
            [
                [0x00, 0x06, 0x43, 0x44],
                loop_start_with_disabled_channel_2_naming_sfx_0b,
                [0x02, 0x08, 0x43, 0x44],
                [0x01, 0x07, 0x43, 0x44],
                [0x03, 0x08, 0x43, 0x44],
                [0x04, 0x09, 0x43, 0x44],
                loop_back_in_bit_7_of_the_second_byte,
                row_the_p8_leaves_out,
            ]
        );
    }
}

#[test]
fn the_sfx_hold_the_bytes_pico8_saved() {
    for cart in [p8(), png()] {
        let sfx = cart.sfx_bytes(0);

        let (editor_mode, speed_20, len_18_as_loop_start, loop_end) = (0x01, 0x14, 0x12, 0x00);
        assert_eq!(
            sfx[64..],
            [editor_mode, speed_20, len_18_as_loop_start, loop_end]
        );
        let pitch_32_waveform_5_volume_3: u16 = 0x32 | 5 << 6 | 3 << 9;
        assert_eq!(sfx[..2], pitch_32_waveform_5_volume_3.to_le_bytes());
        let pitch_18_waveform_5_volume_5_fade_out: u16 = 0x18 | 5 << 6 | 5 << 9 | 5 << 12;
        let step = 2 * 0x11;
        assert_eq!(
            sfx[step..step + 2],
            pitch_18_waveform_5_volume_5_fade_out.to_le_bytes()
        );
    }
}

#[test]
fn import_gives_the_expected_project_from_either_format() {
    let from_p8 = import(&p8(), "advent26.p8");
    let from_png = import(&png(), "advent26.p8.png");

    assert!(from_p8.warnings.is_empty());
    assert!(from_png.warnings.is_empty());
    let text = project_file::write(&from_p8.project);
    assert_eq!(text, include_str!("snapshots/advent26.ptrk"));
    assert_eq!(
        project_file::write(&from_png.project),
        text.replace("\"advent26.p8\"", "\"advent26.p8.png\"")
    );
}

const CHANNEL_SFX: [[u8; PATTERNS]; 2] = [[0, 1, 2, 1, 3, 4, 5], [6, 7, 8, 7, 8, 9, 10]];

fn channel_of_cart(cart: &Cart, channel: usize) -> Vec<f32> {
    let steps: Vec<usize> = (0..LEN).collect();
    let sounds: Vec<_> = CHANNEL_SFX[channel]
        .iter()
        .map(|&slot| (sound_of(cart, slot), &steps[..]))
        .collect();
    assert!(sounds.iter().all(|(sound, _)| sound.speed == SPEED));
    render_sounds(&sounds)
}

fn track_of_song(project: Project, track: usize, samples: usize) -> Vec<f32> {
    let mut mix = TrackMix::default();
    mix.toggle_solo(track);
    let mut player = Player::new();
    player.handle(AudioRequest::SetTrackMix(mix));
    player.handle(AudioRequest::Play {
        play: PlayId::new(1),
        project: Arc::new(project),
        target: PlayTarget::Song { from_row: 0 },
    });
    player.render(samples)
}

#[test]
fn an_import_with_no_warnings_renders_the_samples_the_cart_plays() {
    let cart = p8();
    let imported = import(&cart, "advent26.p8");
    assert!(imported.warnings.is_empty());

    for channel in 0..CHANNEL_SFX.len() {
        let expected = channel_of_cart(&cart, channel);
        let rendered = track_of_song(imported.project.clone(), channel, expected.len());
        assert_same_samples(&rendered, &expected);
    }
}
