use crate::project::{
    ChainEntry, FIRST_CUSTOM_INSTRUMENT, Filters, InstrumentSound, MAX_STEPS, Phrase, PhraseId,
    Project, SfxInstrument, Step, TRACKS, WaveformInstrument,
};
use crate::synth::{
    Instrument, Instruments, Note, SAMPLE_RATE, SAMPLES_PER_TICK, Sound, Voice, Waveform,
    packed_filters,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioRequest {
    PlayPhrase {
        project: Arc<Project>,
        phrase: PhraseId,
        transpose: i8,
        from_step: usize,
    },
    PlayChain {
        project: Arc<Project>,
        song_row: usize,
        track: usize,
        from_entry: usize,
    },
    PlaySong {
        project: Arc<Project>,
        from_row: usize,
    },
    UpdateProject(Arc<Project>),
    SetTrackMix(TrackMix),
    PreviewStep {
        project: Arc<Project>,
        phrase: Phrase,
        step: usize,
        transpose: i8,
    },
    EndPreview,
    AuditionInstrument {
        project: Arc<Project>,
        instrument: u8,
    },
    Stop,
}

#[derive(Clone, Copy, Debug)]
struct Preview {
    samples_left: u64,
    released: bool,
}

const SATURATION_KNEE: f32 = 0.75;
const SATURATION_SLOPE: f32 = 1.0 / 25.0;

fn saturated(mix: f32) -> f32 {
    let past_knee = mix.abs() - SATURATION_KNEE;
    if past_knee > 0.0 {
        mix.signum() * (SATURATION_KNEE + past_knee * SATURATION_SLOPE)
    } else {
        mix
    }
}

const AUDITION_PITCH: u8 = 24;
const AUDITION_VOLUME: u8 = 5;
const AUDITION_SPEED: u8 = 255;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackMix {
    muted: [bool; TRACKS],
    soloed: [bool; TRACKS],
}

impl TrackMix {
    pub fn toggle_mute(&mut self, track: usize) {
        self.muted[track] = !self.muted[track];
    }

    pub fn toggle_solo(&mut self, track: usize) {
        self.soloed[track] = !self.soloed[track];
    }

    pub fn is_muted(&self, track: usize) -> bool {
        self.muted[track]
    }

    pub fn is_soloed(&self, track: usize) -> bool {
        self.soloed[track]
    }

    pub fn is_heard(&self, track: usize) -> bool {
        if self.soloed.contains(&true) {
            self.soloed[track]
        } else {
            !self.muted[track]
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Playhead {
    pub chain_row: Option<(usize, usize)>,
    pub tracks: [Option<(PhraseId, usize)>; TRACKS],
    pub waiting: [bool; TRACKS],
}

impl Playhead {
    pub fn step_in(&self, phrase: PhraseId, track: usize) -> Option<usize> {
        let step_on = |place: &Option<(PhraseId, usize)>| {
            place.filter(|(id, _)| *id == phrase).map(|(_, step)| step)
        };
        step_on(&self.tracks[track]).or_else(|| self.tracks.iter().find_map(step_on))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackReport {
    pub playhead: Option<Playhead>,
    pub finished_play: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    Song,
    Chain { track: usize },
    Phrase { phrase: PhraseId, transpose: i8 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LaneState {
    #[default]
    Idle,
    Playing,
    Finished,
}

#[derive(Clone, Copy, Debug, Default)]
struct Lane {
    state: LaneState,
    loops: bool,
    played_through: bool,
    step: usize,
    samples_left: u64,
}

impl Lane {
    fn starting(entry: Option<ChainEntry>, project: &Project, from_step: usize) -> Self {
        let Some(entry) = entry else {
            return Self::default();
        };
        let phrase = project.phrase(entry.phrase());
        Self {
            state: LaneState::Playing,
            loops: phrase.loops(),
            played_through: false,
            step: from_step.min(phrase.length() - 1),
            samples_left: step_samples(phrase),
        }
    }

    fn finish_step(&mut self, entry: Option<ChainEntry>, project: &Project, repeats: bool) {
        let Some(entry) = entry else {
            self.state = LaneState::Idle;
            return;
        };
        let phrase = project.phrase(entry.phrase());
        self.loops = phrase.loops();
        let repeat_from = phrase
            .phrase_loop()
            .map(|steps| steps.start)
            .or(repeats.then_some(0));
        if self.step + 1 < phrase.length() {
            self.step += 1;
        } else if let Some(start) = repeat_from {
            self.step = start;
            self.played_through = true;
        } else {
            self.state = LaneState::Finished;
            return;
        }
        self.samples_left = step_samples(phrase);
    }
}

fn step_samples(phrase: &Phrase) -> u64 {
    u64::from(phrase.speed()) * u64::from(SAMPLES_PER_TICK)
}

#[derive(Clone, Debug)]
struct Playback {
    project: Arc<Project>,
    route: Route,
    row: usize,
    k: usize,
    lanes: [Lane; TRACKS],
    waiting_for_edit: bool,
}

impl Playback {
    fn next_chain_row(&self) -> Option<(usize, usize)> {
        let project = &self.project;
        let later_in_row = first_chain_row(project, self.row, self.k + 1);
        match self.route {
            Route::Phrase { .. } => None,
            Route::Chain { .. } => later_in_row
                .or_else(|| first_chain_row(project, self.row, 0))
                .map(|k| (self.row, k)),
            Route::Song => {
                if let Some(k) = later_in_row {
                    return Some((self.row, k));
                }
                if project.song_row_is_empty(self.row) {
                    return None;
                }
                let row = project.song_row_after(self.row)?;
                Some((row, first_chain_row(project, row, 0)?))
            }
        }
    }

    fn entry(&self, track: usize) -> Option<ChainEntry> {
        match self.route {
            Route::Phrase { phrase, transpose } => (track == 0).then(|| {
                let mut entry = ChainEntry::new(phrase);
                entry.set_transpose(i32::from(transpose));
                entry
            }),
            _ => self.project.chain_row_entry(self.row, track, self.k),
        }
    }

    fn sounds(&self, track: usize) -> bool {
        match self.route {
            Route::Song => true,
            Route::Chain { track: alone } => track == alone,
            Route::Phrase { .. } => track == 0,
        }
    }

    fn sound(&self, track: usize) -> Option<Sound> {
        let entry = self.entry(track).filter(|_| self.sounds(track))?;
        Some(entry_sound(self.project.phrase(entry.phrase()), entry))
    }

    fn playhead(&self) -> Playhead {
        let mut tracks = [None; TRACKS];
        let mut waiting = [false; TRACKS];
        for track in (0..TRACKS).filter(|&track| self.sounds(track)) {
            let lane = self.lanes[track];
            match lane.state {
                LaneState::Playing => {
                    tracks[track] = self.entry(track).map(|entry| (entry.phrase(), lane.step));
                }
                LaneState::Finished => waiting[track] = true,
                LaneState::Idle => {}
            }
        }
        let chain_row = match self.route {
            Route::Phrase { .. } => None,
            _ => Some((self.row, self.k)),
        };
        Playhead {
            chain_row,
            tracks,
            waiting,
        }
    }

    fn chain_row_is_over(&self) -> bool {
        if matches!(self.route, Route::Phrase { .. }) {
            return false;
        }
        let mut plain = self
            .lanes
            .iter()
            .filter(|lane| lane.state != LaneState::Idle && !lane.loops)
            .peekable();
        if plain.peek().is_some() {
            return plain.all(|lane| lane.state == LaneState::Finished);
        }
        self.lanes
            .iter()
            .filter(|lane| lane.state == LaneState::Playing)
            .all(|lane| lane.played_through)
    }
}

fn first_chain_row(project: &Project, row: usize, from: usize) -> Option<usize> {
    project.played_chain_rows(row).find(|&k| k >= from)
}

#[derive(Clone, Debug)]
pub struct Player {
    voices: Vec<Voice>,
    playback: Option<Playback>,
    track_mix: TrackMix,
    preview: Option<Preview>,
    plays: u64,
    finished_play: u64,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            voices: vec![Voice::new(); TRACKS],
            playback: None,
            track_mix: TrackMix::default(),
            preview: None,
            plays: 0,
            finished_play: 0,
        }
    }
}

impl Player {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle(&mut self, request: AudioRequest) {
        match request {
            AudioRequest::PlayPhrase {
                project,
                phrase,
                transpose,
                from_step,
            } => {
                let route = Route::Phrase { phrase, transpose };
                self.start(project, route, 0, Some(0), from_step);
            }
            AudioRequest::PlayChain {
                project,
                song_row,
                track,
                from_entry,
            } => {
                let k = first_chain_row(&project, song_row, from_entry)
                    .or_else(|| first_chain_row(&project, song_row, 0));
                self.start(project, Route::Chain { track }, song_row, k, 0);
            }
            AudioRequest::PlaySong { project, from_row } => {
                let k = first_chain_row(&project, from_row, 0);
                self.start(project, Route::Song, from_row, k, 0);
            }
            AudioRequest::UpdateProject(project) => self.update_project(project),
            AudioRequest::SetTrackMix(track_mix) => self.track_mix = track_mix,
            AudioRequest::PreviewStep {
                project,
                phrase,
                step,
                transpose,
            } => {
                self.cut();
                self.hear_instruments_of(&project);
                self.start_preview(&phrase, step, transpose);
            }
            AudioRequest::EndPreview => {
                if let Some(preview) = self.preview.as_mut() {
                    preview.released = true;
                }
            }
            AudioRequest::AuditionInstrument {
                project,
                instrument,
            } => self.audition(&project, instrument),
            AudioRequest::Stop => self.cut(),
        }
    }

    pub fn finished_play(&self) -> u64 {
        self.finished_play
    }

    pub fn report(&self) -> PlaybackReport {
        PlaybackReport {
            playhead: self.playback.as_ref().map(Playback::playhead),
            finished_play: self.finished_play,
        }
    }

    fn cut(&mut self) {
        self.playback = None;
        self.preview = None;
        for voice in &mut self.voices {
            voice.stop();
        }
    }

    fn audition(&mut self, project: &Project, instrument: u8) {
        self.cut();
        self.plays += 1;
        self.hear_instruments_of(project);
        let note = Note {
            pitch: i16::from(AUDITION_PITCH),
            instrument,
            custom: true,
            volume: AUDITION_VOLUME,
            effect: 0,
        };
        self.voices[0].play(
            Sound {
                notes: [note; MAX_STEPS],
                speed: AUDITION_SPEED,
                filters: 0,
                loop_start: 0,
                loop_end: MAX_STEPS as u8,
            },
            0,
        );
    }

    fn hear_instruments_of(&mut self, project: &Project) {
        let instruments = Arc::new(instruments(project));
        for voice in &mut self.voices {
            voice.set_instruments(Arc::clone(&instruments));
        }
    }

    fn start_preview(&mut self, phrase: &Phrase, step: usize, transpose: i8) {
        let mut held = sound(phrase, transpose, 0, 1);
        held.notes[0] = held.notes[step];
        self.voices[0].play(held, 0);
        self.preview = Some(Preview {
            samples_left: step_samples(phrase),
            released: false,
        });
    }

    fn advance_preview(&mut self) {
        let Some(preview) = self.preview.as_mut() else {
            return;
        };
        preview.samples_left = preview.samples_left.saturating_sub(1);
        if preview.released && preview.samples_left == 0 {
            self.preview = None;
            self.voices[0].play(Sound::default(), 0);
        }
    }

    fn update_project(&mut self, project: Arc<Project>) {
        self.hear_instruments_of(&project);
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        playback.project = project;
        if playback.waiting_for_edit {
            self.leave_chain_row();
        }
    }

    fn start(
        &mut self,
        project: Arc<Project>,
        route: Route,
        row: usize,
        k: Option<usize>,
        from_step: usize,
    ) {
        self.cut();
        self.hear_instruments_of(&project);
        self.plays += 1;
        let Some(k) = k else {
            self.finished_play = self.plays;
            return;
        };
        let playback = Playback {
            project,
            route,
            row,
            k,
            lanes: [Lane::default(); TRACKS],
            waiting_for_edit: false,
        };
        self.enter_chain_row(playback, from_step);
    }

    fn enter_chain_row(&mut self, mut playback: Playback, from_step: usize) {
        for (track, voice) in self.voices.iter_mut().enumerate() {
            let lane = Lane::starting(playback.entry(track), &playback.project, from_step);
            playback.lanes[track] = lane;
            voice.play_restarting_if_silent(playback.sound(track).unwrap_or_default(), lane.step);
        }
        self.playback = Some(playback);
    }

    fn leave_chain_row(&mut self) {
        let Some(mut playback) = self.playback.take() else {
            return;
        };
        if let Some((row, k)) = playback.next_chain_row() {
            (playback.row, playback.k) = (row, k);
            playback.waiting_for_edit = false;
            self.enter_chain_row(playback, 0);
            return;
        }
        for voice in &mut self.voices {
            voice.play(Sound::default(), 0);
        }
        if matches!(playback.route, Route::Chain { .. }) {
            playback.lanes = [Lane::default(); TRACKS];
            playback.waiting_for_edit = true;
            self.playback = Some(playback);
        } else {
            self.finished_play = self.plays;
        }
    }

    fn advance(&mut self) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        let mut moved = [false; TRACKS];
        for (track, moved) in moved.iter_mut().enumerate() {
            let lane = playback.lanes[track];
            if lane.state == LaneState::Playing && lane.samples_left == 0 {
                let entry = playback.entry(track);
                let repeats = matches!(playback.route, Route::Phrase { .. });
                playback.lanes[track].finish_step(entry, &playback.project, repeats);
                *moved = true;
            }
        }
        if moved.contains(&true) {
            if playback.chain_row_is_over() {
                self.leave_chain_row();
            } else {
                self.start_next_steps(moved);
            }
        }
        if let Some(playback) = self.playback.as_mut() {
            for lane in &mut playback.lanes {
                if lane.state == LaneState::Playing {
                    lane.samples_left -= 1;
                }
            }
        }
    }

    fn start_next_steps(&mut self, moved: [bool; TRACKS]) {
        let Some(playback) = self.playback.as_ref() else {
            return;
        };
        for track in (0..TRACKS).filter(|&track| moved[track]) {
            let lane = playback.lanes[track];
            let sound = playback
                .sound(track)
                .filter(|_| lane.state == LaneState::Playing);
            match sound {
                Some(sound) => self.voices[track].continue_at(sound, lane.step),
                None => self.voices[track].rest(),
            }
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        self.advance();
        self.advance_preview();
        let respects_mix = self
            .playback
            .as_ref()
            .is_some_and(|playback| playback.route == Route::Song);
        let mut mix = 0.0;
        for (track, voice) in self.voices.iter_mut().enumerate() {
            let sample = voice.next_sample();
            if !respects_mix || self.track_mix.is_heard(track) {
                mix += sample;
            }
        }
        saturated(mix)
    }

    pub fn render(&mut self, samples: usize) -> Vec<f32> {
        (0..samples).map(|_| self.next_sample()).collect()
    }
}

fn entry_sound(phrase: &Phrase, entry: ChainEntry) -> Sound {
    match phrase.phrase_loop() {
        Some(_) => sound(
            phrase,
            entry.transpose(),
            phrase.loop_start(),
            phrase.loop_end(),
        ),
        None => sound(phrase, entry.transpose(), 0, phrase.length() as u8),
    }
}

fn sound(phrase: &Phrase, transpose: i8, loop_start: u8, loop_end: u8) -> Sound {
    let mut notes = [Note::default(); MAX_STEPS];
    for (index, note) in notes.iter_mut().enumerate() {
        let step = phrase.step(index);
        *note = Note {
            pitch: i16::from(step.pitch()) + i16::from(transpose),
            instrument: step.instrument(),
            custom: step.instrument() >= FIRST_CUSTOM_INSTRUMENT,
            volume: step.volume(),
            effect: step.effect(),
        };
    }
    Sound {
        notes,
        speed: phrase.speed(),
        filters: packed(phrase.filters()),
        loop_start,
        loop_end,
    }
}

fn packed(filters: Filters) -> u8 {
    packed_filters(
        filters.noiz(),
        filters.buzz(),
        filters.detune(),
        filters.reverb(),
        filters.dampen(),
    )
}

fn instruments(project: &Project) -> Instruments {
    project
        .custom_instruments()
        .map(|(id, instrument)| {
            let played = match &instrument.sound {
                InstrumentSound::Sfx(sfx) => Instrument::Sfx(sfx_instrument_sound(sfx)),
                InstrumentSound::Waveform(wave) => Instrument::Waveform(waveform(wave)),
            };
            (id, played)
        })
        .collect()
}

fn sfx_instrument_sound(sfx: &SfxInstrument) -> Sound {
    let mut notes = [Note::default(); MAX_STEPS];
    for (note, step) in notes.iter_mut().zip(&sfx.steps) {
        *note = built_in_note(*step);
    }
    let (loop_start, loop_end) = if sfx.loop_start < sfx.loop_end {
        (sfx.loop_start, sfx.loop_end)
    } else {
        (0, 0)
    };
    Sound {
        notes,
        speed: sfx.speed,
        filters: packed(sfx.filters),
        loop_start,
        loop_end,
    }
}

fn built_in_note(step: Step) -> Note {
    Note {
        pitch: i16::from(step.pitch()),
        instrument: step.instrument(),
        custom: false,
        volume: step.volume(),
        effect: step.effect(),
    }
}

fn waveform(wave: &WaveformInstrument) -> Waveform {
    Waveform {
        samples: wave.samples,
        bass: wave.bass,
        filters: packed_filters(false, false, wave.detune, wave.reverb, wave.dampen),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Resampler {
    step: f64,
    position: f64,
    previous: f32,
    next: f32,
}

impl Resampler {
    pub fn new(device_rate: u32) -> Self {
        Self {
            step: f64::from(SAMPLE_RATE) / f64::from(device_rate),
            position: 1.0,
            previous: 0.0,
            next: 0.0,
        }
    }

    pub fn next_frame(&mut self, mut source: impl FnMut() -> f32) -> f32 {
        self.position += self.step;
        while self.position >= 1.0 {
            self.position -= 1.0;
            self.previous = self.next;
            self.next = source();
        }
        self.previous + (self.next - self.previous) * self.position as f32
    }
}
