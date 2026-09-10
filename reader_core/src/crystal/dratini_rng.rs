//! Pure, allocation-free calibration evidence, filters and receipt validation.
//! Missing claim-time state prevents a justified deterministic predictor.

pub const SHINY_ATTACKS: [u8; 8] = [2, 3, 6, 7, 10, 11, 14, 15];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gender {
    Any,
    Male,
    Female,
}

impl Gender {
    pub fn name(self) -> &'static str {
        match self {
            Self::Any => "Any",
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dvs(pub u8, pub u8);

impl Dvs {
    pub fn shiny(self) -> bool {
        self.1 == 0xaa && self.0 & 15 == 10 && SHINY_ATTACKS.contains(&(self.0 >> 4))
    }
    pub fn gender(self) -> Gender {
        if self.0 >> 4 < 8 {
            Gender::Female
        } else {
            Gender::Male
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Filter {
    pub shiny: bool,
    pub gender: Gender,
    pub attack: Option<u8>,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            shiny: true,
            gender: Gender::Any,
            attack: None,
        }
    }
}

impl Filter {
    pub fn possible(self) -> bool {
        SHINY_ATTACKS
            .iter()
            .any(|atk| self.matches(Dvs((atk << 4) | 10, 0xaa)))
    }

    pub fn matches(self, dvs: Dvs) -> bool {
        (!self.shiny || dvs.shiny())
            && (self.gender == Gender::Any || self.gender == dvs.gender())
            && self
                .attack
                .map_or_else(|| SHINY_ATTACKS.contains(&(dvs.0 >> 4)), |a| a == dvs.0 >> 4)
    }
    pub fn preset(index: u8) -> Self {
        match index {
            1 => Self {
                gender: Gender::Male,
                ..Self::default()
            },
            2 => Self {
                gender: Gender::Female,
                ..Self::default()
            },
            3 => Self {
                attack: Some(15),
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verification {
    Success,
    MissingExtremeSpeed,
    Failed,
}

pub fn preflight(count: u8, received: bool, prompt: bool) -> Result<(), &'static str> {
    if received {
        return Err("Dratini gift already received");
    }
    if count == 6 {
        return Err("Party full: gift cannot be claimed");
    }
    if !(1..6).contains(&count) {
        return Err("Invalid party count; manual control");
    }
    if !prompt {
        return Err("Stop at: have recognized your worth.");
    }
    Ok(())
}
pub fn verify(species: u8, level: u8, dvs: Dvs, move4: u8, filter: Filter) -> Verification {
    if species != 147 || level != 15 || !filter.matches(dvs) {
        return Verification::Failed;
    }
    if move4 != 0xf5 {
        Verification::MissingExtremeSpeed
    } else {
        Verification::Success
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimProgress {
    Waiting,
    Complete,
    Failed,
}

/// Receipt is usable only after the script completes its moveset routine.
/// A new party slot on its own can still be partially initialized.
pub fn claim_progress(
    before: u8,
    count: u8,
    received: bool,
    in_shrine: bool,
    elapsed: u32,
    counter_monotonic: bool,
) -> ClaimProgress {
    if !(1..6).contains(&before)
        || !in_shrine
        || elapsed > 7200
        || !counter_monotonic
        || count < before
        || count > before + 1
        || (received && count != before + 1)
    {
        ClaimProgress::Failed
    } else if received {
        ClaimProgress::Complete
    } else {
        ClaimProgress::Waiting
    }
}

/// A copy of the ordinary frame observation, not a generation-time snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub state: u16,
    pub advance: u32,
    pub adiv: (Option<usize>, u8),
    pub sdiv: (Option<usize>, u8),
}
impl Snapshot {
    pub fn tracked(self) -> bool {
        matches!(self.adiv.0, Some(0..=0x3fff)) && matches!(self.sdiv.0, Some(0..=0x3fff))
    }
    fn same_observed_state(self, other: Self) -> bool {
        // The counter is plugin bookkeeping, not a component of the game's RNG.
        self.state == other.state && self.adiv == other.adiv && self.sdiv == other.sdiv
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub start: Snapshot,
    pub end: Snapshot,
    pub party_count: u8,
    pub dvs: Dvs,
    pub move4: u8,
}
impl Sample {
    fn comparable(self, other: Self) -> bool {
        self.party_count == other.party_count && self.start.same_observed_state(other.start)
    }
}
pub const SAMPLE_CAPACITY: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureResult {
    Stored,
    Full,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredictionBlocker {
    MoreSamples,
    ConflictingOutcomes,
    UnobservedTiming,
}
impl PredictionBlocker {
    pub fn explanation(self) -> &'static str {
        match self {
            Self::MoreSamples => "Need multiple natural claims",
            Self::ConflictingOutcomes => "Same observed state, different DVs",
            Self::UnobservedTiming => "Claim timing / DIV reads unobserved",
        }
    }
}

/// Evidence log, not a fitted RNG model. Only completed natural claims are added
/// by the UI path. Never evict conflicts, infer an offset or turn repeats into a
/// predictor. These observations cannot establish the hidden input/cycle phase.
#[derive(Default)]
pub struct Calibration {
    samples: [Option<Sample>; SAMPLE_CAPACITY],
    len: usize,
}
impl Calibration {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn full(&self) -> bool {
        self.len == SAMPLE_CAPACITY
    }
    pub fn sample(&self, index: usize) -> Option<Sample> {
        self.samples.get(index).copied().flatten()
    }
    pub fn record(&mut self, sample: Sample, species: u8, level: u8) -> CaptureResult {
        if species != 147
            || level != 15
            || !(1..6).contains(&sample.party_count)
            || !sample.start.tracked()
            || sample.end.advance < sample.start.advance
        {
            return CaptureResult::Invalid;
        }
        if self.full() {
            return CaptureResult::Full;
        }
        self.samples[self.len] = Some(sample);
        self.len += 1;
        CaptureResult::Stored
    }
    /// Pair counts describe observed reproducibility only. Different setup or
    /// hidden phase can explain a conflict; neither is silently discarded.
    pub fn comparisons(&self) -> (usize, usize) {
        let mut matches = 0;
        let mut conflicts = 0;
        for i in 0..self.len {
            for j in 0..i {
                let a = self.samples[i].unwrap();
                let b = self.samples[j].unwrap();
                if a.comparable(b) {
                    if a.dvs == b.dvs {
                        matches += 1;
                    } else {
                        conflicts += 1;
                    }
                }
            }
        }
        (matches, conflicts)
    }
    pub fn prediction_blocker(&self) -> PredictionBlocker {
        if self.comparisons().1 != 0 {
            PredictionBlocker::ConflictingOutcomes
        } else if self.len < 3 {
            PredictionBlocker::MoreSamples
        } else {
            // Three is only a minimum evidence count, never an unlock threshold.
            PredictionBlocker::UnobservedTiming
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_sample() -> Sample {
        let start = Snapshot {
            state: 0x9fe3,
            advance: 100,
            adiv: (Some(468), 0x78),
            sdiv: (Some(16139), 0x78),
        };
        Sample {
            start,
            end: Snapshot {
                advance: 500,
                ..start
            },
            party_count: 3,
            dvs: Dvs(0x20, 0x31),
            move4: 0xf5,
        }
    }

    #[test]
    fn calibration_keeps_unfiltered_receipts_and_rejects_invalid_capture() {
        let mut c = Calibration::default();
        let sample = fixture_sample();
        assert_eq!(c.record(sample, 148, 15), CaptureResult::Invalid);
        assert_eq!(c.record(sample, 147, 16), CaptureResult::Invalid);
        for party_count in [0, 6, 255] {
            assert_eq!(
                c.record(
                    Sample {
                        party_count,
                        ..sample
                    },
                    147,
                    15
                ),
                CaptureResult::Invalid
            );
        }
        for index in [None, Some(0x4000)] {
            let start = Snapshot {
                adiv: (index, 0),
                ..sample.start
            };
            assert_eq!(
                c.record(Sample { start, ..sample }, 147, 15),
                CaptureResult::Invalid
            );
            let start = Snapshot {
                sdiv: (index, 0),
                ..sample.start
            };
            assert_eq!(
                c.record(Sample { start, ..sample }, 147, 15),
                CaptureResult::Invalid
            );
        }
        let end = Snapshot {
            advance: 99,
            ..sample.end
        };
        assert_eq!(
            c.record(Sample { end, ..sample }, 147, 15),
            CaptureResult::Invalid
        );
        assert_eq!(c.len(), 0);
        assert_eq!(c.record(sample, 147, 15), CaptureResult::Stored);
        let missing_move = Sample {
            move4: 0xef,
            ..sample
        };
        assert_eq!(c.record(missing_move, 147, 15), CaptureResult::Stored);
        assert_eq!(c.sample(1).unwrap().move4, 0xef);
        assert_eq!(c.sample(0).unwrap().dvs, sample.dvs);
        assert!(!sample.dvs.shiny());
        assert!(c.sample(SAMPLE_CAPACITY).is_none());
    }

    #[test]
    fn repeatability_never_substitutes_for_missing_claim_timing() {
        let mut c = Calibration::default();
        assert_eq!(c.prediction_blocker(), PredictionBlocker::MoreSamples);
        for n in 0..SAMPLE_CAPACITY {
            let mut sample = fixture_sample();
            // Same observed state, different plugin counter / completion latency.
            sample.start.advance += n as u32;
            sample.end.advance += n as u32 * 10;
            assert_eq!(c.record(sample, 147, 15), CaptureResult::Stored);
            assert_eq!(
                c.prediction_blocker(),
                if n < 2 {
                    PredictionBlocker::MoreSamples
                } else {
                    PredictionBlocker::UnobservedTiming
                }
            );
        }
        assert_eq!(c.comparisons(), (120, 0));
        assert!(c.full());
        assert_eq!(c.record(fixture_sample(), 147, 15), CaptureResult::Full);
        assert_eq!(c.len(), SAMPLE_CAPACITY);
    }

    #[test]
    fn conflicting_natural_outcome_is_retained_and_blocks_prediction() {
        let mut c = Calibration::default();
        let sample = fixture_sample();
        assert_eq!(c.record(sample, 147, 15), CaptureResult::Stored);
        assert_eq!(
            c.record(
                Sample {
                    dvs: Dvs(0xfa, 0xaa),
                    ..sample
                },
                147,
                15
            ),
            CaptureResult::Stored
        );
        for _ in 2..SAMPLE_CAPACITY {
            assert_eq!(c.record(sample, 147, 15), CaptureResult::Stored);
        }
        assert_eq!(c.prediction_blocker(), PredictionBlocker::ConflictingOutcomes);
        assert_eq!(c.comparisons().1, 15);
        assert_eq!(c.record(sample, 147, 15), CaptureResult::Full);
        assert_eq!(c.sample(1).unwrap().dvs, Dvs(0xfa, 0xaa));
    }

    #[test]
    fn different_observed_states_do_not_count_as_repeated_trials() {
        let mut c = Calibration::default();
        for n in 0..6 {
            let mut sample = fixture_sample();
            match n {
                1 => sample.start.state ^= 1,
                2 => sample.start.adiv.0 = Some(469),
                3 => sample.start.sdiv.0 = Some(16140),
                4 => sample.start.adiv.1 ^= 1,
                5 => sample.party_count = 4,
                _ => (),
            }
            assert_eq!(c.record(sample, 147, 15), CaptureResult::Stored);
        }
        assert_eq!(c.comparisons(), (0, 0));
        assert_eq!(c.prediction_blocker(), PredictionBlocker::UnobservedTiming);
    }
    #[test]
    fn shiny_all_65536_spreads() {
        let mut count = 0;
        for raw in 0..=u16::MAX {
            let dvs = Dvs((raw >> 8) as u8, raw as u8);
            let expected = raw & 0x0fff == 0x0aaa && (raw >> 12) & 2 != 0;
            assert_eq!(dvs.shiny(), expected);
            count += usize::from(dvs.shiny());
        }
        assert_eq!(count, 8);
    }

    #[test]
    fn gender_threshold_and_attack_filters() {
        for atk in 0..16 {
            let dvs = Dvs((atk << 4) | 10, 0xaa);
            assert_eq!(dvs.gender(), if atk < 8 { Gender::Female } else { Gender::Male });
            for gender in [Gender::Any, Gender::Female, Gender::Male] {
                for wanted in 0..16 {
                    let filter = Filter {
                        shiny: false,
                        gender,
                        attack: Some(wanted),
                    };
                    assert_eq!(
                        filter.matches(dvs),
                        wanted == atk && (gender == Gender::Any || gender == dvs.gender())
                    );
                }
            }
        }
        assert!(Filter::default().matches(Dvs(0x2a, 0xaa)));
        assert!(!Filter::default().matches(Dvs(0x2a, 0xab)));
        assert!(Filter::preset(1).matches(Dvs(0xaa, 0xaa)));
        assert!(!Filter::preset(1).matches(Dvs(0x7a, 0xaa)));
        assert!(Filter::preset(2).matches(Dvs(0x7a, 0xaa)));
        assert!(!Filter::preset(2).matches(Dvs(0xaa, 0xaa)));
        assert!(Filter::preset(3).matches(Dvs(0xfa, 0xaa)));
        assert!(!Filter::preset(3).matches(Dvs(0xea, 0xaa)));
        assert!(!Filter {
            gender: Gender::Female,
            ..Filter::preset(3)
        }
        .matches(Dvs(0xfa, 0xaa)));
    }

    #[test]
    fn receipt_requires_identity_level_filters_and_final_move() {
        let dvs = Dvs(0xfa, 0xaa);
        assert_eq!(
            verify(147, 15, dvs, 0xf5, Filter::default()),
            Verification::Success
        );
        assert_eq!(
            verify(147, 15, dvs, 0xef, Filter::default()),
            Verification::MissingExtremeSpeed
        );
        assert_eq!(
            verify(148, 15, dvs, 0xf5, Filter::default()),
            Verification::Failed
        );
        assert_eq!(
            verify(147, 16, dvs, 0xf5, Filter::default()),
            Verification::Failed
        );
        assert_eq!(
            verify(147, 15, Dvs(0xea, 0xaa), 0xf5, Filter::preset(3)),
            Verification::Failed
        );
        assert_eq!(
            verify(147, 15, dvs, 0xf5, Filter::preset(2)),
            Verification::Failed
        );
        assert_eq!(
            verify(147, 15, Dvs(0xf0, 0), 0xf5, Filter::default()),
            Verification::Failed
        );
        let no_shiny = Filter {
            shiny: false,
            ..Filter::default()
        };
        assert_eq!(
            verify(147, 15, Dvs(0xf0, 0), 0xf5, no_shiny),
            Verification::Success
        );
        assert_eq!(
            verify(147, 15, Dvs(0xf0, 0), 0xef, no_shiny),
            Verification::MissingExtremeSpeed
        );
    }
    #[test]
    fn eligibility_and_conflicting_filters() {
        for count in 0..=7 {
            assert_eq!(preflight(count, false, true).is_ok(), (1..6).contains(&count));
            assert!(preflight(count, true, true).is_err());
            assert!(preflight(count, false, false).is_err());
        }
        assert!(Filter::default().possible());
        assert!(!Filter {
            gender: Gender::Female,
            ..Filter::preset(3)
        }
        .possible());
        let filter = Filter {
            shiny: false,
            ..Filter::default()
        };
        assert!(!filter.matches(Dvs(0x40, 0)));
        assert!(filter.matches(Dvs(0x20, 0)));
    }
    #[test]
    fn receipt_waits_for_completed_script_and_stops_on_unexpected_state() {
        for before in 1..6 {
            assert_eq!(
                claim_progress(before, before, false, true, 0, true),
                ClaimProgress::Waiting
            );
            assert_eq!(
                claim_progress(before, before + 1, false, true, 7200, true),
                ClaimProgress::Waiting
            );
            assert_eq!(
                claim_progress(before, before + 1, true, true, 7200, true),
                ClaimProgress::Complete
            );
            assert_eq!(
                claim_progress(before, before, true, true, 0, true),
                ClaimProgress::Failed
            );
            assert_eq!(
                claim_progress(before, before - 1, false, true, 0, true),
                ClaimProgress::Failed
            );
            assert_eq!(
                claim_progress(before, before + 2, false, true, 0, true),
                ClaimProgress::Failed
            );
            assert_eq!(
                claim_progress(before, before + 1, true, false, 0, true),
                ClaimProgress::Failed
            );
            assert_eq!(
                claim_progress(before, before + 1, true, true, 7201, true),
                ClaimProgress::Failed
            );
            assert_eq!(
                claim_progress(before, before + 1, true, true, 0, false),
                ClaimProgress::Failed
            );
        }
        for before in [0, 6, 255] {
            assert_eq!(
                claim_progress(before, before, false, true, 0, true),
                ClaimProgress::Failed
            );
        }
    }
}
