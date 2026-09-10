# ExtremeSpeed Dratini RNG (English Crystal VC)

This is a **safe calibration evidence and read-only receipt assistant**, for title
`0004000000172800`, version 0. Deterministic prediction, nearest-target search,
exact claim capture and a validated calibration model are unavailable. The existing
VBlank RNG/DIV tracker does not supply enough information to establish the gift's
four generation-time DIV reads. No timing offset is assumed.

## Workflow

1. Save normally before the gift with 1–5 party Pokémon. The Dragon Master's
   quiz must have no wrong answers for the game to award ExtremeSpeed.
2. On the later visit, stop at the elder's final page ending
   **“have recognized your worth.”** Leave it open. The helper reads the English
   script bank/position, map, party count and `EVENT_GOT_DRATINI` to check eligibility.
3. Open **Dratini RNG**. Defaults: Shiny ON, Gender Any, Attack Any shiny-valid.
   Up/Down selects a row; X changes it; Y switches to live observations.
   Presets: Fast (any shiny), Male shiny, Female shiny, Collector (Atk 15).
   These are receipt filters, not promises of faster or guaranteed shiny generation.
   Shiny OFF frees Defense/Speed/Special; Attack Any still means the eight
   shiny-valid Attack DVs. Female Dratini has Attack 0–7; male has 8–15.
4. The initial status is **CALIBRATION NEEDED**. Select **Capture calibration
   sample** with X and release the buttons. Both existing DIV trackers must have
   valid indices and the log must have space. The action copies the RNG value
   already read by the ordinary Crystal frame update, the existing plugin DIV
   indices/values and the plugin advance counter. It performs no additional RNG
   read and requests the existing pause loop. Status becomes **CALIBRATING**.
   Press physical **A once** to resume and dismiss the text, then complete the
   normal gift/nickname prompts. No input callback, marker or injection is used.
   The snapshot precedes the pause boundary; it does not observe the A edge or
   the generation-time DIV reads. Existing L+R pause, L step and R resume controls
   remain available, but manual stepping does not establish a calibrated offset.
5. The helper waits for exactly one new party member and the received-event flag,
   then reads the final Dratini's identity, level, DVs and move 4. Only level-15
   Dratini receipts can enter the calibration log. Non-shiny outcomes and missing
   ExtremeSpeed are retained too: filtering evidence by desired DVs would bias it.
   SUCCESS separately requires the selected DV/shiny/gender filters and move 4 F5.
   A matching shiny without F5 displays **SHINY OK - EXTREMESPEED MISSING** with
   the quiz explanation. A normal nonmatching receipt is still saved as evidence
   and returns to CALIBRATION NEEDED. Invalid/interrupted receipts are not saved.
6. Reset normally to repeat the natural claim, without saving a gift you wish to
   discard. Keep the same save/setup and input procedure when comparing trials.
   Up to 16 samples live in plugin RAM; restarting the title/plugin loses them.
   The log never evicts a conflicting observation automatically. Clear calibration
   explicitly to begin a new batch. Y cycles options, live state and calibration
   log; Up/Down on the log selects a sample. Each sample includes its observed
   start RNG/DIV/counter, completion RNG/counter, real DV bytes, and move 4.
   The completion span includes waiting, text, nickname and polling latency and
   is labeled **not offset**. Samples with equal observed RNG/DIV indices/values
   and party count are compared. The plugin counter is bookkeeping, so it is not
   part of that comparison key. Matching/conflicting *pair* counts are displayed;
   they are not probabilities or independent validation counts.
7. X on the live page cancels. Leaving/hiding the helper cancels pending work.
   Invalid party changes, leaving the shrine, an observed counter reset, or over
   7200 overlay updates without completion stop verification. These checks cannot
   detect every reset or external state change; cancel before changing the setup.
   Cancellation does not automatically resume the existing pause loop.

## Prediction gate and missing information

The calibration log has three explicit blocking diagnoses:

- Fewer than three valid natural claims: **Need multiple natural claims**.
- Equal observed state with differing real DVs: **Same observed state, different
  DVs**. Conflicts take priority, even before three samples, and remain in the log.
- Otherwise: **Claim timing / DIV reads unobserved**. Three is a minimum evidence
  count for this diagnostic, never a prediction-unlock threshold.

Repeated matching samples establish only observed repeatability. The key omits
unobserved input phase, sub-frame divider phase, interrupt placement and execution
between capture and GivePoke. Different setup can also explain a conflict. None
of those quantities is recovered by counting VBlanks until receipt. Outcomes at
different observed states do not independently validate a common timing model.

Even with a known pre-Random RNG state, the two output DV bytes alone do not
identify all four DIV operands. In the Random equations below, choosing the add
DIV determines its carry, after which a subtract DIV can be chosen modulo 256 to
produce the observed output byte. The second call has the same ambiguity. The
physical emulator cycle relationship must constrain those operands; it cannot be
inferred uniquely by selecting an arbitrary affine family that fits a few claims.
The frame snapshot also precedes that unknown pre-Random state. This algebraic
ambiguity is not a claim that every operand combination is physically reachable.

Consequently this implementation has **no supported prediction path, nearest
search, target distance or READY transition**. It does not extrapolate, use a
starter offset, fit a guessed gift offset, or treat a sample-count threshold as
proof. Real-device samples can detect conflicts and document repeatability, but
the currently available observations cannot establish the missing physical timing
mapping. A future predictor would require an independently justified mapping from
these safe observations to the actual input/generation boundary, plus distinct
natural identification and held-out validation trials. No such evidence is
available here. The log is a safe diagnostic tool, not a promise that collecting
more samples will unlock shiny targeting.

All correlation is pure local computation: a fixed 16-entry array and at most
240 pair comparisons in a UI update. Capture and log operations allocate no
vectors. Existing small UI-formatting allocations remain unchanged in kind.
## Source inspection and actual RNG relationship

Sources inspected, pinned for reproducibility:

- [PokemonRNGGuides Gameboy RNG and DIV](https://github.com/zaksabeast/PokemonRNGGuides/tree/6a18c90bc47d5c24bd48fb6c4513f92ae6d64aa8/rng_tools/src/rng/gameboy),
  and its [starter predictor](https://github.com/zaksabeast/PokemonRNGGuides/blob/6a18c90bc47d5c24bd48fb6c4513f92ae6d64aa8/rng_tools/src/generators/gen2/starter.rs).
  The original DIV tracker is based on this GPL-3.0
  project. Its starter predictor has encounter-specific transforms and a TODO;
  none of its starter offsets or uncertainty combinations are reused here.
- pokecrystal at `7a7881d0d62e0ddbd82dcf10e7116807487ac651`:
  [DragonShrine.asm](https://github.com/pret/pokecrystal/blob/7a7881d0d62e0ddbd82dcf10e7116807487ac651/maps/DragonShrine.asm),
  [dratini.asm](https://github.com/pret/pokecrystal/blob/7a7881d0d62e0ddbd82dcf10e7116807487ac651/engine/events/dratini.asm),
  [givepoke script command](https://github.com/pret/pokecrystal/blob/7a7881d0d62e0ddbd82dcf10e7116807487ac651/engine/overworld/scripting.asm),
  [GivePoke/TryAddMonToParty](https://github.com/pret/pokecrystal/blob/7a7881d0d62e0ddbd82dcf10e7116807487ac651/engine/pokemon/move_mon.asm),
  [Random](https://github.com/pret/pokecrystal/blob/7a7881d0d62e0ddbd82dcf10e7116807487ac651/home/random.asm),
  [VBlank](https://github.com/pret/pokecrystal/blob/7a7881d0d62e0ddbd82dcf10e7116807487ac651/home/vblank.asm).

The shrine runs `waitbutton`, text, sound/waitsfx, then `givepoke DRATINI, 15`.
`Script_givepoke` calls `GivePoke`, which calls `TryAddMonToParty` (the current
name of the relevant GiveMon logic). Outside battle it executes `call Random;
ld b,a; call Random; ld c,a`, storing B as Atk/Def and C as Spe/Spc. Before the
first call, `and a` clears carry. The first Random's subtraction borrow is carried
into the second Random, because `ld b,a`, CALL, PUSH and POP do not change flags.

For each Random, with state bytes `add, sub` and carry `c`:

```
t        = add + ADIV + c
add'     = t mod 256
u        = SDIV + (t > 255)
sub'     = (sub - u) mod 256
carry'   = sub < u
return A = sub'
```

In contrast, existing PokeReader advances count the separate RNG operations in
`VBlank_Normal` (DIV immediate-byte PCs `02B6` and `02BE`). This is NOT a count
of every Random call. The normal VBlank path clears carry before its update.
The two gift calls read DIV at immediate-byte PCs `2F8E` and `2F96`; they are not
two ordinary VBlank advances. Input latency, text/sound work, interrupt placement,
divider phase and emulated cycles separate A from these four reads. Current
overlay RNG/DIV snapshots alone do not statically determine all of those phases.

The earlier affine calibration/search implementation depended on additional
Random-hook observations. It has been removed, including its duplicate local
RNG/DIV simulator. The new evidence log does not restore that model. Inferring a unique safe generation offset from only the frame
observation and resulting two DV bytes is not established. Manual observations
therefore never become a calibrated model.

After GivePoke and nickname handling return, the game itself uses the quiz result
in `GiveDratini` to assign Wrap/Thunder Wave/Twister/ExtremeSpeed, then sets
`EVENT_GOT_DRATINI`. All receipt checks occur afterward in the overlay update.

## Strict legitimacy audit

The audit compares the whole feature branch with main at
`155de39c4b69de89a0f81381c60861bc21d10364`, including the actual call paths, not just
symbol searches. The fix also removes both the RNG observer and pause-resume
callback from the previous implementation.

**No Dratini code is reachable from either Crystal RNG instrumentation hook.**
`reader_core/src/crystal/hook.rs` and `3gx/includes/pokereader.h` are restored to
main's contents. There is no `observe_random` or `crystal_timing_resume` symbol.
Dratini state is owned only by the existing single-threaded overlay frame path.
Snapshot construction, UI, eligibility and verification run there. No Dratini
observer, guest call, DIV measurement, input callback or generation hook remains.
The snapshot copies DIV indices/values from the existing plugin tracker; it never
calls `reader.div()`. The existing frame RNG read is passed by value into the helper; capture, completion
and live display never re-read RNG. Eligibility and party reads use existing reader
methods from the overlay, not a timing-sensitive intercepted RNG path.

Existing instrumentation (unchanged from main):

| Existing ARM hook | Existing behavior | Dratini change |
|---|---|---|
| `001A8360` | Copies r0 into plugin cycle accumulator | None |
| `001AF17C` | Observes normal VBlank DIV reads at PCs 02B6/02BE and updates plugin trackers | None |

The existing branch-hook installer writes ARM BL instructions, and the existing
C framebuffer/HID integration also contains runtime instrumentation. Those paths
are unchanged; this feature adds no runtime patch. Register preservation alone
is not a proof of the closed-source VC emulator's timing noninterference. No
universal proof or real-device timing equivalence is claimed for the legacy
instrumentation or overlay reads. The established boundary is narrower: this
feature adds no execution to the intercepted RNG paths and makes no guest-data
writes or RNG/DIV changes.

Per-file review of the final branch changes:

| File | Change and write assessment |
|---|---|
| `3gx/includes/pnp.h` | Declares the host pause request; no game access. |
| `3gx/sources/main.c` | Adds only `host_request_pause`, assigning existing plugin `is_paused`; permitted execution control, no guest write or input injection. Resume/step paths restored to main. |
| `README.md` | Documents manual scope and limitations; no runtime effect. |
| `docs/extremespeed-dratini.md` | Workflow, research and this audit; no runtime effect. |
| `reader_core/src/crystal/dratini.rs` | Overlay-only explicit capture, calibration log UI, manual pause request and receipt checks. Assignments affect plugin state; game accesses are reads. No observer, generation-time capture or predictor. |
| `reader_core/src/crystal/dratini_rng.rs` | Pure fixed-capacity evidence storage/comparison, filtering and receipt/progress validation, with tests. No game access, allocation, RNG simulation or prediction/search model. |
| `reader_core/src/crystal/frame.rs` | English menu and overlay dispatch; passes the already-read RNG value and tracker copies into the helper; no new game writes. Existing reset resets a plugin counter only. |
| `reader_core/src/crystal/mod.rs` | Registers two Rust modules; no game access. |
| `reader_core/src/crystal/reader.rs` | Read-only party count, received flag, shrine/prompt and level/move accessors. Obsolete setup fingerprint removed. No quiz access added. |
| `reader_core/src/pnp/bindings.rs` | Pause declaration/test stub and existing test-stub cfg adjustments. No new write binding. Existing write stubs remain unchanged in effect. |
| `reader_core/src/pnp/input.rs` | Wraps the host pause request only; no controller injection or game write. |
| `reader_core/src/crystal/hook.rs` | No remaining difference from main. Dratini observer call removed. |
| `3gx/includes/pokereader.h` | No remaining difference from main. Pause callback declaration removed. |

No added path calls `pnp::write`, `write_mem`, `host_write_mem` or an equivalent
write API. No Pokémon, WRAM/SRAM, party, DV/species/move/PP, save, RNG/seed, DIV,
hRandomAdd/hRandomSub, event, script or quiz assignments exist in the feature.
Game-state observations use the existing read APIs. The only new host control
call requests an allowed pause. Physical controller input remains unchanged.

PASS — no Dratini-specific nested emulator/game calls in timing-sensitive hooks, no game-data writes, no RNG/DIV modification.

## Validation

Unit tests exhaust all 65,536 DV combinations for shininess and cover gender,
Attack filters/presets, eligibility, partial receipt/completion, identity, level,
move 4, reset/timeout and interruption guards. Calibration tests cover invalid
tracker indices, invalid receipts, unfiltered non-shiny/missing-move evidence,
bounded storage, conflict retention, equal-state comparison across plugin counter
changes, distinct-state trials and the inability of matching samples to unlock
prediction without the missing timing relationship. Test fixtures are synthetic
and do not constitute real-device calibration or proof of timing accuracy.

The fork's CI runs `make lint`, `make test` and `make`, uploading
`out/default.3gx`. Host tests use stubs; the release build does not. Neither a
passing build nor consistent test fixtures imply that prediction is supported.
