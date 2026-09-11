# `extern()` naming a symbol that exists nowhere: refused at the reference (EMP-EXTERN-UNKNOWN-GREEN), 2026-09-11

**Provenance.** Sigil base `2c5b3608` on branch `parcel/emp-extern-unknown-name`. Reference
tree: a private worktree of aeon at `ec640bcf` (the provenance tip's `aeon_rev`), provisioned
by `scripts/provision-aeon-ref.sh`, positive witness `repin --check` printed `pins.rs
unchanged`. The aeon `extern(` enumeration was read at aeon `origin/master` = `e4b4f38f`.
Routed finding: `2026-09-11-aeon-lens-pins-findings.md`, finding 1 (aeon measured at
`cd075f2d`, mechanism not established there).

## 1. Reproduction, before any change

Every probe ran on the base binary built from `2c5b3608` (`sigil --version` says clean at
`2c5b3608`). Each reference-tree edit was restored and the restore proven with
`git status --porcelain` (0 lines).

| probe | where | tool | result |
|---|---|---|---|
| R1 | `z80_sound_driver.emp:113` `const YM_ADDR_TO_DATA_MIN_T = extern("NO_SUCH_SYMBOL_LENS_PIN")`, read by the driver's eight `ensure(cycles(..) >= YM_ADDR_TO_DATA_MIN_T)` guards | `emit_sound_blob` | exit 0, both blobs `cmp`-identical to the provisioned ones |
| R2 | same file, item-position `ensure(extern("NO_SUCH_SYMBOL_LENS_PIN") == 1, ..)` | `emit_sound_blob` | exit 0, both blobs identical |
| R3 | 68000: the same body-position `ensure` inside `Vectors` (`engine/system/vectors.emp`) | `sigil build --native --game sonic4` | exit 0, ROM `b09ccd65/820229` = the golden |

Frontend unit probes on the same code (`lower_module`, then `resolve_layout`,
`check_link_asserts` and `link` over the module alone):

| shape | refused? |
|---|---|
| 68000 item-position `ensure(extern("NO_SUCH") == 1)` | yes, the generic "not defined in this link" guard message |
| 68000 body-position `ensure(extern("NO_SUCH") == 1)` | **no, green** |
| 68000 `const X = extern("NO_SUCH")`, never read | no (never evaluated) |
| 68000 const read by a body guard | **no, green** |
| Z80 item-position | yes |
| Z80 body-position `ensure(cycles(.a, .b) >= X)` | **no, green** |
| 68000 operand `move.w #extern("NO_SUCH"), d0` | yes, `unresolved symbol ... for fixup` |

**The finding reproduced, and it is not Z80-resident only.** R3 is a 68000 module in the
map-driven build.

## 2. Mechanism, established from the code and the probes

1. `extern(name)` evaluates to `Value::LinkExpr(Expr::Sym(name))` with no existence check.
   That is by design: only the link knows every symbol.
2. Each consumer of that value either decides it or does not:
   - an operand or data cell becomes a fixup; `link` refuses an unresolved one;
   - an `equ` becomes an `EquSym`; `resolve_layout` refuses an unresolvable one;
   - a comptime-required position refuses any `LinkExpr` (`[here.provisional]`);
   - an `ensure` whose condition becomes link-time defers to a `LinkAssert`, and every
     map-build path (canonical sonic4 included, through `resolve_chained`), `--check`,
     seam-2 and the CLI run `check_link_asserts`, which refuses an unresolved symbol (the
     inapplicable-guard allowlists are empty). **Except:**
3. **`eval_proc_body_env` never drained the body evaluator's `link_asserts`.** A
   body-position `ensure` over a link-time value was dropped with the evaluator, for every
   CPU. The driver's eight cycle guards are this shape. This is the mechanism of R1 and R3.
4. **seam-1's `lower_one` kept only the section**, discarding `module.link_asserts`, and
   seam-1 never called `check_link_asserts`. Resident item-position guards were dropped
   too. This is R2.
5. Consts are lazy: `const X = extern("NO_SUCH")` that nothing reads is never evaluated, so
   its `extern()` is never seen by any stage.

Aeon's reading (seam-1 builds extern proc stubs from `pub proc` only) is accurate as the
reason `use engine.sound_fm.{YM_ADDR_TO_DATA_MIN_T}` fails in a resident module (see section
7), but it is not the mechanism of the green-on-unknown-name case. That is items 3 and 4.

## 3. The fix (no language surface)

- `sigil-ir`: `LinkAssert` gains `kind: AssertKind { Condition, ExternDefined }` and the
  constructor `LinkAssert::extern_defined(name, span)`.
- `eval_extern` records one `ExternDefined` assert per (name, call site). It rides the
  existing `LinkAssert` channel, so every composer that already calls `check_link_asserts`
  checks it with no new call site.
- `check_link_asserts` decides references first. An undefined one is, verbatim:

  ```
  [extern.unknown] `extern("NAME")` names a symbol not defined in this link: no label, equ or supplied stub is called `NAME`
  ```

  at the `extern()` call's own span, once per call site (`sigil_link::EXTERN_UNKNOWN_ID` is
  the id). A condition whose every unresolved leaf was refused that way adds no second
  diagnostic. A reference passes on any value, 0 included.
- Proc, dispatch-body and script lowering take the body's link asserts through
  `eval_proc_body_lowering` and push them onto the module. `eval_proc_body` and
  `eval_proc_body_env` serve analyses that never link and still drop them (documented).
- seam-1: `lower_one` returns the module's asserts; `native_blob_checked` decides them per
  module against the linked blob and returns an `Err` located as `path:line:col`;
  `emit_sound_blob` uses it, so a refusal is the emitter's error, not a panic.
- A guard-message placeholder is lexed from the message text and has no file span, so the
  references it makes are attributed to the guard. `span()` truncates link asserts like it
  truncates diagnostics, keeping it a pure query.
- A reference record is not a guard: the census (`GuardCensus`), `is_drift_guard` and
  `sigil test`'s leak check count `Condition` asserts only. The native drift verdict fails
  on `[extern.unknown]` with located lines before its inapplicable partition.

**Covered:** 68000 and Z80; map-built modules at item position and in proc, dispatch-body
and script bodies; resident seam-1 modules at item and body position; seam-2 modules; guard
messages; values nothing reads.

**Not covered, by construction:** an `extern()` that is never evaluated (a const nothing
reads, a comptime fn never called, a gated-off `if` arm); `sigil test` bodies, which never
link; analysis walks. The typed declaration `extern NAME: Type` reads as a `Value::Label`
(the D-PP.3 residual) and gets no reference record: every consumer of that value is now
decided, so only a read nothing consumes goes unchecked (aeon has two such declarations,
`SFXID_RING_RIGHT` and `SFXID_RING_LEFT` in `sound_api.emp`, both consumed). `bankid()` and
`winptr()` share `extern()`'s argument contract and are likewise left without a record.

**Behaviour that changed besides the refusal:** a body-position `ensure` over a link-time
value is now decided at link, where it was dropped. `check_only_census`'s undefined-extern
test pinned the old outcome (an undefined extern's guard counted "inapplicable"); it now pins
the refusal at `probe/census.emp:9:8` and `:11:8`.

## 4. The aeon `extern(` enumeration

**Read at** aeon `origin/master` = `e4b4f38f` after `git fetch`:
`git -C aeon grep -n 'extern(' origin/master -- '*.emp'` gives 374 lines. **Positive
control:** the search finds `engine/system/z80_init.emp:94`, the use aeon named. 52 of the
374 are comment lines; the other 322 are code in 52 files and name 165 distinct symbols
literally. Some sit inside guard-message strings as `extern(\"NAME\")`: a deferred guard
evaluates those placeholders when it defers, so under the fix they are checked references
too. Every placeholder name also appears in a condition. Two sites compute their name:
`engine/objects/objdef.emp:69` `extern(code)` and `:74` `extern(map)`, fed by each object
definition.

Between the provenance tip `ec640bcf` and `e4b4f38f`, the only new `extern(` code is four
item guards in `engine/system/dma_queue.emp` over `DMA_Critical`, `DMA_Critical_End`,
`DMA_Important`, `DMA_Important_End`, `DMA_Deferrable`, `DMA_Deferrable_End`, `DMA_Queue`
and `DMA_Queue_End`.

**The resident sound modules use `extern()` nowhere in code.** The three hits in
`z80_sound_driver.emp`, `sound_sequencer.emp` and `sound_fm.emp` are comments (the ones that
warn against respelling the YM floor as `extern(...)`). The new seam-1 verdict therefore
cannot refuse an existing aeon use; what it newly decides there are the resident modules'
existing deferred guards.

**`engine/system/z80_init.emp:94`** `ensure(extern("Z80_IDLE_SIZE") == 40, ..)`, with
`:38` `const Z80_RAM_SIZE = extern("Z80_RAM_END") - extern("Z80_RAM")`. The module is
lowered only in sound-off shapes (`games/sonic4/map.toml`: the hole `filled_by =
"engine.z80_init"`, `when = "sound_off"`), so among the four shapes only demo evaluates
it. **Resolution: resolves.** Both demo shapes built green with the refusal on (section 6),
and a positive control proves the site is evaluated there: with the name edited to
`Z80_IDLE_SIZE_LENS_PIN` in the reference tree (line 73 at `ec640bcf`),
`sigil build --game demo` exits 1 with

```
error: native build (demo plain): extern() names a symbol no module in this link defines: 1 error(s):
  .../engine/system/z80_init.emp:73:8: [Error] [extern.unknown] `extern("Z80_IDLE_SIZE_LENS_PIN")` names a symbol not defined in this link: no label, equ or supplied stub is called `Z80_IDLE_SIZE_LENS_PIN`
```

(restored, `git status --porcelain` 0 lines). Aeon's LS-16a reading stays true for sonic4:
the module is `[module.unreachable]` in both sonic4 shapes, so no sonic4 build evaluates it.

**The poison fixtures** (`poison_extern_equate`, `_span`, `_addr`) are lowered only by
aeon's expect-fail lane through `--extra-entry`. With the fix binaries that lane passed
56 of 56 rows (0 FAIL) in both the plain and the debug build, including `link sentinel`
(the equate fixture) and both `LS16` rows, so `COLLECTED_PARK_SLOTS`, `Parallax_State`,
`Parallax_State_End` and `EndOfRom` resolve with the refusal on.

**Every other site, by what lowers it.** Per-shape `[module.unreachable]` lists
(`SIGIL_WARNINGS=full sigil build --game sonic4|demo [--debug]`, fix binary, ROMs to scratch,
all four exit 0) classify the 52 files. The lists name engine and `games/sonic4` files alike,
so a file they do not name in a sonic4 shape is inside that shape's closure; for demo, a
`games/sonic4` file's absence says only that demo does not scan it.

- 32 files are named in no shape's list, and 13 more are lowered by both sonic4 shapes only
  (the three animation tables, the three players and `player_common`, `act_descriptor`,
  `effects_scenes`, `mt_bank_blob`, `dust_spindash`, `test_solid`, `ojz_scroll_test`). Every
  one of them is lowered by a shape that built green with the refusal on, the two computed
  names in `objdef.emp` included. They resolve.
- `z80_init.emp`: demo shapes only, resolves (above).
- Six files no shape lowers:
  - `games/sonic4/data/sound/mt_bank.emp` (`SONG_COUNT`, `SONG_MOVINGTRUCKS`) and
    `sfx_blob_win_tab.emp` (`SFX_TABLE_LEN`) are lowered by seam-2 (`seam2.rs:1161` through
    `emit_mt_bank`, reached from the `emit_sound_blob` bin via `emit_mt_artifacts`;
    `seam2.rs:747`), which fails on any link-assert error, reference records included.
    `emit_sound_blob` ran green inside all four builds: they resolve.
  - The three poison fixtures: lowered only by aeon's expect-fail lane, which passed
    (above): they resolve.
  - `engine/debug/sound_debug.emp` (`Dynamic_Live`, `Sound_Dbg_Mirror`, `Z80_RAM`): not
    lowered by the four shapes, but lowered by the off-canonical `config_a` profile (debug +
    hotkeys + mirror; `sound_debug` is absent from its unreachable list), which builds green
    with the refusal on and matches its golden: they resolve. (`--extra-entry` cannot reach
    it: the module declares an `equ`, and an extra entry must mint no link symbol.)

The off-canonical profiles, same fix binaries, `sigil build --config-a|--config-b|--lean`:
`config_a` `9a63c8c1/846881`, `config_b` `ed60e162/620727`, `lean` `0b5d25f5/773136`, each
exit 0 and equal to its provenance-tip golden, none with an `[extern.unknown]` line.
`config_b` (sound off) is a second shape that lowers `z80_init`.

**Verdict for the stop rule: no aeon `extern(` use at `ec640bcf` names a symbol the fix
refuses.**

**At aeon `origin/master` `e4b4f38f`** (the enumeration's revision; the private tree moved
there, then back to `ec640bcf`), the four shapes through aeon's own `./build.sh` with its
default lanes and the fix binaries: all exit 0, pytest 2428 passed and 2 skipped, and
expect-fail 56 PASS and 0 FAIL in every shape. That includes the four new `dma_queue.emp`
guards. There is no golden at that revision, so byte-neutrality there is measured
against the base binary. `sigil build` with the base and the fix binaries gives identical ROMs,
equal to what `build.sh` wrote: s4 `064e0ae6/821123`, s4_debug `cb0e2019/847389`, demo
`bc230dd7/97075`, demo_debug `523e0287/103359`. **The stop rule does not trigger at
master either.** The tree then went back to `ec640bcf`. There, `NO_LINT=1 ./build.sh` for the
four shapes rewrote every golden (`b09ccd65`, `1b7fe316`, `0ad17404`, `2565ece2`),
`git status --porcelain` shows 0 lines, and `repin --check` prints `pins.rs unchanged`.

## 5. Tests and red-first evidence

Added:

- `crates/sigil-frontend-emp/tests/extern_unknown_name.rs` (8): the unknown name refused
  once at its own span in a 68000 body guard, in a Z80 const read by two body cycle guards
  (the reported shape), in an item guard (68000 and Z80), in a comptime result nothing reads,
  and in a guard message (attributed to the guard); a defined name resolving in every
  position in both module kinds; an equ of 0 resolving; a false body guard over a defined
  name firing (68000 and Z80).
- `crates/sigil-harness/tests/seam1_link_verdict.rs` (3, reference-dependent): the reported
  case through the real driver fed from memory (`with_resident_source_override`, the tree is
  never edited), refused at the const's line and column with exactly one `[Error]`, both
  shapes; a resident item guard refused; a defined blob label (`Sequencer_Frame`) resolving
  with pristine bytes, and a false guard over it firing at its own line, both shapes.
- `check_only_census.rs`: the undefined-extern test now pins the refusal; the drained-count
  test counts the two kinds.
- `link_assert_reporting.rs` (found by the scoped strict suite, not by my targeted runs): its
  probe counts `Condition` asserts, and its inapplicable test, which pinned the removed
  outcome, now pins the refusal at `probe/guarded.emp:4:8` and `:6:8`; its real-drift half
  defines both truths so the refusal cannot mask the drift. Red under m1 re-applied on
  `c7312885` (`declared_chain_verdict_refuses_an_undefined_extern_and_still_reports_real_drift`
  FAILED, the other 4 green), restored and proven.
- `scripts/nightly_source_gates.sh`: `seam1_link_verdict` joins the source-only run list
  (the lane's audit refused the tree while it was unclassified).

Each mutation was applied, quoted from disk, run, then restored with
`git restore --source=HEAD` and proven with `git diff --quiet HEAD` (base for m1 to m4:
`20635c05`; m5 was re-run on `a387d774` after the seam-1 gate was tightened).

| mutation (quoted from disk) | the half-fix it models | red |
|---|---|---|
| m1 `builtins.rs:1005: if false && !self.link_asserts.contains(&reference) {` | refuse only inside `ensure` (no reference record) | 10: 6 frontend, 2 census, 2 seam-1; `an_unknown_extern_whose_value_nothing_reads_is_refused` gets no diagnostic at all |
| m2 `proc.rs:140: drop(asserts); // MUTATION m2` | proc-body asserts still dropped | 5: the 68000 body, Z80 cycle-guard, false-body-guard, defined-in-every-position tests, and the seam-1 reported case |
| m3 `seam1.rs:762: let _ = (&asserts, debug); // MUTATION m3` | refuse in 68000 and map-built modules but not resident Z80 | all 3 seam-1 gates; the frontend tests stay green, which is exactly why this half-fix needs its own gate |
| m4 `proc.rs:141: if ctx.cpu != sigil_ir::backend::Cpu::Z80 { builder.push_link_assert(a); }` | drain 68000 proc bodies only | 4: the Z80 cycle-guard test, the Z80 arms of the false-body-guard and defined-name tests, and the seam-1 reported case; the 68000 body test stays green |
| m5 `lib.rs:446: if false && !missing.is_empty() && ...` | one root cause, several diagnostics | 3 frontend and the census refusal test; on the first run the seam-1 gate stayed GREEN (it counted `[extern.unknown]`, the duplicates carry other wording), so it was tightened to count every `[Error]` line and re-run red under m5 |

## 6. The four engine ROM shapes

Built in the private reference tree at aeon `ec640bcf` by the fix binaries (`sigil --version`:
`20635c05`, clean at capture), through aeon's own `./build.sh` with its default lanes on
(pytest, the expect-fail lane). Each ROM was proven written by that build (newer than a
per-shape marker file). Goldens from the provenance tip `ls12-parallax-discards-refused`.

| shape | command | exit | CRC32/size | golden | lanes |
|---|---|---|---|---|---|
| s4 | `./build.sh` | 0 | `b09ccd65/820229` | `b09ccd65/820229` | pytest 2284 passed, 2 skipped; expect-fail 56 PASS, 0 FAIL |
| s4_debug | `DEBUG=1 ./build.sh` | 0 | `1b7fe316/846529` | `1b7fe316/846529` | pytest 2284 passed, 2 skipped; expect-fail 56 PASS, 0 FAIL |
| demo | `./build.sh demo` | 0 | `0ad17404/96863` | `0ad17404/96863` | pytest 2284 passed, 2 skipped; expect-fail 56 PASS, 0 FAIL |
| demo_debug | `DEBUG=1 ./build.sh demo` | 0 | `2565ece2/103185` | `2565ece2/103185` | pytest 2284 passed, 2 skipped; expect-fail 56 PASS, 0 FAIL |

All four equal the provenance tip. The tree was clean before and after (`git status
--porcelain` 0 lines), and `repin --check` printed `pins.rs unchanged` when the tree was
provisioned and again before the second suite run.

## 7. The language-surface half: options and costs (not implemented)

**What the missing half actually is.** Aeon's T1, `use engine.sound_fm.{YM_ADDR_TO_DATA_MIN_T}`
in the resident driver, fails `unknown name`. In the map build the same `use` works: the
resolve pass exports a `pub const` (`resolve/imports.rs`, `item_pub_name`) and injects a
clone into the consumer with its value folded at the definition site
(`resolve/mod.rs`, `collect_pub_comptime`). Seam-1 does not run that pass. It lowers each
resident file with a bare `lower_module`, injects derived `extern proc` stubs for `use`d
`pub proc`s (`use_import_stubs`), and seeds a fixed `-D` authority list from
`sound_constants.emp` (`resolve_consts`). So "a const does not cross a `use`" is a gap
between seam-1 and the language as the map build implements it, not a missing spelling.
Closing it still changes which resident programs are accepted, so it goes to the owner as
the brief says. Nothing here is implemented.

| option | spelling | what it costs |
|---|---|---|
| A. seam-1 honours `use` of a `pub const` as the map build does | none new; the resident dialect gains a feature the language already has | seam-1 grows a const-import path beside its proc stubs (or adopts the resolve pass for the five files); a const whose value comes from a seam-2 derivation (the `DacSampleTable` class) must be excluded or preset, as the size-only path already does; the per-module copies and aeon's `test_ym_floor_single_authority.py` retire; guards stay comptime |
| B. put the name on seam-1's injected authority list | none | move the const into `sound_constants.emp` (an aeon edit) and add it to the file spec's `const_names` (a sigil edit), once per shared name, forever: the friction aeon's own comments cite; guards stay comptime |
| C. `pub equ` in the owner, `extern()` in the consumers (possible with this fix alone) | none new | `pub equ YM_ADDR_TO_DATA_MIN_T = 8` must sit inside `section sound_fm { }` (a module-level equ lands in the default carrier section, which seam-1 discards); consumers write `const YM_ADDR_TO_DATA_MIN_T = extern("YM_ADDR_TO_DATA_MIN_T")`; the consumers' guards move from comptime to link time, so a comptime `if` over the value is refused as provisional; an aeon edit. Measured below |
| D. a typed `extern YM_ADDR_TO_DATA_MIN_T: Cycles` declaration in consumers | exists (L8) | the same equ requirement as C, plus a newtype; the value is a `Value::Label`, which this parcel gives no reference record (section 3), so a read nothing consumes would go unchecked |

**Option C, measured** in the reference tree at `ec640bcf` (both files restored afterwards
from their pre-probe copies, md5 equal to `HEAD`, `git status --porcelain` 0 lines):

| state | fix binary (`20635c05`) | base binary (`2c5b3608`) |
|---|---|---|
| `sound_fm.emp:150` `pub equ YM_ADDR_TO_DATA_MIN_T = 8` inside the section; driver `const YM_ADDR_TO_DATA_MIN_T = extern("YM_ADDR_TO_DATA_MIN_T")` | `emit_sound_blob` exit 0, both blobs byte-identical to the pristine ones | (not needed) |
| the same with the driver at `extern("YM_ADDR_TO_DATA_MIN_T") + 100` | exit 1, **all 8 driver cycle guards fire at their own lines** (`z80_sound_driver.emp:595:9`, `597:9`, `998:9`, `1153:9`, `1155:9`, `1387:9`, `1389:9`, `1391:9`, each with its own message) | exit 0, both blobs byte-identical: all eight dropped |

So option C gives aeon a single authority for the YM floor, decided in every consumer, with
this parcel alone. Its cost is the one in the table above: link-time rather than comptime
guards, and the equ has to sit inside the section.

## 8. Scoped suites and clippy

Strict (`SIGIL_STRICT_GATE=1`, `AEON_DIR` = the private tree at `ec640bcf`, witness
`pins.rs unchanged` before each run), `cargo test --release -p <crate> --no-fail-fast`,
each log stamped with its tree, HEAD and branch:

| crate | at `a387d774` | at `c7312885` |
|---|---|---|
| sigil-ir | exit 0, 52 passed, 0 failed | (unchanged since) |
| sigil-link | exit 0, 150 passed, 0 failed | (unchanged since) |
| sigil-frontend-emp | exit 0, 2676 passed, 0 failed | (unchanged since) |
| sigil-harness | exit 101, 467 passed, **6 failed**, 1 ignored | exit 0, 473 passed, 0 failed, 1 ignored |
| sigil-cli | exit 0, 790 passed, 0 failed, 1 ignored | (unchanged since) |

The six at `a387d774`: `link_assert_reporting` (`emitter_verdict_renders_every_failing_guard_located`,
`declared_chain_verdict_renders_every_failing_guard_located`,
`declared_chain_verdict_separates_inapplicable_from_real_drift`,
`agreeing_authority_passes_both_verdicts`) and `source_gate_classification`
(`every_selected_test_file_is_classified`, `the_derived_accessor_set_is_the_declared_guard_set`),
fixed in `c7312885` (section 5). `c7312885` changed one harness test and one script, so the
other four crates were not re-run.

`cargo clippy --release -p sigil-ir -p sigil-link -p sigil-frontend-emp -p sigil-harness
--all-targets -- -D warnings`: exit 0 at `20635c05`, `a387d774` and `c7312885`.

## 9. What this parcel concluded the brief and the routed finding got wrong

- **"Resident Z80 only."** The green-on-unknown-name case is general. A body-position
  `ensure` over any link-time value was dropped for every CPU (R3: a 68000 module in the
  map-driven sonic4 build). Resident modules add a second drop (seam-1 discarded their
  item-position guards as well).
- **The mechanism.** Aeon's hypothesis (seam-1 derives stubs from `pub proc` only) explains
  T1, not T3a. T3a is the two dropped-assert paths in section 2.
- **"The other half is language surface."** A `pub const` already crosses a `use` in the
  map build; seam-1 does not run that resolution. The missing half is seam-1 conformance to
  existing semantics, not a new spelling. It still changes what resident modules accept, so
  it is left to the owner as instructed (section 7).
- **`z80_init.emp:94` as aeon's Z80-side exposure.** It is a map-built Z80 module, not a
  resident one, so the seam-1 hole never applied to it. Its only silence is LS-16a (not
  lowered in sonic4). Under the fix it resolves in demo, demo_debug and `config_b`.
- **T2 as written** (`const X = extern("YM_ADDR_TO_DATA_MIN_T")`, "ROM identical"): a
  `pub const` is not a link symbol, so with the fix this spelling is refused rather than
  green. Measured at `ec640bcf` with the fix binary (restored after, 0 porcelain lines):
  `emit_sound_blob` exits 1, refusing the name at `engine/sound/z80_sound_driver.emp:113:31`
  with the `[extern.unknown]` message. Option C in section 7 is the spelling that does
  resolve. The driver and
  `sound_fm.emp` comments that say an
  `extern(...)` "builds green" describe the pre-fix compiler; they are aeon's to update.
