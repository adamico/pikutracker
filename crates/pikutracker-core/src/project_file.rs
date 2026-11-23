use crate::project::{
    CART_SLOTS, CHAIN_ENTRIES, ChainEntry, ChainId, Claim, CustomInstrument, Filters,
    InstrumentSound, MAX_FILTER_LEVEL, MAX_INSTRUMENT_NAME, MAX_STEPS, NOTE_NAMES, Phrase,
    PhraseId, Project, SONG_ROWS, SfxInstrument, Step, TRACKS, TargetCart, WAVEFORM_SAMPLES,
    WaveformInstrument, slot_of_custom_instrument,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const FORMAT_VERSION: u32 = 1;

pub type Migration = fn(&str) -> Result<String, String>;

const MIGRATIONS: [Migration; FORMAT_VERSION as usize - 1] = [];

const SILENT_TOKEN: &str = "--- - - -";
const EMPTY_CELL: &str = "--";
const SEMITONES_PER_OCTAVE: u8 = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectFileError {
    Newer { found: u32 },
    Damaged(String),
}

impl fmt::Display for ProjectFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Newer { found } => write!(
                formatter,
                "MADE BY A NEWER PIKUTRACKER: FORMAT {found}, THIS VERSION READS UP TO \
                 {FORMAT_VERSION}"
            ),
            Self::Damaged(reason) => write!(formatter, "NOT A READABLE PROJECT: {reason}"),
        }
    }
}

impl std::error::Error for ProjectFileError {}

fn damaged(reason: impl fmt::Display) -> ProjectFileError {
    ProjectFileError::Damaged(reason.to_string())
}

pub fn parse(text: &str) -> Result<Project, ProjectFileError> {
    let current = migrate(text, FORMAT_VERSION, &MIGRATIONS)?;
    let file: ProjectDocument = ron::from_str(&current).map_err(damaged)?;
    file.into_project().map_err(damaged)
}

pub fn migrate(
    text: &str,
    target: u32,
    migrations: &[Migration],
) -> Result<String, ProjectFileError> {
    let mut version = version_of(text)?;
    if version > target {
        return Err(ProjectFileError::Newer { found: version });
    }
    let mut text = text.to_owned();
    while version < target {
        let migration = migrations
            .get(version as usize - 1)
            .ok_or_else(|| damaged(format!("NO MIGRATION FROM FORMAT {version}")))?;
        text = migration(&text).map_err(damaged)?;
        let migrated = version_of(&text)?;
        if migrated != version + 1 {
            return Err(damaged(format!(
                "MIGRATING FORMAT {version} GAVE FORMAT {migrated}"
            )));
        }
        version = migrated;
    }
    Ok(text)
}

fn version_of(text: &str) -> Result<u32, ProjectFileError> {
    let header: VersionHeader = ron::from_str(text).map_err(damaged)?;
    if header.version == 0 {
        return Err(damaged("FORMAT VERSION 0 DOES NOT EXIST"));
    }
    Ok(header.version)
}

pub fn write(project: &Project) -> String {
    ron::ser::to_string_pretty(&ProjectDocument::from_project(project), PrettyConfig::new())
        .expect("a Project document always serialises")
}

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

#[derive(Serialize, Deserialize)]
struct ProjectDocument {
    version: u32,
    default_speed: u8,
    song_loop_row: Option<u16>,
    target_cart: Option<TargetCartDocument>,
    song: Vec<SongRowDocument>,
    chains: Vec<ChainDocument>,
    phrases: Vec<PhraseDocument>,
    instruments: Vec<InstrumentDocument>,
}

#[derive(Serialize, Deserialize)]
struct TargetCartDocument {
    path: String,
    claim: ClaimDocument,
    claimed_bytes_hash: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ClaimDocument {
    sfx: Vec<u8>,
    patterns: Option<PatternRangeDocument>,
}

#[derive(Serialize, Deserialize)]
struct PatternRangeDocument {
    first: u8,
    last: u8,
}

#[derive(Serialize, Deserialize)]
struct SongRowDocument {
    row: u16,
    tracks: String,
}

#[derive(Serialize, Deserialize)]
struct ChainDocument {
    id: u8,
    entries: Vec<EntryDocument>,
}

#[derive(Serialize, Deserialize)]
struct EntryDocument {
    index: u8,
    phrase: u8,
    transpose: i8,
}

#[derive(Serialize, Deserialize)]
struct PhraseDocument {
    id: u8,
    loop_start: u8,
    loop_end: u8,
    speed: u8,
    noiz: bool,
    buzz: bool,
    detune: u8,
    reverb: u8,
    dampen: u8,
    steps: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct InstrumentDocument {
    id: u8,
    name: Option<String>,
    sound: SoundDocument,
}

#[derive(Serialize, Deserialize)]
enum SoundDocument {
    Sfx(SfxDocument),
    Waveform(WaveformDocument),
}

#[derive(Serialize, Deserialize)]
struct SfxDocument {
    speed: u8,
    noiz: bool,
    buzz: bool,
    detune: u8,
    reverb: u8,
    dampen: u8,
    loop_start: u8,
    loop_end: u8,
    steps: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct WaveformDocument {
    bass: bool,
    detune: u8,
    reverb: u8,
    dampen: u8,
    samples: Vec<i8>,
}

impl ProjectDocument {
    fn from_project(project: &Project) -> Self {
        let blank_phrase = Phrase::default();
        let phrases = (0..=u8::MAX)
            .filter(|&id| *project.phrase(PhraseId::new(id)) != blank_phrase)
            .map(|id| PhraseDocument::from_phrase(id, project.phrase(PhraseId::new(id))))
            .collect();
        let chains = (0..=u8::MAX)
            .filter_map(|id| ChainDocument::from_chain(project, id))
            .collect();
        let song = (0..SONG_ROWS)
            .filter_map(|row| SongRowDocument::from_row(project, row))
            .collect();
        let instruments = project
            .custom_instruments()
            .map(|(id, instrument)| InstrumentDocument::from_instrument(id, instrument))
            .collect();
        Self {
            version: FORMAT_VERSION,
            default_speed: project.default_speed(),
            song_loop_row: project.song_loop_row().map(|row| row as u16),
            target_cart: project.target_cart().map(TargetCartDocument::from_cart),
            song,
            chains,
            phrases,
            instruments,
        }
    }

    fn into_project(self) -> Result<Project, String> {
        let mut project = Project::new();
        require(self.default_speed >= 1, "DEFAULT SPEED IS 0")?;
        project.set_default_speed(i32::from(self.default_speed));

        if let Some(row) = self.song_loop_row {
            require(
                usize::from(row) < SONG_ROWS,
                "SONG LOOP ROW IS OUT OF RANGE",
            )?;
        }
        project.set_song_loop_row(self.song_loop_row.map(usize::from));

        project.set_target_cart(
            self.target_cart
                .map(TargetCartDocument::into_cart)
                .transpose()?,
        );

        for row in self.song {
            row.read_into(&mut project)?;
        }
        for chain in self.chains {
            chain.read_into(&mut project)?;
        }
        for phrase in self.phrases {
            phrase.read_into(&mut project)?;
        }
        for instrument in self.instruments {
            instrument.read_into(&mut project)?;
        }
        Ok(project)
    }
}

fn require(condition: bool, problem: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(problem.to_owned())
    }
}

impl TargetCartDocument {
    fn from_cart(cart: &TargetCart) -> Self {
        Self {
            path: cart.path.clone(),
            claim: ClaimDocument {
                sfx: cart.claim.sfx.clone(),
                patterns: cart
                    .claim
                    .first_and_last_pattern
                    .map(|(first, last)| PatternRangeDocument { first, last }),
            },
            claimed_bytes_hash: cart.claimed_bytes_hash.clone(),
        }
    }

    fn into_cart(self) -> Result<TargetCart, String> {
        require(
            self.claim.sfx.iter().all(|&slot| slot < CART_SLOTS),
            "CLAIMED SFX SLOT IS OUT OF RANGE",
        )?;
        if let Some(range) = &self.claim.patterns {
            require(
                range.first <= range.last && range.last < CART_SLOTS,
                "CLAIMED PATTERN RANGE IS OUT OF RANGE",
            )?;
        }
        Ok(TargetCart {
            path: self.path,
            claim: Claim {
                sfx: self.claim.sfx,
                first_and_last_pattern: self.claim.patterns.map(|range| (range.first, range.last)),
            },
            claimed_bytes_hash: self.claimed_bytes_hash,
        })
    }
}

impl SongRowDocument {
    fn from_row(project: &Project, row: usize) -> Option<Self> {
        let cells: Vec<Option<ChainId>> = (0..TRACKS)
            .map(|track| project.song_cell(row, track))
            .collect();
        cells.iter().any(Option::is_some).then(|| Self {
            row: row as u16,
            tracks: cells
                .iter()
                .map(|cell| {
                    cell.map_or_else(
                        || EMPTY_CELL.to_owned(),
                        |chain| format!("{:02X}", chain.number()),
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
        })
    }

    fn read_into(self, project: &mut Project) -> Result<(), String> {
        let row = usize::from(self.row);
        require(row < SONG_ROWS, "SONG ROW IS OUT OF RANGE")?;
        let cells: Vec<&str> = self.tracks.split(' ').collect();
        require(cells.len() == TRACKS, "A SONG ROW NEEDS 4 TRACKS")?;
        for (track, cell) in cells.into_iter().enumerate() {
            let chain = if cell == EMPTY_CELL {
                None
            } else {
                Some(ChainId::new(parse_hex(cell).ok_or_else(|| {
                    format!("BAD CHAIN NUMBER {cell} ON SONG ROW {row:02X}")
                })?))
            };
            project.set_song_cell(row, track, chain);
        }
        Ok(())
    }
}

fn parse_hex(text: &str) -> Option<u8> {
    (text.len() == 2 && text.chars().all(|ch| ch.is_ascii_hexdigit()))
        .then(|| u8::from_str_radix(text, 16).ok())
        .flatten()
}

impl ChainDocument {
    fn from_chain(project: &Project, id: u8) -> Option<Self> {
        let chain = project.chain(ChainId::new(id));
        let entries: Vec<EntryDocument> = (0..CHAIN_ENTRIES)
            .filter_map(|index| {
                chain.entry(index).map(|entry| EntryDocument {
                    index: index as u8,
                    phrase: entry.phrase().number(),
                    transpose: entry.transpose(),
                })
            })
            .collect();
        (!entries.is_empty()).then_some(Self { id, entries })
    }

    fn read_into(self, project: &mut Project) -> Result<(), String> {
        for entry in self.entries {
            let index = usize::from(entry.index);
            require(index < CHAIN_ENTRIES, "CHAIN ENTRY IS OUT OF RANGE")?;
            let mut read = ChainEntry::new(PhraseId::new(entry.phrase));
            read.set_transpose(i32::from(entry.transpose));
            *project.chain_mut(ChainId::new(self.id)).entry_mut(index) = Some(read);
        }
        Ok(())
    }
}

impl PhraseDocument {
    fn from_phrase(id: u8, phrase: &Phrase) -> Self {
        let filters = phrase.filters();
        let last_written = (0..MAX_STEPS)
            .rev()
            .find(|&index| phrase.step(index) != Step::default())
            .map_or(0, |index| index + 1);
        Self {
            id,
            loop_start: phrase.loop_start(),
            loop_end: phrase.loop_end(),
            speed: phrase.speed(),
            noiz: filters.noiz(),
            buzz: filters.buzz(),
            detune: filters.detune(),
            reverb: filters.reverb(),
            dampen: filters.dampen(),
            steps: (0..last_written)
                .map(|index| step_token(phrase.step(index)))
                .collect(),
        }
    }

    fn read_into(self, project: &mut Project) -> Result<(), String> {
        let in_range = |value: u8| usize::from(value) <= MAX_STEPS;
        require(
            in_range(self.loop_start) && in_range(self.loop_end),
            "PHRASE LOOP IS OUT OF RANGE",
        )?;
        require(self.speed >= 1, "PHRASE SPEED IS 0")?;
        require(self.steps.len() <= MAX_STEPS, "PHRASE HAS TOO MANY STEPS")?;
        let filters = read_filters(self.noiz, self.buzz, self.detune, self.reverb, self.dampen)?;

        let phrase = project.phrase_mut(PhraseId::new(self.id));
        phrase.set_loop(i32::from(self.loop_start), i32::from(self.loop_end));
        phrase.set_speed(i32::from(self.speed));
        *phrase.filters_mut() = filters;
        for (index, token) in self.steps.iter().enumerate() {
            *phrase.step_mut(index) = parse_step(token)?;
        }
        Ok(())
    }
}

fn read_filters(
    noiz: bool,
    buzz: bool,
    detune: u8,
    reverb: u8,
    dampen: u8,
) -> Result<Filters, String> {
    for level in [detune, reverb, dampen] {
        require(
            i32::from(level) <= MAX_FILTER_LEVEL,
            "FILTER LEVEL IS OUT OF RANGE",
        )?;
    }
    let mut filters = Filters::default();
    filters.set_noiz(noiz);
    filters.set_buzz(buzz);
    filters.set_detune(i32::from(detune));
    filters.set_reverb(i32::from(reverb));
    filters.set_dampen(i32::from(dampen));
    Ok(filters)
}

impl InstrumentDocument {
    fn from_instrument(id: u8, instrument: &CustomInstrument) -> Self {
        let sound = match &instrument.sound {
            InstrumentSound::Sfx(sfx) => SoundDocument::Sfx(SfxDocument {
                speed: sfx.speed,
                noiz: sfx.filters.noiz(),
                buzz: sfx.filters.buzz(),
                detune: sfx.filters.detune(),
                reverb: sfx.filters.reverb(),
                dampen: sfx.filters.dampen(),
                loop_start: sfx.loop_start,
                loop_end: sfx.loop_end,
                steps: sfx.steps.iter().map(|step| step_token(*step)).collect(),
            }),
            InstrumentSound::Waveform(wave) => SoundDocument::Waveform(WaveformDocument {
                bass: wave.bass,
                detune: wave.detune,
                reverb: wave.reverb,
                dampen: wave.dampen,
                samples: wave.samples.to_vec(),
            }),
        };
        Self {
            id,
            name: instrument.name.clone(),
            sound,
        }
    }

    fn read_into(self, project: &mut Project) -> Result<(), String> {
        require(
            slot_of_custom_instrument(self.id).is_some(),
            "CUSTOM INSTRUMENT NUMBER IS OUT OF RANGE",
        )?;
        if let Some(name) = &self.name {
            require(
                name.chars().count() <= MAX_INSTRUMENT_NAME,
                "INSTRUMENT NAME IS TOO LONG",
            )?;
        }
        let sound = match self.sound {
            SoundDocument::Sfx(sfx) => InstrumentSound::Sfx(sfx.into_instrument()?),
            SoundDocument::Waveform(wave) => InstrumentSound::Waveform(wave.into_instrument()?),
        };
        project.set_custom_instrument(
            self.id,
            Some(CustomInstrument {
                name: self.name,
                sound,
            }),
        );
        Ok(())
    }
}

impl SfxDocument {
    fn into_instrument(self) -> Result<SfxInstrument, String> {
        require(
            (1..=MAX_STEPS).contains(&self.steps.len()),
            "SFX INSTRUMENT NEEDS 1 TO 32 STEPS",
        )?;
        require(self.speed >= 1, "SFX INSTRUMENT SPEED IS 0")?;
        require(
            usize::from(self.loop_start) <= MAX_STEPS && usize::from(self.loop_end) <= MAX_STEPS,
            "SFX INSTRUMENT LOOP IS OUT OF RANGE",
        )?;
        Ok(SfxInstrument {
            steps: self
                .steps
                .iter()
                .map(|token| parse_step(token))
                .collect::<Result<_, _>>()?,
            speed: self.speed,
            filters: read_filters(self.noiz, self.buzz, self.detune, self.reverb, self.dampen)?,
            loop_start: self.loop_start,
            loop_end: self.loop_end,
        })
    }
}

impl WaveformDocument {
    fn into_instrument(self) -> Result<WaveformInstrument, String> {
        let samples: [i8; WAVEFORM_SAMPLES] = self
            .samples
            .try_into()
            .map_err(|_| "WAVEFORM INSTRUMENT NEEDS 64 SAMPLES".to_owned())?;
        for level in [self.detune, self.reverb, self.dampen] {
            require(
                i32::from(level) <= MAX_FILTER_LEVEL,
                "FILTER LEVEL IS OUT OF RANGE",
            )?;
        }
        Ok(WaveformInstrument {
            samples,
            bass: self.bass,
            detune: self.detune,
            reverb: self.reverb,
            dampen: self.dampen,
        })
    }
}

fn step_token(step: Step) -> String {
    if step == zero_step() {
        return SILENT_TOKEN.to_owned();
    }
    let name = NOTE_NAMES[usize::from(step.pitch() % SEMITONES_PER_OCTAVE)];
    let octave = step.pitch() / SEMITONES_PER_OCTAVE;
    format!(
        "{name}{octave} {:X} {} {}",
        step.instrument(),
        step.volume(),
        step.effect()
    )
}

fn zero_step() -> Step {
    Step::new(0, 0, 0, 0).expect("zeros are inside every limit")
}

fn parse_step(token: &str) -> Result<Step, String> {
    if token == SILENT_TOKEN {
        return Ok(zero_step());
    }
    read_step(token).ok_or_else(|| format!("BAD STEP {token}"))
}

fn read_step(token: &str) -> Option<Step> {
    let mut parts = token.split(' ');
    let (note, instrument, volume, effect) =
        (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || note.len() != 3 || !note.is_ascii() {
        return None;
    }
    let (name, octave) = note.split_at(2);
    let semitone = NOTE_NAMES.iter().position(|known| *known == name)? as u8;
    let pitch = single_digit(octave)?.checked_mul(SEMITONES_PER_OCTAVE)? + semitone;
    Step::new(
        pitch,
        single_hex_digit(instrument)?,
        single_digit(volume)?,
        single_digit(effect)?,
    )
}

fn single_digit(text: &str) -> Option<u8> {
    single_digit_in_radix(text, 10)
}

fn single_hex_digit(text: &str) -> Option<u8> {
    single_digit_in_radix(text, 16)
}

fn single_digit_in_radix(text: &str, radix: u32) -> Option<u8> {
    let mut chars = text.chars();
    let digit = chars.next()?.to_digit(radix)? as u8;
    chars.next().is_none().then_some(digit)
}
