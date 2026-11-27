use crate::player::{AudioRequest, PlayId, PlaybackReport, Playhead, TrackMix};
use crate::project::{ChainId, Phrase, PhraseId, Project};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlayTarget {
    Song {
        from_row: usize,
    },
    ChainInSongRow {
        song_row: usize,
        track: usize,
        from_entry: usize,
    },
    ChainAlone {
        chain: ChainId,
        from_entry: usize,
    },
    Phrase {
        phrase: PhraseId,
        transpose: i8,
        from_step: usize,
    },
    Audition {
        instrument: u8,
    },
}

#[derive(Debug)]
pub(super) struct Transport {
    playing: bool,
    latest_play: Option<PlayId>,
    heard_project: Arc<Project>,
    track_mix: TrackMix,
    previewing: bool,
    playhead: Option<Playhead>,
    requests: Vec<AudioRequest>,
}

impl Transport {
    pub(super) fn new(project: &Project) -> Self {
        Self {
            playing: false,
            latest_play: None,
            heard_project: Arc::new(project.clone()),
            track_mix: TrackMix::default(),
            previewing: false,
            playhead: None,
            requests: Vec::new(),
        }
    }

    pub(super) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(super) fn playhead(&self) -> Option<Playhead> {
        self.playhead
    }

    pub(super) fn track_mix(&self) -> &TrackMix {
        &self.track_mix
    }

    pub(super) fn take_requests(&mut self) -> Vec<AudioRequest> {
        std::mem::take(&mut self.requests)
    }

    pub(super) fn play(&mut self, target: PlayTarget, project: &Project) {
        let play = self.latest_play.map_or(PlayId::FIRST, PlayId::next);
        let project = self.hear(project);
        let request = match target {
            PlayTarget::Song { from_row } => AudioRequest::PlaySong {
                play,
                project,
                from_row,
            },
            PlayTarget::ChainInSongRow {
                song_row,
                track,
                from_entry,
            } => AudioRequest::PlayChain {
                play,
                project,
                song_row,
                track,
                from_entry,
            },
            PlayTarget::ChainAlone { chain, from_entry } => AudioRequest::PlayChainAlone {
                play,
                project,
                chain,
                from_entry,
            },
            PlayTarget::Phrase {
                phrase,
                transpose,
                from_step,
            } => AudioRequest::PlayPhrase {
                play,
                project,
                phrase,
                transpose,
                from_step,
            },
            PlayTarget::Audition { instrument } => AudioRequest::AuditionInstrument {
                play,
                project,
                instrument,
            },
        };
        self.playing = true;
        self.previewing = false;
        self.latest_play = Some(play);
        self.requests.push(request);
    }

    pub(super) fn stop(&mut self) {
        if self.playing || self.previewing {
            self.requests.push(AudioRequest::Stop);
        }
        self.playing = false;
        self.previewing = false;
        self.playhead = None;
    }

    pub(super) fn reset(&mut self) {
        self.stop();
        if self.track_mix != TrackMix::default() {
            self.clear_track_mix();
        }
    }

    pub(super) fn clear_track_mix(&mut self) {
        self.change_track_mix(|mix| *mix = TrackMix::default());
    }

    pub(super) fn change_track_mix(&mut self, change: impl FnOnce(&mut TrackMix)) {
        change(&mut self.track_mix);
        self.requests
            .push(AudioRequest::SetTrackMix(self.track_mix));
    }

    pub(super) fn preview(
        &mut self,
        project: &Project,
        phrase: Phrase,
        step: usize,
        transpose: i8,
    ) {
        if self.playing {
            return;
        }
        self.previewing = true;
        let project = self.hear(project);
        self.requests.push(AudioRequest::PreviewStep {
            project,
            phrase,
            step,
            transpose,
        });
    }

    pub(super) fn end_preview(&mut self) {
        if self.previewing {
            self.previewing = false;
            self.requests.push(AudioRequest::EndPreview);
        }
    }

    pub(super) fn project_changed(&mut self, project: &Project) {
        if self.playing && *self.heard_project != *project {
            let project = self.hear(project);
            self.requests.push(AudioRequest::UpdateProject(project));
        }
    }

    pub(super) fn receive(&mut self, report: &PlaybackReport) -> bool {
        if report.finished_play.is_some() && report.finished_play == self.latest_play {
            self.playing = false;
        }
        let playhead = report.playhead.filter(|_| self.playing);
        let moved = playhead != self.playhead;
        self.playhead = playhead;
        moved
    }

    fn hear(&mut self, project: &Project) -> Arc<Project> {
        self.heard_project = Arc::new(project.clone());
        Arc::clone(&self.heard_project)
    }
}
