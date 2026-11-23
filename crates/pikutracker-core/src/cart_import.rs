mod instruments;
mod rebuild;

use crate::cart::{Cart, Pattern, SLOTS};
use crate::project::{
    Chain, ChainEntry, ChainId, Claim, Phrase, PhraseId, Project, SongPlace, SongRowEnding,
    SongRowMarks, TRACKS, TargetCart, custom_instrument_of_slot,
};
use crate::wording::counted;
use instruments::imported_instrument;
use rebuild::{RebuiltChannel, RebuiltPattern, has_bit_7, loose_phrase, rebuild_pattern};
use std::collections::BTreeSet;

const LAST_SLOT: u8 = SLOTS as u8 - 1;
const MAX_PHRASES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedCart {
    pub project: Project,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportWarning {
    pub text: String,
    pub place: Option<SongPlace>,
}

impl ImportWarning {
    fn general(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            place: None,
        }
    }

    fn at(place: SongPlace, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            place: Some(place),
        }
    }
}

#[derive(Default)]
struct SharedItems {
    phrases: Vec<Phrase>,
    chains: Vec<Chain>,
}

impl SharedItems {
    fn phrase(&mut self, project: &mut Project, phrase: Phrase) -> Option<PhraseId> {
        let known = self.phrases.iter().position(|known| *known == phrase);
        let index = match known {
            Some(index) => index,
            None if self.phrases.len() < MAX_PHRASES => {
                self.phrases.push(phrase);
                self.phrases.len() - 1
            }
            None => return None,
        };
        let id = PhraseId::new(index as u8);
        *project.phrase_mut(id) = phrase;
        Some(id)
    }

    fn chain(&mut self, project: &mut Project, chain: Chain) -> ChainId {
        let index = self.chains.iter().position(|known| *known == chain);
        let index = index.unwrap_or_else(|| {
            self.chains.push(chain);
            self.chains.len() - 1
        });
        let id = ChainId::new(index as u8);
        *project.chain_mut(id) = chain;
        id
    }
}

fn music_patterns(cart: &Cart) -> Option<(u8, u8)> {
    let mut non_empty = (0..=LAST_SLOT).filter(|&slot| !cart.pattern(slot).is_empty());
    let first = non_empty.next()?;
    Some((first, non_empty.next_back().unwrap_or(first)))
}

pub fn import(cart: &Cart, cart_path: &str) -> ImportedCart {
    let mut project = Project::new();

    let mut warnings = Vec::new();
    let mut claim = Claim::default();
    let mut shared = SharedItems::default();
    let mut instruments = BTreeSet::new();
    if let Some(patterns) = music_patterns(cart) {
        let builder = SongBuilder::new(
            cart,
            &mut project,
            &mut warnings,
            &mut shared,
            &mut instruments,
        );
        claim = builder.build(patterns);
    }
    let loose = LooseSfx::of(cart, &claim, &mut instruments);
    loose.keep(&mut project, &mut shared, &mut warnings);
    for slot in instruments {
        let id = custom_instrument_of_slot(slot);
        project.set_custom_instrument(id, Some(imported_instrument(cart, slot)));
    }

    let claimed_bytes_hash = Some(cart.claimed_bytes_hash(&claim));
    project.set_target_cart(Some(TargetCart {
        path: cart_path.to_owned(),
        claim,
        claimed_bytes_hash,
    }));
    ImportedCart { project, warnings }
}

struct LooseSfx {
    phrases: Vec<(u8, Phrase)>,
}

impl LooseSfx {
    fn of(cart: &Cart, claim: &Claim, instruments: &mut BTreeSet<u8>) -> Self {
        let unclaimed = (0..=LAST_SLOT).filter(|slot| !claim.sfx.contains(slot));
        let candidates: Vec<(u8, Phrase)> = unclaimed
            .filter_map(|slot| {
                let (phrase, used) = loose_phrase(cart.sfx(slot))?;
                instruments.extend(used);
                Some((slot, phrase))
            })
            .collect();
        let phrases = candidates
            .into_iter()
            .filter(|(slot, _)| !instruments.contains(slot))
            .collect();
        Self { phrases }
    }

    fn keep(
        self,
        project: &mut Project,
        shared: &mut SharedItems,
        warnings: &mut Vec<ImportWarning>,
    ) {
        let mut kept = Vec::new();
        let mut lost = Vec::new();
        for (slot, phrase) in self.phrases {
            match shared.phrase(project, phrase) {
                Some(_) => kept.push(slot),
                None => lost.push(slot),
            }
        }
        if !kept.is_empty() {
            let text = if kept.len() == 1 {
                "A LOOSE PHRASE OUTSIDE THE CLAIM"
            } else {
                "LOOSE PHRASES OUTSIDE THE CLAIM"
            };
            warnings.push(ImportWarning::general(format!(
                "SFX {} KEPT AS {text}.",
                slot_list(&kept)
            )));
        }
        if !lost.is_empty() {
            warnings.push(ImportWarning::general(format!(
                "CART NEEDS MORE THAN {MAX_PHRASES} PHRASES: SFX {} NOT KEPT.",
                slot_list(&lost)
            )));
        }
    }
}

fn slot_list(slots: &[u8]) -> String {
    let names: Vec<String> = slots.iter().map(|slot| format!("{slot:02X}")).collect();
    names.join(" ")
}

fn lost_step_warning(
    pattern: u8,
    is_last_row: bool,
    place: SongPlace,
    phrase: Option<(PhraseId, usize)>,
) -> ImportWarning {
    let track = track_name(place.track);
    let (lost, step) = match phrase {
        Some((id, length)) => (
            format!("{track} PHRASE {:02X} LOST ITS LAST STEP.", id.number()),
            length - 1,
        ),
        None => (format!("{track} LOST A STEP."), 0),
    };
    let ends = if is_last_row { "ENDS" } else { "SPLITS" };
    ImportWarning::at(
        SongPlace { step, ..place },
        format!("{} {ends} INSIDE A STEP: {lost}", pattern_name(pattern)),
    )
}

fn pattern_name(slot: u8) -> String {
    format!("PATTERN {slot:02X}")
}

fn marks_of(pattern: Pattern) -> SongRowMarks {
    let ending = if pattern.stop() {
        Some(SongRowEnding::Stop)
    } else if pattern.loop_back() {
        Some(SongRowEnding::LoopBack)
    } else {
        None
    };
    SongRowMarks {
        loop_start: pattern.loop_start(),
        ending,
    }
}

fn track_name(track: usize) -> String {
    format!("T{}", track + 1)
}

struct SongBuilder<'a> {
    cart: &'a Cart,
    project: &'a mut Project,
    warnings: &'a mut Vec<ImportWarning>,
    shared: &'a mut SharedItems,
    instruments: &'a mut BTreeSet<u8>,
    played: BTreeSet<u8>,
    spd_0_warned: BTreeSet<u8>,
    bit_7_warned: BTreeSet<u8>,
    chain_rows: usize,
    out_of_phrases: bool,
}

impl<'a> SongBuilder<'a> {
    fn new(
        cart: &'a Cart,
        project: &'a mut Project,
        warnings: &'a mut Vec<ImportWarning>,
        shared: &'a mut SharedItems,
        instruments: &'a mut BTreeSet<u8>,
    ) -> Self {
        Self {
            cart,
            project,
            warnings,
            shared,
            instruments,
            played: BTreeSet::new(),
            spd_0_warned: BTreeSet::new(),
            bit_7_warned: BTreeSet::new(),
            chain_rows: 0,
            out_of_phrases: false,
        }
    }

    fn build(mut self, patterns: (u8, u8)) -> Claim {
        let (first, last) = patterns;
        for (row, slot) in (first..=last).enumerate() {
            let pattern = self.cart.pattern(slot);
            if !pattern.is_empty() {
                self.add_pattern(row, slot);
            }
            self.project.set_song_row_marks(row, marks_of(pattern));
        }
        self.warn_about_the_whole_song(patterns);
        let mut claimed = self.played;
        claimed.extend(self.instruments.iter());
        Claim {
            sfx: claimed.into_iter().collect(),
            first_and_last_pattern: Some(patterns),
        }
    }

    fn add_pattern(&mut self, row: usize, slot: u8) {
        let pattern = self.cart.pattern(slot);
        let rebuilt = rebuild_pattern(self.cart, pattern);
        self.chain_rows += rebuilt.chain_rows.len();
        if rebuilt.only_bit_7 {
            let track = (0..TRACKS).find(|&track| pattern.channel(track).is_some());
            self.warnings.push(ImportWarning {
                text: format!(
                    "{} HOLDS ONLY BIT 7 SFX: THE CART STALLS HERE, THE EXPORTED SONG PLAYS \
                     THROUGH.",
                    pattern_name(slot)
                ),
                place: track.map(|track| SongPlace {
                    song_row: row,
                    track,
                    chain_row: 0,
                    step: 0,
                }),
            });
        }
        for track in 0..TRACKS {
            let mut chain = Chain::default();
            if let Some(played) = pattern.channel(track) {
                self.played.insert(played);
                self.warn_about_sfx(played, row, track);
                chain = self.chain_of(slot, row, track, &rebuilt);
            }
            let chain = self.shared.chain(self.project, chain);
            self.project.set_song_cell(row, track, Some(chain));
        }
    }

    fn warn_about_sfx(&mut self, played: u8, song_row: usize, track: usize) {
        let place = SongPlace {
            song_row,
            track,
            chain_row: 0,
            step: 0,
        };
        let sfx = self.cart.sfx(played);
        if sfx.speed() == 0 && self.spd_0_warned.insert(played) {
            self.warnings.push(ImportWarning::at(
                place,
                format!("SFX {played:02X} HAS SPD 0: SET TO SPEED 1, TIMING FOLLOWS PICO-8."),
            ));
        }
        if has_bit_7(sfx) && self.bit_7_warned.insert(played) {
            self.warnings.push(ImportWarning::at(
                place,
                format!(
                    "SFX {played:02X} HAS BIT 7 SET: IMPORTED AS A PLAIN PHRASE THAT NEVER SETS \
                     THE PATTERN LENGTH."
                ),
            ));
        }
    }

    fn chain_of(
        &mut self,
        slot: u8,
        song_row: usize,
        track: usize,
        rebuilt: &RebuiltPattern,
    ) -> Chain {
        let mut chain = Chain::default();
        let last_row = rebuilt.chain_rows.len() - 1;
        for (k, chain_row) in rebuilt.chain_rows.iter().enumerate() {
            let Some(channel) = &chain_row[track] else {
                continue;
            };
            let place = SongPlace {
                song_row,
                track,
                chain_row: k,
                step: 0,
            };
            let phrase = self.place_entry(&mut chain, k, channel);
            let arpeggio_changed = channel.entry.as_ref().and_then(|e| e.arpeggio_changed);
            if let Some(step) = arpeggio_changed {
                self.warnings.push(ImportWarning::at(
                    SongPlace { step, ..place },
                    format!(
                        "{} {} STEP {step:02X} PLAYS A DIFFERENT ARPEGGIO AFTER IMPORT.",
                        pattern_name(slot),
                        track_name(track)
                    ),
                ));
            }
            if channel.lost_step {
                let warning = lost_step_warning(slot, k == last_row, place, phrase);
                self.warnings.push(warning);
            }
            let sounds = channel
                .entry
                .as_ref()
                .is_some_and(|e| !e.phrase.is_silent());
            if channel.early_ticks > 0 && sounds {
                self.warnings.push(ImportWarning::at(
                    place,
                    format!(
                        "{} SPLITS INSIDE A STEP: {} PLAYS ITS LATER STEPS {} EARLY.",
                        pattern_name(slot),
                        track_name(track),
                        counted(channel.early_ticks as usize, "TICK")
                    ),
                ));
            }
        }
        chain
    }

    fn place_entry(
        &mut self,
        chain: &mut Chain,
        k: usize,
        channel: &RebuiltChannel,
    ) -> Option<(PhraseId, usize)> {
        let entry = channel.entry.as_ref()?;
        self.instruments.extend(entry.instruments.iter().copied());
        let Some(id) = self.shared.phrase(self.project, entry.phrase) else {
            self.out_of_phrases = true;
            return None;
        };
        *chain.entry_mut(k) = Some(ChainEntry::new(id));
        Some((id, entry.phrase.length()))
    }

    fn warn_about_the_whole_song(&mut self, patterns: (u8, u8)) {
        if self.out_of_phrases {
            self.warnings.push(ImportWarning::general(format!(
                "SONG NEEDS MORE THAN {MAX_PHRASES} PHRASES: SOME ENTRIES STAY EMPTY."
            )));
        }
        let claimed_patterns = usize::from(patterns.1 - patterns.0) + 1;
        let claimed_sfx = self.played.len();
        let needed_sfx = self.shared.phrases.len();
        if self.chain_rows > claimed_patterns || needed_sfx > claimed_sfx {
            self.warnings.push(ImportWarning::general(format!(
                "SONG NEEDS {} PATTERNS AND {needed_sfx} SFX BUT THE CLAIM HOLDS \
                 {claimed_patterns} AND {claimed_sfx}: WIDEN THE CLAIM BEFORE EXPORT.",
                self.chain_rows
            )));
        }
    }
}
