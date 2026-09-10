# ExtremeSpeed Dratini RNG (English Crystal VC)

This is a read-only, empirically calibrated timing assistant for title
`0004000000172800`, version 0. It cannot manufacture a shiny or change the quiz.
Other languages retain their existing menus. No Dratini timing offset is shipped.

## Workflow

1. Save normally before claiming the gift, with 1–5 party Pokémon. Answering the
   Dragon Master's quiz without a wrong answer is necessary for ExtremeSpeed.
2. Talk to the elder on the later visit. Stop at the final page ending
   **“have recognized your worth.”** Do not dismiss it yet. The helper checks the
   English script bank/position, map, party capacity and `EVENT_GOT_DRATINI`.
3. Open **Dratini RNG**. Defaults are Shiny ON, Gender Any, Attack Any shiny-valid.
   Up/Down selects a row; X changes it. Y switches between options and live data.
   Presets cycle through Fast, Male shiny, Female shiny and Collector (Atk 15).
   With Shiny OFF, Defense/Speed/Special are unrestricted; Attack Any still means
   the eight shiny-valid Attack DVs. Female Dratini has Attack 0–7; male has 8–15.
4. Select **Start / calibrate**, press X, and release all buttons. The existing
   pause loop is requested. Press physical **A once**, on its own, to resume and
   claim. Capture occurs inside that pause loop immediately before resumption,
   not when X was pressed. Finish the nickname prompt normally. The helper waits
   for the shrine's received-event flag, which is set after the moveset routine.
5. Reset using normal game controls without saving that claim, return to the same
   setup and repeat at different RNG states. Three distinct claims must identify
   one model; two additional distinct claims must validate it. Plugin calibration
   lives only in RAM; restarting the title/plugin loses it. Changes to options,
   party contents, trainer ID, quiz state or Dratini's Pokédex state invalidate it.
   Do not change the setup between samples. “Unmodelled RNG path” means this
   setup cannot currently be predicted; retain manual control rather than guessing.
6. After five accepted samples, reset normally again and use Start at the same
   prompt. Search examines at most 256 candidates per overlay frame, up to one
   million advances. It selects the first still-future matching candidate.
   RNG/DIV continuity is checked while approaching. L+R pauses, L frame-steps,
   and R resumes using PokeReader's existing controls.
7. At READY the helper requests pause. Release other buttons, then press **A
   once**. The actual pause-release snapshot must equal the target, or the helper
   reports failure/manual control. It never suppresses your physical input;
   a mistimed A can still naturally claim a non-target Dratini.
8. Complete the nickname prompt. SUCCESS requires newest party member Dratini,
   level 15, exactly the targeted DVs, requested filters, and move 4 `F5`.
   Correct shiny DVs without ExtremeSpeed show
   **SHINY OK - EXTREMESPEED MISSING** and the quiz explanation. Moves are never
   repaired. X on the live page cancels; leaving/hiding the helper cancels active
   work. A cancellation does not unpause the game automatically.

The search model is empirical, not a proof that every VC execution has the same
latency. No real 3DS calibration or hardware end-to-end success is claimed by the
unit tests. If the bounded model cannot fit the observations, this feature remains
a calibration/timing assistant. It has no write-based fallback.

## Source inspection and actual RNG relationship

Sources inspected, pinned for reproducibility:

- [PokemonRNGGuides Gameboy RNG and DIV](https://github.com/zaksabeast/PokemonRNGGuides/tree/6a18c90bc47d5c24bd48fb6c4513f92ae6d64aa8/rng_tools/src/rng/gameboy),
  and its [starter predictor](https://github.com/zaksabeast/PokemonRNGGuides/blob/6a18c90bc47d5c24bd48fb6c4513f92ae6d64aa8/rng_tools/src/generators/gen2/starter.rs).
  The DIV increment schedule and reference sequence are credited to this GPL-3.0
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

Calibration observes the first two Random calls after A through the existing read
observer. It rejects extra/misordered calls and any VBlank between those four
reads. It records the count of ordinary VBlanks preceding the first call (max
512), the pre-call RNG/DIV snapshot, and all four observed DIV bytes. The idle
forecast must exactly reproduce that pre-call snapshot; the above Random
arithmetic must then reproduce both real party DV bytes.

Each observed DIV byte is independently fitted to `slope * baseline + offset
(mod 256)` with slope in `{0,1,2}`. Baseline is the corresponding last normal
VBlank DIV, not a starter forecast. Both the constant offset and slope are inferred
from observed reads. This is a deliberately limited hypothesis family, not a
claim about universal hardware behavior. Conflicting observations clear the model;
ambiguity does not unlock search. At least three distinct captures are needed for
identification, followed by two distinct held-out captures. Search rolls both an
input-state cursor and a delayed cursor, so it does not replay hundreds of frames
or allocate a vector for every candidate. Receipt also checks the learned model
against the new observed trace and invalidates it on disagreement.

After GivePoke and nickname handling return, the game itself uses the quiz result
in `GiveDratini` to assign Wrap/Thunder Wave/Twister/ExtremeSpeed, then sets
`EVENT_GOT_DRATINI`. The verifier waits for this flag and a party count increase
of exactly one. It never mistakes the early, partially initialized party slot or
the normal moveset before `GiveDratini` for the final gift.

## Read-only and hook audit

No new runtime patch or hook is installed. The already-present Crystal setup
hooks two ARM BL call sites in the VC emulator:

| Existing hook | Observation | Feature change |
|---|---|---|
| `001A8360` | Copies `r0` to a plugin cycle accumulator | None |
| `001AF17C` | Observes GB DIV reads, PC and DIV trackers | For Random's two PCs, additionally copies four DIV bytes and a pre-call snapshot while a manual claim is armed |

`utils/hook_game_branch.rs` and the existing ARM trampoline save `r0-r12` and
the return address, dispatch the observer, then restore registers and execute the
original BL destination with its original inputs. The added observer receives no
mutable register/stack slice. It calls no memory-write helper and assigns only
plugin-owned fields. It does not replace a DIV read or a Random return value.
All added game accesses are reads of ordinary HRAM/WRAM through the existing
`Gen2Reader`/`gb_mem` reader, or reads of the existing emulator DIV pointer.
The emulator cycle accumulator hook, original destinations and trampoline are
unchanged. ARM condition flags are call-clobbered under the existing C ABI; the
observer does not write emulated Game Boy flags.

This establishes source-level noninterference with RNG values and Pokémon data:
no added assignment targets guest memory, emulated CPU state, function inputs or
return values; original operations still run. Observation costs host CPU time.
The repository does not contain the closed-source VC emulator, so a universal
proof of its wall-clock scheduling or behavior on every console is unavailable.
The helper consequently makes no unconditional deterministic-timing promise and
rejects observed drift. A real-device timing comparison is still needed to certify
that scheduling on a particular console; failed calibration must remain manual.

The only new host control API, `host_request_pause`, assigns the pre-existing
plugin `is_paused` boolean. The existing pause loop calls `crystal_timing_resume`
when a physical resume/step button is detected, before it resumes execution.
It does not change HID shared memory, inject a button, swallow an input or invoke
any Pokémon-generating routine. No new feature call reaches `pnp::write`,
`host_write_mem`, save/event writes, RNG reseeding, or any Pokémon edit API.

## Validation

Unit tests exhaust all 65,536 DV combinations for shininess, check all gender
thresholds and Attack filters, presets, carry/borrow/wrap arithmetic, the external
idle-RNG reference sequence, DIV exceptions, calibration ambiguity/duplicates/
holdouts/inconsistent paths, chunked nearest-target selection and receipt checks.
Host tests automatically use the existing test stubs; the 3DS release does not.
The fork's CI runs `make lint`, `make test`, and `make` and uploads
`out/default.3gx`. The artifact must still be calibrated on the user's real English
Crystal VC setup before claiming that its timing is reliable there.
