use super::cases::Plays;
use crate::support::{stored_instrument, stored_sound};
use pikutracker_core::cart::Cart;
use pikutracker_core::cart_import::import;
use pikutracker_core::player::{AudioRequest, PlayId, PlayTarget, Player};
use pikutracker_core::synth::Voice;
use std::sync::Arc;

const SFX_INSTRUMENTS: u8 = 8;
const CHANNEL_0_PHASE_AT_BOOT: u16 = 26_056;

pub fn render(cart: &Cart, plays: Plays, samples: usize) -> Vec<f32> {
    match plays {
        Plays::Sfx(slot) => {
            let mut voice = Voice::with_phase(CHANNEL_0_PHASE_AT_BOOT);
            for instrument in 0..SFX_INSTRUMENTS {
                voice.set_instrument(instrument, stored_instrument(cart, instrument));
            }
            voice.play(stored_sound(cart, slot), 0);
            (0..samples).map(|_| voice.next_sample()).collect()
        }
        Plays::Music => {
            let mut player = Player::new();
            player.handle(AudioRequest::Play {
                play: PlayId::FIRST,
                project: Arc::new(import(cart, "reference.p8").project),
                target: PlayTarget::Song { from_row: 0 },
            });
            player.render(samples)
        }
    }
}
