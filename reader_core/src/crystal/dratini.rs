//! English VC safe calibration evidence and read-only receipt verifier.
//! Called only from the overlay frame path; no RNG or pause-hook observer.
use super::{dratini_rng::*, hook, reader::Gen2Reader};
use crate::{
    pnp::{self, Button},
    title::{loaded_title, LoadedTitle},
};
use once_cell::unsync::Lazy;

#[derive(Clone, Copy)]
enum Status {
    Needed,
    Calibrating,
    Verifying,
    Success,
    Failed,
}
impl Status {
    fn name(self) -> &'static str {
        match self {
            Self::Needed => "CALIBRATION NEEDED",
            Self::Calibrating => "CALIBRATING",
            Self::Verifying => "VERIFYING",
            Self::Success => "SUCCESS",
            Self::Failed => "FAILED / MANUAL CONTROL",
        }
    }
}

/// Copy only values already observed by the normal Crystal frame update and
/// existing plugin trackers. No emulator/game call, RNG read or DIV measurement.
pub fn snapshot(state: u16) -> Snapshot {
    let div = hook::measured_div();
    Snapshot {
        state,
        advance: hook::rng_advance(),
        adiv: (hook::add_div_tracker().index(), (div >> 8) as u8),
        sdiv: (hook::sub_div_tracker().index(), div as u8),
    }
}
struct Helper {
    status: Status,
    note: &'static str,
    filter: Filter,
    row: u8,
    page: u8,
    preset: u8,
    party_count: u8,
    start_snapshot: Option<Snapshot>,
    elapsed: u32,
    last_advance: u32,
    calibration: Calibration,
    sample_index: usize,
}
impl Default for Helper {
    fn default() -> Self {
        Self {
            status: Status::Needed,
            note: "English Crystal VC - manual timing",
            filter: Filter::default(),
            row: 0,
            page: 0,
            preset: 0,
            party_count: 0,
            start_snapshot: None,
            elapsed: 0,
            last_advance: 0,
            calibration: Calibration::default(),
            sample_index: 0,
        }
    }
}
// Same single-threaded overlay ownership as the existing Crystal UI. No RNG
// observer or pause-resume callback accesses this state.
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
        self.start_snapshot = None;
    }
    fn start(&mut self, reader: &Gen2Reader, now: Snapshot) {
        let count = reader.party_count();
        if let Err(reason) = preflight(count, reader.dratini_received(), reader.dratini_prompt()) {
            self.stop(reason);
            return;
        }
        if !self.filter.possible() {
            self.stop("Gender and Attack filters conflict");
            return;
        }
        if !now.tracked() {
            self.stop("Wait for both existing DIV trackers");
            return;
        }
        if self.calibration.full() {
            self.stop("Calibration log full; clear to retry");
            return;
        }
        self.party_count = count;
        self.elapsed = 0;
        self.last_advance = now.advance;
        self.start_snapshot = Some(now);

        self.page = 1;
        self.status = Status::Calibrating;
        self.note = "Release keys; press A once to claim";
        // Requests the existing host pause loop, without changing guest state.
        // This is not a calibrated target or an exact input-time snapshot.
        pnp::request_pause();
    }
    fn finish(&mut self, reader: &Gen2Reader, now: Snapshot) {
        let Some(start) = self.start_snapshot.take() else {
            return;
        };
        let mon = reader.party(self.party_count);
        let dvs = Dvs((mon.atk << 4) | mon.def, (mon.spe << 4) | mon.spc);
        let (level, move4) = reader.party_level_move4(self.party_count);
        // Collect all natural DV outcomes, including non-shiny and missing F5.
        // Filtering calibration by the desired outcome would bias the evidence.
        let result = self.calibration.record(
            Sample {
                start,
                end: now,
                party_count: self.party_count,
                dvs,
                move4,
            },
            mon.spec_index,
            level,
        );
        if result != CaptureResult::Stored {
            self.stop("Invalid receipt; no sample recorded");
            return;
        }
        self.sample_index = self.calibration.len() - 1;
        self.page = 2;
        match verify(mon.spec_index, level, dvs, move4, self.filter) {
            Verification::Success => {
                self.status = Status::Success;
                self.note = "Natural gift matches filters; move4 F5";
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
                self.status = Status::Needed;
                self.note = "Sample saved; receipt filters not met";
            }
        }
    }
}
pub fn cancel() {
    if english() {
        let h = unsafe { helper() };
        if h.start_snapshot.is_some() {
            h.stop("Cancelled; manual control");
        }
    }
}

/// Ordinary overlay update only. No exact claim marker or generation offset is
/// inferred: this interval also includes input latency, text and nickname time.
pub fn tick(reader: &Gen2Reader, now: Snapshot) {
    if !english() {
        return;
    }
    let h = unsafe { helper() };
    if h.start_snapshot.is_none() {
        return;
    }
    h.elapsed = h.elapsed.saturating_add(1);
    let advance = now.advance;
    let count = reader.party_count();
    let received = reader.dratini_received();
    match claim_progress(
        h.party_count,
        count,
        received,
        reader.in_dragon_shrine(),
        h.elapsed,
        advance >= h.last_advance,
    ) {
        ClaimProgress::Failed => h.stop("Claim interrupted, reset or timed out"),
        ClaimProgress::Complete => h.finish(reader, now),
        ClaimProgress::Waiting => {
            h.last_advance = advance;
            if count == h.party_count + 1 {
                h.status = Status::Verifying;
                h.note = "Finish nickname and gift text normally";
            }
        }
    }
}

pub fn draw(reader: &Gen2Reader, locked: bool, now: Snapshot) {
    if !english() {
        pnp::println!("English Crystal VC only");
        return;
    }
    pnp::set_print_max_len(40);
    let h = unsafe { helper() };
    if !(locked || pnp::is_pressing(Button::X) && pnp::is_pressing(Button::Y)) {
        if pnp::is_just_pressed(Button::Y) {
            h.page = (h.page + 1) % 3;
        }
        if h.page == 0 {
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
                    4 => h.start(reader, now),
                    _ => {
                        h.stop("Cancelled; manual control");
                        h.calibration = Calibration::default();
                        h.sample_index = 0;
                        h.status = Status::Needed;
                        h.note = "Calibration cleared; capture a claim";
                    }
                }
            }
        } else if h.page == 2 && pnp::is_just_pressed(Button::Dup) {
            h.sample_index = h.sample_index.saturating_sub(1);
        } else if h.page == 2 && pnp::is_just_pressed(Button::Ddown) {
            h.sample_index = (h.sample_index + 1).min(h.calibration.len().saturating_sub(1));
        } else if pnp::is_just_pressed(Button::X) {
            h.stop("Cancelled; manual control");
        }
    }
    pnp::println!("ExtremeSpeed Dratini RNG");
    pnp::println!("{}", h.status.name());
    if h.page == 0 {
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
        pnp::println!("{} Capture calibration sample", cursor(4));
        pnp::println!("{} Clear calibration / cancel", cursor(5));
        pnp::println!("Up/Down select; X change; Y pages");
        pnp::println!("Stop at elder's final text:");
        pnp::println!("have recognized your worth.");
        pnp::println!("Prediction disabled: timing unobserved.");
        pnp::println!("Reset normally to retry a claim.");
        pnp::println!("L+R pause; L step; R resume");
    } else if h.page == 1 {
        pnp::println!("RNG {:04X} Advances {}", now.state, now.advance);
        pnp::println!("ADIV {:?} / {:02X}", now.adiv.0, now.adiv.1);
        pnp::println!("SDIV {:?} / {:02X}", now.sdiv.0, now.sdiv.1);
        pnp::println!("Nearest / distance / predicted: --");
        pnp::println!("PREDICTION DISABLED");
        pnp::println!("{}", h.calibration.prediction_blocker().explanation());
        pnp::println!("Natural samples {}/{}", h.calibration.len(), SAMPLE_CAPACITY);
        if let Some(start) = h.start_snapshot {
            pnp::println!("Captured {:04X} adv {}", start.state, start.advance);
        }
        pnp::println!("Y calibration log / X cancel");
    } else {
        let (matched, conflicts) = h.calibration.comparisons();
        pnp::println!("Natural samples {}/{}", h.calibration.len(), SAMPLE_CAPACITY);
        pnp::println!("Matching pairs {} / conflicts {}", matched, conflicts);
        pnp::println!("{}", h.calibration.prediction_blocker().explanation());
        if let Some(sample) = h.calibration.sample(h.sample_index) {
            pnp::println!("Sample {} / {}", h.sample_index + 1, h.calibration.len());
            pnp::println!("Start {:04X} adv {}", sample.start.state, sample.start.advance);
            pnp::println!(
                "A{:?}/{:02X} S{:?}/{:02X}",
                sample.start.adiv.0,
                sample.start.adiv.1,
                sample.start.sdiv.0,
                sample.start.sdiv.1
            );
            pnp::println!(
                "End {:04X} span {} (not offset)",
                sample.end.state,
                sample.end.advance.saturating_sub(sample.start.advance)
            );
            pnp::println!(
                "Actual DVs {:02X} {:02X} Move4 {:02X}",
                sample.dvs.0,
                sample.dvs.1,
                sample.move4
            );
            pnp::println!("Shiny {} / {}", sample.dvs.shiny(), sample.dvs.gender().name());
        } else {
            pnp::println!("Capture, claim normally, repeat.");
        }
        pnp::println!("No verified timing model; no search");
        pnp::println!("Up/Down samples; Y options; X cancel");
    }
    pnp::println!("{}", h.note);
    if h.note.contains("EXTREMESPEED MISSING") {
        pnp::println!("Dragon Master quiz condition not met.");
    }
}
