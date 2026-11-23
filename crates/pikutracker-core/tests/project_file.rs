use pikutracker_core::project::{
    ChainEntry, ChainId, Claim, CustomInstrument, Filters, InstrumentSound, PhraseId, Project,
    SfxInstrument, Step, TargetCart, WaveformInstrument,
};
use pikutracker_core::project_file::{self, FORMAT_VERSION, Migration, ProjectFileError};

const FIXTURE: &str = include_str!("fixtures/project_v1.ptrk");

fn step(pitch: u8, instrument: u8, volume: u8, effect: u8) -> Step {
    Step::new(pitch, instrument, volume, effect).unwrap()
}

fn fixture_project() -> Project {
    let mut project = Project::new();
    project.set_default_speed(12);
    project.set_song_loop_row(Some(1));
    project.set_target_cart(Some(TargetCart {
        path: "../carts/game.p8".to_owned(),
        claim: Claim {
            sfx: vec![3, 4, 50],
            first_and_last_pattern: Some((24, 29)),
        },
        claimed_bytes_hash: Some("5d41402abc4b2a76b9719d911017c592".to_owned()),
    }));

    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    project.set_song_cell(0, 3, Some(ChainId::new(1)));
    project.set_song_cell(2, 3, Some(ChainId::new(0xFF)));

    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    let mut lowered = ChainEntry::new(PhraseId::new(1));
    lowered.set_transpose(-12);
    *project.chain_mut(ChainId::new(0)).entry_mut(15) = Some(lowered);
    let mut raised = ChainEntry::new(PhraseId::new(0xFF));
    raised.set_transpose(127);
    *project.chain_mut(ChainId::new(1)).entry_mut(3) = Some(raised);

    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_length(3);
    phrase.set_speed(8);
    phrase.filters_mut().set_noiz(true);
    phrase.filters_mut().set_detune(1);
    phrase.filters_mut().set_reverb(2);
    *phrase.step_mut(0) = step(25, 8, 5, 3);
    *phrase.step_mut(1) = step(25, 8, 0, 3);
    *phrase.step_mut(2) = step(0, 0, 0, 0);
    *phrase.step_mut(3) = step(63, 0xF, 7, 7);

    let phrase = project.phrase_mut(PhraseId::new(1));
    phrase.set_loop(4, 32);
    phrase.set_speed(255);
    phrase.filters_mut().set_buzz(true);
    phrase.filters_mut().set_dampen(2);
    *phrase.step_mut(0) = step(0, 0, 1, 0);

    let phrase = project.phrase_mut(PhraseId::new(0xFF));
    phrase.set_length(1);
    phrase.set_speed(1);
    phrase.filters_mut().set_detune(2);
    phrase.filters_mut().set_reverb(1);
    phrase.filters_mut().set_dampen(1);

    let mut filters = Filters::default();
    filters.set_noiz(true);
    filters.set_buzz(true);
    filters.set_detune(2);
    filters.set_reverb(1);
    project.set_custom_instrument(
        8,
        Some(CustomInstrument {
            name: Some("BASS".to_owned()),
            sound: InstrumentSound::Sfx(SfxInstrument {
                steps: vec![step(24, 3, 5, 0), step(31, 3, 4, 1)],
                speed: 4,
                filters,
                loop_start: 1,
                loop_end: 2,
            }),
        }),
    );

    let mut samples = [0_i8; 64];
    samples[0] = -128;
    samples[1] = 127;
    for (index, sample) in samples.iter_mut().enumerate().skip(2) {
        *sample = index as i8 - 2;
    }
    project.set_custom_instrument(
        0xF,
        Some(CustomInstrument {
            name: None,
            sound: InstrumentSound::Waveform(WaveformInstrument {
                samples,
                bass: true,
                detune: 1,
                reverb: 0,
                dampen: 2,
            }),
        }),
    );
    project
}

fn fixture_with(from: &str, to: &str) -> String {
    assert!(FIXTURE.contains(from), "the fixture has no {from}");
    FIXTURE.replacen(from, to, 1)
}

#[test]
fn the_comprehensive_fixture_loads_to_the_expected_project() {
    assert_eq!(project_file::parse(FIXTURE).unwrap(), fixture_project());
}

#[test]
fn the_format_version_comes_first_as_an_integer() {
    let text = project_file::write(&Project::new());

    assert_eq!(
        text.lines().nth(1).map(str::trim),
        Some(format!("version: {FORMAT_VERSION},").as_str())
    );
}

#[test]
fn steps_are_written_as_tracker_tokens() {
    let mut project = Project::new();
    *project.phrase_mut(PhraseId::new(0)).step_mut(0) = step(25, 8, 5, 3);

    assert!(project_file::write(&project).contains("\"C#2 8 5 3\""));
}

#[test]
fn a_silent_step_holding_data_is_written_in_full() {
    let mut project = Project::new();
    *project.phrase_mut(PhraseId::new(0)).step_mut(0) = step(25, 8, 0, 3);

    let text = project_file::write(&project);

    assert!(text.contains("\"C#2 8 0 3\""));
    assert_eq!(
        project_file::parse(&text)
            .unwrap()
            .phrase(PhraseId::new(0))
            .step(0),
        step(25, 8, 0, 3)
    );
}

#[test]
fn only_a_step_of_all_zeros_is_written_as_dashes() {
    let mut project = Project::new();
    *project.phrase_mut(PhraseId::new(0)).step_mut(0) = step(0, 0, 0, 0);
    *project.phrase_mut(PhraseId::new(0)).step_mut(1) = step(0, 0, 0, 1);

    let text = project_file::write(&project);

    assert!(text.contains("\"--- - - -\""));
    assert!(text.contains("\"C-0 0 0 1\""));
}

#[test]
fn the_dash_token_reads_back_as_a_step_of_all_zeros() {
    let project = project_file::parse(FIXTURE).unwrap();

    assert_eq!(project.phrase(PhraseId::new(0)).step(2), step(0, 0, 0, 0));
}

#[test]
fn an_empty_project_survives_a_save_and_reload() {
    let project = Project::new();

    assert_eq!(
        project_file::parse(&project_file::write(&project)).unwrap(),
        project
    );
}

#[test]
fn the_fixture_project_survives_a_save_and_reload() {
    let project = fixture_project();

    assert_eq!(
        project_file::parse(&project_file::write(&project)).unwrap(),
        project
    );
}

struct SeededRandom(u64);

impl SeededRandom {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, limit: u16) -> u8 {
        (self.next() % u64::from(limit)) as u8
    }

    fn flip(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

fn random_step(random: &mut SeededRandom) -> Step {
    step(
        random.below(64),
        random.below(16),
        random.below(8),
        random.below(8),
    )
}

fn random_project(random: &mut SeededRandom) -> Project {
    let mut project = Project::new();
    project.set_default_speed(i32::from(random.below(255)) + 1);
    let loop_row = random.flip().then(|| usize::from(random.below(256)));
    project.set_song_loop_row(loop_row);
    for _ in 0..40 {
        let row = usize::from(random.below(256));
        let track = usize::from(random.below(4));
        let chain = ChainId::new(random.below(256));
        project.set_song_cell(row, track, Some(chain));

        let mut entry = ChainEntry::new(PhraseId::new(random.below(256)));
        entry.set_transpose(i32::from(random.below(256)) - 128);
        let index = usize::from(random.below(16));
        *project.chain_mut(chain).entry_mut(index) = Some(entry);

        let id = PhraseId::new(random.below(256));
        let loop_start = i32::from(random.below(33));
        let loop_end = i32::from(random.below(33));
        let speed = i32::from(random.below(255)) + 1;
        let phrase = project.phrase_mut(id);
        phrase.set_loop(loop_start, loop_end);
        phrase.set_speed(speed);
        phrase.filters_mut().set_noiz(random.flip());
        phrase.filters_mut().set_buzz(random.flip());
        phrase.filters_mut().set_detune(i32::from(random.below(3)));
        phrase.filters_mut().set_reverb(i32::from(random.below(3)));
        phrase.filters_mut().set_dampen(i32::from(random.below(3)));
        for index in 0..32 {
            *phrase.step_mut(index) = random_step(random);
        }
    }
    project
}

#[test]
fn any_project_survives_a_save_and_reload() {
    let mut random = SeededRandom(0x9E37_79B9_7F4A_7C15);
    for _ in 0..50 {
        let project = random_project(&mut random);

        assert_eq!(
            project_file::parse(&project_file::write(&project)).unwrap(),
            project
        );
    }
}

fn to_version_2(text: &str) -> Result<String, String> {
    Ok(text.replace("version: 1", "version: 2 /* via 1 */"))
}

fn to_version_3(text: &str) -> Result<String, String> {
    Ok(text.replace("version: 2", "version: 3 /* via 2 */"))
}

#[test]
fn an_older_version_migrates_forward_one_step_at_a_time() {
    let migrations: [Migration; 2] = [to_version_2, to_version_3];

    let migrated = project_file::migrate("(version: 1)", 3, &migrations).unwrap();

    assert_eq!(migrated, "(version: 3 /* via 2 */ /* via 1 */)");
}

#[test]
fn migration_starts_from_the_version_the_file_has() {
    let migrations: [Migration; 2] = [to_version_2, to_version_3];

    let migrated = project_file::migrate("(version: 2)", 3, &migrations).unwrap();

    assert_eq!(migrated, "(version: 3 /* via 2 */)");
}

#[test]
fn a_migration_that_does_not_advance_one_version_is_refused() {
    fn stuck(text: &str) -> Result<String, String> {
        Ok(text.to_owned())
    }
    let migrations: [Migration; 1] = [stuck];

    assert!(matches!(
        project_file::migrate("(version: 1)", 2, &migrations),
        Err(ProjectFileError::Damaged(_))
    ));
}

#[test]
fn a_newer_version_is_refused_with_a_clear_message() {
    let text = fixture_with("version: 1", "version: 99");

    let error = project_file::parse(&text).unwrap_err();

    assert_eq!(error, ProjectFileError::Newer { found: 99 });
    assert_eq!(
        error.to_string(),
        "MADE BY A NEWER PIKUTRACKER: FORMAT 99, THIS VERSION READS UP TO 1"
    );
}

#[test]
fn a_newer_version_is_refused_even_when_its_contents_are_unknown() {
    let error = project_file::parse("(version: 2, tomorrow: [1, 2, 3])").unwrap_err();

    assert_eq!(error, ProjectFileError::Newer { found: 2 });
}

#[test]
fn text_that_is_not_a_project_is_refused() {
    for text in ["", "not ron at all", "(version: \"one\")", "(song: [])"] {
        assert!(
            matches!(project_file::parse(text), Err(ProjectFileError::Damaged(_))),
            "{text:?}"
        );
    }
}

#[test]
fn version_zero_is_refused() {
    assert!(matches!(
        project_file::parse(&fixture_with("version: 1", "version: 0")),
        Err(ProjectFileError::Damaged(_))
    ));
}

#[test]
fn values_outside_their_limits_are_refused() {
    let damaged = [
        ("\"C#2 8 5 3\"", "\"E-5 8 5 3\""),
        ("\"C#2 8 5 3\"", "\"C#2 8 8 3\""),
        ("\"C#2 8 5 3\"", "\"C#2 8 5 8\""),
        ("\"C#2 8 5 3\"", "\"H-2 8 5 3\""),
        ("\"C#2 8 5 3\"", "\"C#2 08 5 3\""),
        ("\"C#2 8 5 3\"", "\"C#2 G 5 3\""),
        ("\"C#2 8 5 3\"", "\"C#2 8 5\""),
        ("loop_start: 3", "loop_start: 33"),
        ("loop_end: 0", "loop_end: 33"),
        ("speed: 8", "speed: 0"),
        (
            "detune: 1,\n            reverb: 2",
            "detune: 3,\n            reverb: 2",
        ),
        ("default_speed: 12", "default_speed: 0"),
        ("song_loop_row: Some(1)", "song_loop_row: Some(256)"),
        ("[3, 4, 50]", "[3, 4, 64]"),
        ("(first: 24, last: 29)", "(first: 30, last: 29)"),
        ("(first: 24, last: 29)", "(first: 24, last: 64)"),
        ("(row: 2,", "(row: 256,"),
        ("-- -- -- FF\"", "-- -- FF\""),
        ("(index: 15,", "(index: 16,"),
        ("id: 8,", "id: 7,"),
        ("id: 15,\n            name", "id: 16,\n            name"),
        ("\"BASS\"", "\"THIRTEEN CHAR\""),
        ("steps: [\"C-2 3 5 0\", \"G-2 3 4 1\"]", "steps: []"),
        ("loop_end: 2", "loop_end: 33"),
        ("-128, 127, 0,", "-128, 127,"),
    ];

    for (from, to) in damaged {
        let result = project_file::parse(&fixture_with(from, to));
        assert!(
            matches!(result, Err(ProjectFileError::Damaged(_))),
            "{from} -> {to}: {result:?}"
        );
    }
}
