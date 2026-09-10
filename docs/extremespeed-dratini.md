# ExtremeSpeed Dratini RNG (English Crystal VC)

This is a **manual timing and read-only receipt assistant**, for title
`0004000000172800`, version 0. Deterministic prediction, nearest-target search,
exact claim capture and automatic calibration are unavailable. The existing
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
4. Select **Start manual claim** with X and release the buttons. The helper
   records an ordinary overlay-frame observation and requests the existing pause
   loop. It displays MANUAL TIMING, never READY or a predicted target.
   Press physical **A once** to resume and dismiss the pending text. Input is
   neither generated nor consumed by this feature. The observation precedes the
   actual pause boundary and is not an exact snapshot at A or at generation.
   Existing L+R pause, L frame-step and R resume controls remain available for
   manual timing. Additional normal text/nickname input may be required.
5. Finish the gift and nickname prompts normally. Verification waits for exactly
   one new party member and the received-event flag, set after the game's moveset
   routine. SUCCESS requires Dratini, level 15, selected DV/shiny/gender filters,
   and move 4 `F5`. It does not certify a predicted DV target or timing model.
   A matching shiny without move 4 F5 displays
   **SHINY OK - EXTREMESPEED MISSING** and explains the quiz condition.
6. Live data shows current RNG, existing ADIV/SDIV indices and values, advances,
   the last start/end observations and actual DVs/shiny/gender. The observed span
   includes waiting, text, nickname time and polling latency; it is **not a gift
   offset**. Only the last observation is retained in plugin RAM. No amount of
   manual observations unlocks prediction. Reset normally to retry, without saving
   a claim you wish to discard; the plugin cannot reset or restore the game.
7. X on the live page cancels. Leaving/hiding the helper cancels pending work.
   Invalid party changes, leaving the shrine, an observed counter reset, or over
   7200 overlay updates without completion stop verification. These checks cannot
   detect every reset or external state change; cancel before changing the setup.
   Cancellation does not automatically resume the existing pause loop.

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
RNG/DIV simulator. Inferring a unique safe generation offset from only the frame
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
calls `reader.div()`. RNG and party reads use existing reader methods from the
overlay, not from a timing-sensitive intercepted RNG path.

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
| `reader_core/src/crystal/dratini.rs` | Overlay-only observations, manual pause request, filters and receipt checks. Assignments affect plugin state; game accesses are reads. No observer, generation-time capture or predictor. |
| `reader_core/src/crystal/dratini_rng.rs` | Pure filtering and receipt/progress validation on copied arguments, with tests. No game access, allocation, RNG simulation or calibration/search logic. |
| `reader_core/src/crystal/frame.rs` | English menu and overlay tick/draw/cancel dispatch. Reuses existing tracker; no new game writes. Existing reset resets a plugin counter only. |
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

Unit tests exhaust all 65,536 DV combinations for shininess, check gender and
Attack filters/presets, conflicts and eligibility, and validate receipt identity,
level, filters and move 4. Progress tests cover partial party initialization,
completion only after the received flag, unexpected party changes, shrine exit,
counter reset and timeout. There are no predictor/calibration tests because those
capabilities have been removed, rather than retained behind an unproven model.
Host tests use test stubs; the 3DS release does not. The fork's CI runs `make lint`,
`make test` and `make`, uploading `out/default.3gx`. These checks do not establish
real-console timing accuracy; the feature remains a manual assistant.
