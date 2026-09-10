//! English VC only. Observer, calibrated timing assistant and read-only verifier.
//! Never calls a game-memory write API or installs a hook.
use super::{dratini_rng::*, hook, reader::Gen2Reader};
use crate::{
    pnp::{self, Button},
    title::{loaded_title, LoadedTitle},
};
use once_cell::unsync::Lazy;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Idle,
    Needed,
    Calibrating,
    Found,
    Approaching,
    Ready,
    Verifying,
    Success,
    Failed,
}
impl Status {
    fn name(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Needed => "CALIBRATION NEEDED",
            Self::Calibrating => "CALIBRATING",
            Self::Found => "TARGET FOUND",
            Self::Approaching => "APPROACHING",
            Self::Ready => "READY",
            Self::Verifying => "VERIFYING",
            Self::Success => "SUCCESS",
            Self::Failed => "FAILED / MANUAL CONTROL",
        }
    }
}

#[derive(Clone, Copy)]
struct Snapshot {
    rng: Rng,
    advance: u32,
}
fn snapshot(reader: &Gen2Reader) -> Option<Snapshot> {
    let div = hook::measured_div();
    Some(Snapshot {
        rng: Rng {
            state: reader.rng_state(),
            add: Div {
                index: hook::add_div_tracker().index()? as u16,
                value: (div >> 8) as u8,
            },
            sub: Div {
                index: hook::sub_div_tracker().index()? as u16,
                value: div as u8,
            },
        },
        advance: hook::rng_advance(),
    })
}

struct Trace {
    start: Snapshot,
    before: Option<Snapshot>,
    reads: [u8; 4],
    count: usize,
    bad: bool,
}

struct Helper {
    status: Status,
    note: &'static str,
    filter: Filter,
    row: u8,
    details: bool,
    preset: u8,
    calibration: Calibration,
    context: Option<u64>,
    party_count: u8,
    armed: bool,
    trace: Option<Trace>,
    search: Option<Search>,
    target: Option<Target>,
    previous: Option<Snapshot>,
    elapsed: u32,
    last_sample: Option<Sample>,
}
impl Default for Helper {
    fn default() -> Self {
        Self {
            status: Status::Idle,
            note: "English Crystal VC - timing only",
            filter: Filter::default(),
            row: 0,
            details: false,
            preset: 0,
            calibration: Calibration::default(),
            context: None,
            party_count: 0,
            armed: false,
            trace: None,
            search: None,
            target: None,
            previous: None,
            elapsed: 0,
            last_sample: None,
        }
    }
}
unsafe fn helper() -> &'static mut Helper {
    static mut STATE: Lazy<Helper> = Lazy::new(Helper::default);
    Lazy::force_mut(&mut STATE)
}
pub fn english() -> bool {
    matches!(loaded_title(), Ok(LoadedTitle::CrystalEn))
}

impl Helper {
    fn stop(&mut self, note: &'static str) {
        self.status = Status::Failed;
        self.note = note;
        self.armed = false;
        self.trace = None;
        self.search = None;
        self.target = None;
        self.previous = None;
    }
    fn start(&mut self, reader: &Gen2Reader) {
        if !(1..6).contains(&reader.party_count()) {
            self.stop("Need 1-5 party Pokemon");
            return;
        }
        if reader.dratini_received() {
            self.stop("Dratini gift already received");
            return;
        }
        if !reader.dratini_prompt() {
            self.stop("Stop at: have recognized your worth.");
            return;
        }
        let Some(now) = snapshot(reader) else {
            self.stop("Wait for both DIV trackers");
            return;
        };
        let context = reader.dratini_context();
        if self.context != Some(context) {
            self.calibration = Calibration::default();
            self.context = Some(context);
        }
        self.party_count = reader.party_count();
        self.elapsed = 0;
        self.trace = None;
        self.previous = Some(now);
        self.target = None;
        self.armed = false;
        self.details = true;
        if let Some(model) = self.calibration.model() {
            self.search = Some(Search::new(now.rng, now.advance, model));
            self.status = Status::Approaching;
            self.note = "Searching (256 candidates/frame)";
        } else {
            self.search = None;
            self.armed = true;
            self.status = Status::Needed;
            self.note = "Paused: release keys; press A once";
            pnp::request_pause();
        }
    }

    fn finish(&mut self, reader: &Gen2Reader) {
        let Some(trace) = self.trace.take() else {
            self.stop("Missing input capture");
            return;
        };
        if reader.party_count() != self.party_count + 1 {
            self.stop("Unexpected party change");
            return;
        }
        let mon = reader.party(self.party_count);
        let dvs = Dvs((mon.atk << 4) | mon.def, (mon.spe << 4) | mon.spc);
        let (level, move4) = reader.party_level_move4(self.party_count);
        if mon.spec_index != 147 || level != 15 {
            self.stop("Newest Pokemon is not L15 Dratini");
            return;
        }
        let sample = trace.before.and_then(|before| {
            Some(Sample {
                start: trace.start.rng,
                delay: before.advance.checked_sub(trace.start.advance)?,
                before: before.rng,
                reads: trace.reads,
                dvs,
            })
        });
        self.last_sample = sample;
        let valid_trace = !trace.bad && trace.count == 4 && sample.map_or(false, |s| s.valid());
        if let Some(target) = self.target {
            let stable = valid_trace
                && self
                    .calibration
                    .model()
                    .map_or(false, |m| m.matches_sample(sample.unwrap()));
            if !stable {
                self.calibration = Calibration::default();
            }
            let outcome = verify(mon.spec_index, level, dvs, move4, target.dvs, self.filter);
            self.search = None;
            self.previous = None;
            match outcome {
                Verification::Success => {
                    self.status = Status::Success;
                    self.note = if stable {
                        "Natural Dratini verified: move4 F5"
                    } else {
                        "Target verified; timing needs calibration"
                    };
                }
                Verification::MissingExtremeSpeed => {
                    self.status = Status::Failed;
                    self.note = if dvs.shiny() {
                        "SHINY OK - EXTREMESPEED MISSING"
                    } else {
                        "DVS OK - EXTREMESPEED MISSING"
                    };
                }
                Verification::Failed => {
                    self.calibration = Calibration::default();
                    self.stop("Prediction failed; recalibrate");
                }
            }
        } else {
            if !valid_trace {
                self.calibration = Calibration::default();
                self.stop("Unmodelled RNG path; manual control");
                return;
            }
            let accepted = self.calibration.add(sample.unwrap());
            self.status = Status::Needed;
            self.previous = None;
            self.note = if accepted {
                "Sample read. Reset normally; repeat."
            } else {
                "Unstable/duplicate sample; retry."
            };
        }
    }
}

/// Called from the existing emulator read observer. Only plugin-owned copies change.
/// PC is the immediate-byte address of each Random ldh a,[rDIV].
pub fn observe_random(pc: u16, reader: &Gen2Reader) {
    if !english() || ![0x2f8e, 0x2f96].contains(&pc) {
        return;
    }
    let h = unsafe { helper() };
    let Some(trace) = h.trace.as_mut() else {
        return;
    };
    let expected_pc = if trace.count & 1 == 0 { 0x2f8e } else { 0x2f96 };
    if trace.count >= 4 || pc != expected_pc {
        trace.bad = true;
        return;
    }
    if trace.count == 0 {
        trace.before = snapshot(reader);
    }
    if trace.before.map_or(true, |s| s.advance != hook::rng_advance()) {
        trace.bad = true;
    }
    trace.reads[trace.count] = reader.div();
    trace.count += 1;
}

/// Existing pause loop reports a real physical input before resuming emulation.
/// This callback neither consumes nor synthesizes input.
#[no_mangle]
pub extern "C" fn crystal_timing_resume(keys: u32) {
    if !english() {
        return;
    }
    let h = unsafe { helper() };
    if !h.armed && h.status != Status::Ready {
        return;
    }
    if keys != Button::A as u32 || pnp::is_pressing(!(Button::A as u32)) {
        h.stop("Use A alone from pause; re-arm");
        return;
    }
    let reader = Gen2Reader::crystal();
    if !reader.dratini_prompt()
        || reader.dratini_received()
        || reader.party_count() != h.party_count
        || h.context != Some(reader.dratini_context())
    {
        h.stop("Gift setup changed; manual control");
        return;
    }
    let Some(now) = snapshot(&reader) else {
        h.stop("DIV trackers lost; re-arm");
        return;
    };
    if h.target
        .map_or(false, |t| t.advance != now.advance || t.rng != now.rng)
    {
        h.stop("Pause boundary missed; re-arm");
        return;
    }
    h.armed = false;
    h.elapsed = 0;
    h.trace = Some(Trace {
        start: now,
        before: None,
        reads: [0; 4],
        count: 0,
        bad: false,
    });
    h.status = if h.target.is_some() {
        Status::Verifying
    } else {
        Status::Calibrating
    };
    h.note = "Claim normally; finish nickname prompt";
}

pub fn cancel() {
    if !english() {
        return;
    }
    let h = unsafe { helper() };
    if h.armed || h.search.is_some() || h.trace.is_some() {
        h.stop("Cancelled; manual control");
    }
}

/// Runs even when the overlay is hidden, so a completed gift cannot leave stale work.
pub fn tick(reader: &Gen2Reader) {
    if !english() {
        return;
    }
    let h = unsafe { helper() };
    if h.trace.is_some() {
        h.elapsed = h.elapsed.saturating_add(1);
        if !reader.in_dragon_shrine() || h.elapsed > 7200 {
            h.stop("Claim interrupted or timed out");
            return;
        }
        if reader.party_count() < h.party_count || reader.party_count() > h.party_count + 1 {
            h.stop("Unexpected party change");
            return;
        }
        if reader.party_count() == h.party_count + 1 {
            h.status = Status::Verifying;
        }
        // EVENT_GOT_DRATINI is set AFTER GivePoke, nickname and GiveDratini.
        if reader.dratini_received() {
            h.finish(reader);
        }
        return;
    }
    if h.search.is_none() {
        return;
    }
    if !reader.dratini_prompt() || reader.dratini_received() || reader.party_count() != h.party_count {
        h.stop("Left gift timing point");
        return;
    }
    let Some(now) = snapshot(reader) else {
        h.stop("DIV tracker lost; manual control");
        return;
    };
    if let Some(previous) = h.previous {
        let Some(distance) = now.advance.checked_sub(previous.advance) else {
            h.stop("RNG reset; re-arm");
            return;
        };
        if distance > 8 {
            h.stop("Unexpected advance gap");
            return;
        }
        let mut expected = previous.rng;
        expected.advance(distance);
        if expected != now.rng {
            h.stop("Idle RNG diverged; re-arm");
            return;
        }
    }
    h.previous = Some(now);
    if h.target.is_none() {
        let search = h.search.as_mut().unwrap();
        h.target = search.chunk(h.filter, now.advance.saturating_add(1));
        if h.target.is_some() {
            h.status = Status::Found;
            h.note = "Empirical target; checking live RNG";
        } else if search.exhausted() {
            h.stop("No match in 1,000,000 advances");
        }
        return;
    }
    let target = h.target.unwrap();
    if now.advance > target.advance {
        h.stop("Target passed; manual control");
    } else if now.advance == target.advance {
        if now.rng != target.rng {
            h.stop("Target state mismatch");
            return;
        }
        h.status = Status::Ready;
        h.note = "Pause requested. Press A once.";
        pnp::request_pause();
    } else {
        h.status = Status::Approaching;
        h.note = "L+R pause / L step / R resume";
    }
}

pub fn draw(reader: &Gen2Reader, locked: bool) {
    if !english() {
        pnp::println!("English Crystal VC only");
        return;
    }
    pnp::set_print_max_len(40);
    let h = unsafe { helper() };
    if !locked && !(pnp::is_pressing(Button::X) && pnp::is_pressing(Button::Y)) {
        if pnp::is_just_pressed(Button::Y) {
            h.details = !h.details;
        }
        if !h.details {
            if pnp::is_just_pressed(Button::Dup) {
                h.row = (h.row + 5) % 6;
            }
            if pnp::is_just_pressed(Button::Ddown) {
                h.row = (h.row + 1) % 6;
            }
            if pnp::is_just_pressed(Button::X) {
                match h.row {
                    0 => {
                        h.filter.shiny = !h.filter.shiny;
                        h.stop("Filter changed; re-arm");
                    }
                    1 => {
                        h.filter.gender = match h.filter.gender {
                            Gender::Any => Gender::Male,
                            Gender::Male => Gender::Female,
                            Gender::Female => Gender::Any,
                        };
                        h.stop("Filter changed; re-arm");
                    }
                    2 => {
                        h.filter.attack = match h.filter.attack {
                            None => Some(2),
                            Some(a) => SHINY_ATTACKS
                                .iter()
                                .position(|n| *n == a)
                                .and_then(|i| SHINY_ATTACKS.get(i + 1).copied()),
                        };
                        h.stop("Filter changed; re-arm");
                    }
                    3 => {
                        h.preset = (h.preset + 1) % 4;
                        h.filter = Filter::preset(h.preset);
                        h.stop("Preset changed; re-arm");
                    }
                    4 => h.start(reader),
                    _ => {
                        h.stop("Cancelled; manual control");
                        h.calibration = Calibration::default();
                    }
                }
            }
        } else if pnp::is_just_pressed(Button::X) {
            h.stop("Cancelled; manual control");
        }
    }
    pnp::println!("ExtremeSpeed Dratini RNG");
    pnp::println!("{}", h.status.name());
    if !h.details {
        let cursor = |row| if h.row == row { ">" } else { " " };
        pnp::println!(
            "{} Shiny: {}",
            cursor(0),
            if h.filter.shiny { "ON" } else { "OFF" }
        );
        pnp::println!("{} Gender: {}", cursor(1), h.filter.gender.name());
        match h.filter.attack {
            Some(a) => pnp::println!("{} Attack DV: {}", cursor(2), a),
            None => pnp::println!("{} Attack DV: Any shiny-valid", cursor(2)),
        }
        pnp::println!(
            "{} Preset: {}",
            cursor(3),
            [
                "Fast: any shiny",
                "Male shiny",
                "Female shiny",
                "Collector: Atk 15"
            ][h.preset as usize]
        );
        pnp::println!("{} Start / calibrate", cursor(4));
        pnp::println!("{} Clear calibration / cancel", cursor(5));
        pnp::println!("Up/Down select; X change; Y live");
        pnp::println!("Stop at elder's final text:");
        pnp::println!("have recognized your worth.");
        pnp::println!("Five distinct manual claims needed.");
        pnp::println!("Reset normally without saving claims.");
        pnp::println!("L+R pause; L step; R resume");
    } else {
        let div = hook::measured_div();
        pnp::println!("RNG {:04X}  Advances {}", reader.rng_state(), hook::rng_advance());
        pnp::println!("ADIV {:?} / {:02X}", hook::add_div_tracker().index(), div >> 8);
        pnp::println!("SDIV {:?} / {:02X}", hook::sub_div_tracker().index(), div as u8);
        if let Some(target) = h.target {
            pnp::println!(
                "Nearest {}  Distance {}",
                target.advance,
                target.advance.saturating_sub(hook::rng_advance())
            );
            pnp::println!(
                "Pred Atk/Def/Spe/Spc {}/{}/{}/{}",
                target.dvs.0 >> 4,
                target.dvs.0 & 15,
                target.dvs.1 >> 4,
                target.dvs.1 & 15
            );
            pnp::println!(
                "Pred shiny {} / {}",
                target.dvs.shiny(),
                target.dvs.gender().name()
            );
        } else {
            pnp::println!("Nearest / distance / predicted: --");
        }
        pnp::println!("Calibration {}/5", h.calibration.samples);
        if let Some(model) = h.calibration.model() {
            pnp::println!("Validated delay {} VBlanks", model.delay);
        }
        if let Some(sample) = h.last_sample {
            pnp::println!(
                "Last capture RNG {:04X} delay {}",
                sample.start.state,
                sample.delay
            );
            pnp::println!("Last DVs {:02X} {:02X}", sample.dvs.0, sample.dvs.1);
            pnp::println!("DIV reads {:02X?}", sample.reads);
        }
        pnp::println!("Y options / X cancel");
    }
    pnp::println!("{}", h.note);
    if h.note.contains("EXTREMESPEED MISSING") {
        pnp::println!("Dragon Master quiz condition not met.");
    }
}
