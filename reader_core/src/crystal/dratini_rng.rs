//! Pure, allocation-free filters and manual receipt validation. No RNG predictor.

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

#[cfg(test)]
mod tests {
    use super::*;
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
