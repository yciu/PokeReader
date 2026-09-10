//! Pure, allocation-free Crystal timing arithmetic. All values are plugin copies.
//! No starter encounter offsets are used. Gift models must be learned from real claims.

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
pub struct Div {
    pub index: u16,
    pub value: u8,
}

impl Div {
    fn next(&mut self) {
        const INC: [u8; 16] = [18, 18, 18, 19, 18, 18, 19, 18, 18, 19, 18, 18, 19, 18, 18, 19];
        let mut increment = INC[(self.index & 15) as usize];
        if [8, 9, 0x562, 0x563, 0x22b5, 0x22b6].contains(&self.index) {
            increment = 37 - increment;
        }
        self.value = self.value.wrapping_add(increment);
        self.index = (self.index + 1) & 0x3fff;
    }
}

/// Exact Random arithmetic when its two DIV reads and incoming carry are known.
/// Carry out is the SUB borrow, and is carried into the second DV-producing Random.
pub fn random(state: u16, adiv: u8, sdiv: u8, carry: bool) -> (u16, bool) {
    let sum = (state >> 8) + u16::from(adiv) + u16::from(carry);
    let subtrahend = u16::from(sdiv) + u16::from(sum > 255);
    let sub = state & 255;
    (
        ((sum & 255) << 8) | (sub.wrapping_sub(subtrahend) & 255),
        sub < subtrahend,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    pub state: u16,
    pub add: Div,
    pub sub: Div,
}

impl Rng {
    pub fn next(&mut self) {
        self.add.next();
        self.sub.next();
        self.state = random(self.state, self.add.value, self.sub.value, false).0;
    }
    pub fn advance(&mut self, count: u32) {
        for _ in 0..count {
            self.next();
        }
    }
}

/// Empirical DIV transformation, fitted independently to each of the four reads.
/// The family is deliberately bounded; unmatched/ambiguous data cannot unlock search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform {
    pub slope: u8,
    pub offset: u8,
}
impl Transform {
    fn apply(self, div: u8) -> u8 {
        div.wrapping_mul(self.slope).wrapping_add(self.offset)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub delay: u32,
    pub reads: [Transform; 4],
}

impl Model {
    pub fn matches_sample(self, sample: Sample) -> bool {
        if self.delay != sample.delay || !sample.valid() {
            return false;
        }
        let base = [
            sample.before.add.value,
            sample.before.sub.value,
            sample.before.add.value,
            sample.before.sub.value,
        ];
        (0..4).all(|i| self.reads[i].apply(base[i]) == sample.reads[i])
            && self.dvs(sample.before) == sample.dvs
    }

    pub fn dvs(self, before: Rng) -> Dvs {
        let base = [
            before.add.value,
            before.sub.value,
            before.add.value,
            before.sub.value,
        ];
        let reads = core::array::from_fn::<_, 4, _>(|i| self.reads[i].apply(base[i]));
        let (first, carry) = random(before.state, reads[0], reads[1], false);
        let (second, _) = random(first, reads[2], reads[3], carry);
        Dvs(first as u8, second as u8)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub start: Rng,
    pub delay: u32,
    pub before: Rng,
    pub reads: [u8; 4],
    pub dvs: Dvs,
}

impl Sample {
    pub fn valid(self) -> bool {
        if self.delay > 512 {
            return false;
        }
        let mut expected = self.start;
        expected.advance(self.delay);
        let (first, carry) = random(self.before.state, self.reads[0], self.reads[1], false);
        let (second, _) = random(first, self.reads[2], self.reads[3], carry);
        expected == self.before && self.dvs == Dvs(first as u8, second as u8)
    }
}

pub struct Calibration {
    first: Option<Sample>,
    starts: [Option<Rng>; 5],
    masks: [u8; 4],
    pub samples: usize,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            first: None,
            starts: [None; 5],
            masks: [7; 4],
            samples: 0,
        }
    }
}

impl Calibration {
    /// Three distinct claims identify a unique model; two further distinct claims
    /// are held-out validation. Repeated identical starts never increase confidence.
    pub fn add(&mut self, sample: Sample) -> bool {
        if !sample.valid() {
            *self = Self::default();
            return false;
        }
        if self.starts.contains(&Some(sample.start)) {
            return false;
        }
        if let Some(first) = self.first {
            if first.delay != sample.delay {
                *self = Self::default();
                return false;
            }
            for i in 0..4 {
                let base = if i & 1 == 0 {
                    sample.before.add.value
                } else {
                    sample.before.sub.value
                };
                for slope in 0..3 {
                    if self.transform(i, slope).apply(base) != sample.reads[i] {
                        self.masks[i] &= !(1 << slope);
                    }
                }
            }
            if self.masks.contains(&0) {
                *self = Self::default();
                return false;
            }
        } else {
            self.first = Some(sample);
        }
        if self.samples < 5 {
            self.starts[self.samples] = Some(sample.start);
            self.samples += 1;
        }
        // Ambiguity after identification must not be counted as held-out validation.
        if self.samples >= 3 && self.masks.iter().any(|mask| mask.count_ones() != 1) {
            self.samples = 2;
        }
        true
    }
    fn transform(&self, i: usize, slope: u8) -> Transform {
        let first = self.first.unwrap();
        let base = if i & 1 == 0 {
            first.before.add.value
        } else {
            first.before.sub.value
        };
        Transform {
            slope,
            offset: first.reads[i].wrapping_sub(base.wrapping_mul(slope)),
        }
    }
    pub fn model(&self) -> Option<Model> {
        if self.samples < 5 || self.masks.iter().any(|mask| mask.count_ones() != 1) {
            return None;
        }
        Some(Model {
            delay: self.first?.delay,
            reads: core::array::from_fn(|i| self.transform(i, self.masks[i].trailing_zeros() as u8)),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub advance: u32,
    pub rng: Rng,
    pub dvs: Dvs,
}

pub struct Search {
    cursor: Rng,
    delayed: Rng,
    advance: u32,
    remaining: u32,
    model: Model,
}
impl Search {
    pub fn new(start: Rng, advance: u32, model: Model) -> Self {
        let mut delayed = start;
        delayed.advance(model.delay);
        Self {
            cursor: start,
            delayed,
            advance,
            remaining: 1_000_000,
            model,
        }
    }
    /// Constant memory, at most 256 candidates per overlay frame, in ascending order.
    pub fn chunk(&mut self, filter: Filter, minimum: u32) -> Option<Target> {
        for _ in 0..256 {
            if self.remaining == 0 {
                return None;
            }
            let target = Target {
                advance: self.advance,
                rng: self.cursor,
                dvs: self.model.dvs(self.delayed),
            };
            self.cursor.next();
            self.delayed.next();
            self.remaining -= 1;
            self.advance = match self.advance.checked_add(1) {
                Some(n) => n,
                None => {
                    self.remaining = 0;
                    return None;
                }
            };
            if target.advance >= minimum && filter.matches(target.dvs) {
                return Some(target);
            }
        }
        None
    }
    pub fn exhausted(&self) -> bool {
        self.remaining == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verification {
    Success,
    MissingExtremeSpeed,
    Failed,
}
pub fn verify(species: u8, level: u8, dvs: Dvs, move4: u8, target: Dvs, filter: Filter) -> Verification {
    if species != 147 || level != 15 || dvs != target || !filter.matches(dvs) {
        return Verification::Failed;
    }
    if move4 != 0xf5 {
        Verification::MissingExtremeSpeed
    } else {
        Verification::Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng {
            state: 0x9fe3,
            add: Div {
                index: 468,
                value: 0x78,
            },
            sub: Div {
                index: 16139,
                value: 0x78,
            },
        }
    }
    fn model() -> Model {
        Model {
            delay: 7,
            reads: [
                Transform { slope: 1, offset: 10 },
                Transform { slope: 1, offset: 10 },
                Transform { slope: 1, offset: 11 },
                Transform { slope: 1, offset: 11 },
            ],
        }
    }
    fn sample(n: u8) -> Sample {
        let mut start = rng();
        start.add.value = n;
        start.sub.value = n.wrapping_add(19);
        start.state = u16::from(n) * 257;
        let mut before = start;
        before.advance(model().delay);
        let reads = [
            before.add.value.wrapping_add(10),
            before.sub.value.wrapping_add(10),
            before.add.value.wrapping_add(11),
            before.sub.value.wrapping_add(11),
        ];
        Sample {
            start,
            delay: model().delay,
            before,
            reads,
            dvs: model().dvs(before),
        }
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
    fn random_carry_and_borrow() {
        assert_eq!(random(0xff00, 1, 0, false), (0x00ff, true));
        assert_eq!(random(0x00ff, 0, 255, true), (0x0100, false));
        assert_eq!(random(0xffff, 255, 255, true), (0xffff, true));
        assert_eq!(random(0x0000, 0, 1, false), (0x00ff, true));
    }

    #[test]
    fn idle_reference_and_div_wrap() {
        let mut r = rng();
        // Published GameboyRng reference sequence, PokemonRNGGuides (GPL-3.0).
        for expected in [0x2958, 0xc5bb, 0x740b, 0x3549, 0x0874, 0xee8e, 0xe695, 0xf08b] {
            r.next();
            assert_eq!(r.state, expected);
        }
        let mut div = Div {
            index: 0x3fff,
            value: 250,
        };
        div.next();
        assert_eq!(div, Div { index: 0, value: 13 });
        for (index, expected) in [
            (8, 19),
            (9, 18),
            (0x562, 19),
            (0x563, 18),
            (0x22b5, 19),
            (0x22b6, 18),
        ] {
            let mut div = Div { index, value: 0 };
            div.next();
            assert_eq!(div.value, expected);
        }
    }

    #[test]
    fn calibration_requires_identification_and_holdouts() {
        let mut c = Calibration::default();
        for n in 1..=4 {
            assert!(c.add(sample(n)));
            assert!(c.model().is_none());
        }
        assert!(c.add(sample(5)));
        assert_eq!(c.model(), Some(model()));
        assert!(!c.add(sample(5)));
        assert_eq!(c.samples, 5);
        let mut wrong = sample(6);
        wrong.reads[0] = wrong.reads[0].wrapping_add(9);
        let (first, carry) = random(wrong.before.state, wrong.reads[0], wrong.reads[1], false);
        let (second, _) = random(first, wrong.reads[2], wrong.reads[3], carry);
        wrong.dvs = Dvs(first as u8, second as u8);
        assert!(!c.add(wrong));
        assert!(c.model().is_none());
        assert_eq!(c.samples, 0);
    }

    #[test]
    fn calibration_rejects_bad_path_and_ambiguity() {
        let mut c = Calibration::default();
        let mut s = sample(1);
        s.dvs.0 ^= 1;
        assert!(!c.add(s));
        let mut s = sample(1);
        s.before.state ^= 1;
        assert!(!s.valid());
        let mut s = sample(1);
        s.delay = 513;
        assert!(!s.valid());
        for n in 1..=10 {
            // Different states but identical DIV baselines cannot identify the slope.
            let mut s = sample(1);
            s.start.state = n;
            s.before = s.start;
            s.before.advance(s.delay);
            s.dvs = model().dvs(s.before);
            assert!(c.add(s));
        }
        assert!(c.model().is_none());
    }

    #[test]
    fn search_returns_nearest_without_skipping_chunk_edges() {
        let filter = Filter {
            shiny: false,
            gender: Gender::Female,
            attack: Some(7),
        };
        let mut base = rng();
        let mut delayed = base;
        delayed.advance(model().delay);
        let mut expected = None;
        for advance in 0..10000 {
            let dvs = model().dvs(delayed);
            if advance >= 300 && filter.matches(dvs) {
                expected = Some(Target {
                    advance,
                    rng: base,
                    dvs,
                });
                break;
            }
            base.next();
            delayed.next();
        }
        assert!(expected.is_some());
        let mut search = Search::new(rng(), 0, model());
        assert!(search.chunk(filter, 300).is_none());
        let mut found = None;
        for _ in 0..40 {
            if found.is_none() {
                found = search.chunk(filter, 300);
            }
        }
        assert_eq!(found, expected);
    }

    #[test]
    fn receipt_requires_identity_level_target_shiny_and_move4() {
        let dvs = Dvs(0xfa, 0xaa);
        assert_eq!(
            verify(147, 15, dvs, 0xf5, dvs, Filter::default()),
            Verification::Success
        );
        assert_eq!(
            verify(147, 15, dvs, 0xef, dvs, Filter::default()),
            Verification::MissingExtremeSpeed
        );
        assert_eq!(
            verify(148, 15, dvs, 0xf5, dvs, Filter::default()),
            Verification::Failed
        );
        assert_eq!(
            verify(147, 16, dvs, 0xf5, dvs, Filter::default()),
            Verification::Failed
        );
        assert_eq!(
            verify(147, 15, Dvs(0xea, 0xaa), 0xf5, dvs, Filter::default()),
            Verification::Failed
        );
        assert_eq!(
            verify(147, 15, Dvs(0, 0), 0xf5, Dvs(0, 0), Filter::default()),
            Verification::Failed
        );
    }
}
