# Link-assert reporting (LS-16b, LS-16) and three answers for aeon

Branch `parcel/link-assert-reporting` off master `52bc7844`. Fix commit
`df454b0f`; this note is the commit after it. Byte-neutral by construction
(diagnostic channel and text only); the controller proves that at the landing
gate. Built and tested in `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-linkassert`.

## (i) LS-16b: a firing `extern()` guard reached the reader as a panic, exit 101

### The site set at 52bc7844 (every place a guard/assert diagnostic became a panic or a `{:?}`)

Panic sites (`crates/sigil-harness`):

| file:line | what it unwrapped |
|---|---|
| `src/native.rs:1158` | `require_reference_tree` precondition, `unwrap_or_else(panic!)` |
| `src/native.rs:1161` | `seam1::emit_sound_blob` (the line aeon hit) |
| `src/native.rs:1162..1172` | the six seam-2 emitters (`emit_dac_artifacts`, `emit_mt_artifacts`, `emit_sfx_artifacts`, `emit_seq_opcode_artifacts`, `emit_sound_tables_artifacts`, `emit_pitchtable_artifacts`), each `unwrap_or_else(panic!)` |
| `src/seam1.rs:1183`, `:1195` | `dac_sample_table_vma` / `sound_bank_id` (both call `seam2::sound_layout`, which links `mt_bank.emp` and can carry its guard failure). In the build and bin paths these are unreachable for a guard failure: `emit_sound_blob` runs `check_banked_carrier_drift` first (`seam1.rs:738`), which surfaces the same `sound_layout` error as an `Err`, and `sound_layout` caches only on success. Left as they are; listed so nobody rediscovers them. |
| `src/seam1.rs:514`, `:603`, `:691`, `:693` | `resolve_layout`/`link` failures inside `native_sound_blob` (not guard diagnostics; adjacent, same `{d:?}` shape). Untouched. |

`{:?}`-rendered guard lists (returned as `Err(String)`, then unwrapped above):
`src/seam2.rs:632` (dac_sample_tab size guard), `:800` (sfx co-residency/drift/span),
`:884` (seq_opcode_tab), `:953` (sound_tables_z80), `:1044` (movingtrucks_pitchtable),
`:1206` (mt_bank co-residency/drift).

`crates/sigil-cli`: no panic site. `run_build_native` prints
`error: native build ({label}): {err}` and exits 1 (`src/main.rs:2075`). The
`emit_sound_blob` bin (`crates/sigil-harness/src/bin/emit_sound_blob.rs`) also
never panicked: each emitter's `Err` prints as `error: emit_sound_blob (..) failed: ..`
with exit 1. The exit-101 aeon measured is the `sigil build` path, whose sound-on
profiles call `ensure_generated` (`native.rs:3612`, `:3776`). That is a
correction to the brief's framing ("the bin renders through the normal channel"):
the bin already did; the driver did not.

### Reproduction on the committed code (52bc7844)

A partial aeon tree read from aeon `origin/master` `0d64f534` (`git archive` of
`engine/sound engine/system games/sonic4/data/sound games/sonic4/config
games/sonic4/map.toml`, 89 files, into the on-disk target dir, never an aeon
worktree), with `games/sonic4/data/sound/mt_bank.emp:116` doctored
`const SONG_MOVINGTRUCKS: SongId = 1` to `= 2` so the guard at `mt_bank.emp:120`
(`ensure(extern("SONG_MOVINGTRUCKS") == SONG_MOVINGTRUCKS, ..)`) fires against
the carrier's `SONG_MOVINGTRUCKS = 1`. A four-line probe bin calls
`sigil_harness::native::ensure_generated(tree)`, the same entry `sigil build`
reaches. Before:

```
exit=101

thread 'main' (781945) panicked at .../crates/sigil-harness/src/native.rs:1161:59:
emit_sound_blob (blob): mt_bank co-residency/drift guards fired: [Diagnostic { level: Error, message: "SONG_MOVINGTRUCKS drifted from games.sonic4.sound_ids: 2", primary: Span { source: SourceId(0), start: 7569, end: 7703 } }]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

After (the probe calls the new `emit_generated` and renders its `Err` the way
`run_build_native` does):

```
exit=1
error: native build (sonic4): emit_sound_blob (blob): mt_bank co-residency/drift guards fired: 1 error(s):
  .../aeon-partial/games/sonic4/data/sound/mt_bank.emp:120:1: [Error] SONG_MOVINGTRUCKS drifted from games.sonic4.sound_ids: 2
```

The line number is aeon's own (`mt_bank.emp:120`), the level is the `[Error]`
token aeon's lane counts, and the message is the guard's own text.

### The fix

* `crates/sigil-harness/src/diag_render.rs` (new): `render_diag_lines` is THE
  renderer, one line per diagnostic, `path:line:col: [Error] message` where the
  span locates and `[Error] message @ Span{..}` where it does not;
  `link_assert_failure(what, diags, locate)` is the verdict over a guard family
  (`Ok` with no errors, else `Err("{what}: N error(s):\n" + every error
  rendered)`); `SourceTexts` registers each single-file `.emp` text under the
  `SourceId` it is parsed with, so the sfx co-link (two files) locates each
  guard in its own file.
* `seam2.rs`: `lower_emp_file` takes a `SourceTexts` and parses with
  `parse_file(src, id)`; the six guard sites call `link_assert_failure`.
* `native.rs`: `emit_generated(aeon) -> Result<(), String>` holds the emitter
  calls with `map_err`; `ensure_generated` is the panicking wrapper the gates use
  as a precondition step; the two build drivers propagate with `?`. The old
  `fmt_diag_list` is gone; the comptime `build_program` error path renders
  through `render_diag_lines` located by the manifest's `SourceIndex` (it was
  byte spans; the comment claiming no map was available was stale, the index is
  built two screens later in the same function).
* `tests/common/mod.rs`: the write-guard gate's body parser is re-pointed at
  `emit_generated`'s signature (it derives the emitter set from that body).

## (j) LS-16: real drift reported as `first Some(Diagnostic{..})`, no `[Error]`

Mechanism, by file:line: an `ensure` whose condition evaluates to a
`Value::LinkExpr` (an `extern()` anywhere in it) is deferred
(`crates/sigil-frontend-emp/src/eval/guards.rs:104..107`) into a
`sigil_ir::LinkAssert` (`guards.rs:189`); `sigil_link::check_link_asserts`
(`crates/sigil-link/src/lib.rs:364`) folds each one against the post-
`resolve_layout` symbol table and returns one `Diagnostic` per failing guard.
The declared-chain driver then partitioned them (`native.rs:3678..3688` at
52bc7844) and returned
`"declared-chain drift guard FIRED: {} error(s); first {:?}"`, with the other
N-1 printed only under `NATIVE_DEBUG=1` as `REAL DRIFT: <message>` lines.

Fix: `native::declared_chain_drift_verdict(adiags, locate)` (pub, tested
without a tree) keeps the partition and returns
`"declared-chain drift guard FIRED: N error(s):\n" + render_diag_lines(all real)`,
located through the build's `SourceIndex`. The header is kept on purpose: aeon's
`DRIFT_COUNT_RE` (`tools/emp_expect_fail.py:712`) still matches while their LINK
rows migrate to counting `[Error]`, which they now can (one token per failing
guard). The `NATIVE_DEBUG` `REAL DRIFT:` loop is removed: it was a second
formatter for the same diagnostics. The declared-chain `resolve_layout`/`link`
failures (now `native.rs:3706`/`:3741`, both `first {:?}` before) render every
diagnostic through the same function.

### Tests and runners

`cargo test -p sigil-harness --test link_assert_reporting` (5): a two-file probe
(guarded `.emp` module with three `extern()` guards, plus an AS equ carrier that
supplies the truths) through the real `check_link_asserts` and both real verdict
functions, no aeon tree:

* `emitter_verdict_renders_every_failing_guard_located` (both mirrors drifted:
  header count 2, `probe/guarded.emp:4:1: [Error] ..` and `:5:1: [Error] ..`,
  nothing else on the report)
* `declared_chain_verdict_renders_every_failing_guard_located`
* `agreeing_authority_passes_both_verdicts` (the control)
* `declared_chain_verdict_separates_inapplicable_from_real_drift` (an undefined
  extern is inapplicable and rides back for the allowlist; a real drift beside it
  still fails with one `[Error]`)
* `emit_generated_returns_its_failure_and_creates_nothing` (the channel: `Err`,
  not a panic, and nothing created)

`cargo test -p sigil-harness --lib diag_render` (3): located line shape,
unlocatable fallback, verdict counting.

### Red-first, mutation shown applied and restored from the commit

With `df454b0f` committed, the three behaviours were reverted to their baseline
shapes in the working tree (`git diff --stat`: 2 files, +3/-7):

```
-    Err(format!("{what}: {} error(s):\n{}", errors.len(), render_diag_lines(&errors, locate)))
+    Err(format!("{what}: {errors:?}"))
-        .map_err(|e| format!("ensure_generated writes into the reference tree: {e}"))?;
+        .unwrap_or_else(|e| panic!("ensure_generated writes into the reference tree: {e}"));
-        return Err(format!(
-            "declared-chain drift guard FIRED: {} error(s):\n{}",
-            real.len(),
-            crate::diag_render::render_diag_lines(&real, locate)
-        ));
+        return Err(format!("declared-chain drift guard FIRED: {} error(s); first {:?}", real.len(), real.first()));
```

Run: `test result: FAILED. 1 passed; 4 failed`, cargo exit 101. The failures print
the baseline shapes verbatim (`FIRED: 2 error(s); first Some(Diagnostic {..})`,
`probe guards fired: [Diagnostic {..}, Diagnostic {..}]`, and
`emit_generated must not panic`); the one pass is the agreeing-authority control,
which is insensitive to rendering by design. Restored with
`git checkout -- crates/sigil-harness/src/diag_render.rs crates/sigil-harness/src/native.rs`
(`git diff --stat` empty), re-run: `5 passed; 0 failed`, exit 0.

### Which of sigil's OWN diagnostic paths a red-first `[Error]` token does not reach

Sigil renders diagnostics in three dialects, and only one carries the token:

1. `sigil build` (the native harness): `[Error]` lines, now for the comptime
   `build_program` family, the link-assert family, and the six emitter guard
   families. Still outside the renderer in this path (`Err(String)` with `{:?}`
   or first-only, none of them guard diagnostics): `native.rs:1204/1218/1258/1262/1309/1318/1322`
   (constant/struct harvest parse+resolve), `:1383/1419/1444` (RAM harvest),
   `:1546` (AS residual assemble, `first: {:?}`), `:1872` (manifest scan),
   `:1939` (synthetic entry parse), `:2012` (place_sections), `:2294` (span
   pass), `:3822` (frozen resolve); `seam2.rs` parse/lower/place/resolve/link
   sites (`:440/452/469/473/475/523/534/632/636/645/782/804/814/817/864/894/900/960/963/969/1025/1036/1053/1056/1059/1165/1177/1192/1207/1217/1220`);
   `seam1.rs:210/414/1002/1008` and the panics listed above. The contract closure
   gate prints `error: [proc.out-unverified] ..` lines (`main.rs:1341..1407`),
   the word `error:` and a bracketed family tag, no `[Error]`.
2. `sigil emp <entry> --root <dir>`, `sigil test --root`, and the contract
   report: `path:line:col: error: message` (`render_program_diags`,
   `main.rs:988`). This path already reports link asserts, all of them, located;
   the control run of the same two-file probe through the CLI:
   ```
   ./probe/guarded.emp:3:1: error: MIRROR_A disagrees with the link: the difference is 5
   ./probe/guarded.emp:4:1: error: a second guard that fails
   exit=1
   ```
   No `[Error]` token here either; a token-keyed lane pointed at this path
   counts zero.
3. `sigil emp <file>` (single file): `path:line:col: message`, no level word at
   all (`main.rs:835..840`); `sigil test` failures print `    path:line:col:
   message` (`main.rs:711..715`). The AS front end renders `file(line): error:
   message` (`render_as_diags`, `main.rs:199`).

Sigil's own negative corpus does not key on a token at all: the
`*_negative_probes.rs` files, `lower_guards.rs`, `eval_guards.rs`,
`const_fold_diagnostics.rs`, `diag_assert_vector.rs` assert `Level::Error` and a
message substring at the library level. The single token-keyed gate is
`crates/sigil-cli/tests/extra_entry.rs` (counts `[Error]` on the `sigil build`
stream, reference-dependent), which is exactly the aeon contract. So for sigil
the honest statement is: the `[Error]` token is a property of one dialect of one
subcommand, and nothing in sigil's own suite would notice if a family left it.
Unifying the three dialects (one `path:line:col: [Error] message` shape from
one function in `sigil-span`) is the structural fix; it is an owner call
because aeon's lane and `extra_entry.rs` would need to move with it. Not done
here.

## (g) What enforces the explicit-size invariant R6 rests on

Nothing enforces "every relaxable operand carries an explicit size" for the
`.emp` corpus, and the invariant as R6 words it does not describe sigil. What
sigil enforces and what relaxation may touch are two different sizes:

* **Operand data size is explicit or inherent, never inferred.** An instruction
  with no `.b/.w/.l` suffix is refused (`crates/sigil-frontend-emp/src/lower/code.rs:243..249`
  and `:559..564`, "instruction needs an explicit size suffix") unless the
  mnemonic has an inherent size (`m68k_default_size`, `code.rs:1689..1701`:
  `moveq/lea/pea/swap/nop/rts/rte/tas/trap/illegal/jmp/jsr/btst/bset/bclr/dbcc/scc`);
  `movem` (`:683`) and a link-time immediate (`:774`) are refused separately.
  The size is resolved at `code.rs:243` BEFORE any relaxable fragment is built,
  so no fragment carries an unresolved data size.
* **Relaxation moves a fragment only along its own rung ladder.** `relax.rs`
  states it (`crates/sigil-link/src/relax.rs:9..19`, `:54..60`): `JmpJsrSym`
  and `RelaxAbsSym` have two rungs (`abs.w`, `abs.l`), a `RelaxLadder` has one
  per candidate (`.s`/`.w` branch displacement, Z80 `jr`/`jp`), and the
  fixpoint only grows a rung index. The rungs differ in addressing width and
  displacement width; every rung of one fragment has the same data size, because
  the candidates were built after the size fold. `rung_reaches`
  (`relax.rs:577..625`) is the whole decision surface: `PcRel8`, `PcRelDisp16`,
  `Abs16Be` (via `asl_width_rule`, `crates/sigil-ir/src/width.rs:34`),
  `Abs32Be`, `Z80JrRel8`, `Value16Le`.
* **Which `.emp` sites are width-variable, by construction:** a bare symbolic
  absolute operand (`move.w Foo, d0`, `code.rs:1170..1200` builds the
  `RelaxAbsSym` pair; an authored `(Foo).w`/`(Foo).l` pins it to one candidate,
  `:1194..1199`), bare `jmp/jsr Sym` (`JmpJsrSym`), an unsized `bra/bsr/Bcc` in
  a module without `@as_compat` (`code.rs:345..400` -> `lower_unsized_branch`
  -> `RelaxLadder`), `jbra/jbsr`, and Z80 `jr` (`crates/sigil-backend-z80/src/lib.rs:36..64`).
  Under `@as_compat` an unsized branch is the `[branch.missing-size]` error
  (`code.rs:381..390`): that attribute is the only place sigil enforces R6's
  wording, and it is per-module opt-in for the byte-pinned ports.

So the A1 guarantee for `.emp` is not "zero autonomous sizing decisions"; sigil
makes exactly the width decisions above, on purpose, and the equivalence to
asl's converged choice is proven per site by the byte gates (the twins and the
frozen goldens), plus the documented residual where no equivalence is claimed
(the `0xFF8000` sign-extension wrap, `relax.rs:626..645`, grow-only vs asl's
bidirectional relaxer). R6 should be restated as: relaxation cannot change a
data size (mechanism: size folded before fragment construction); it selects
addressing and displacement width only; AS-equivalence of those selections is a
byte-gate property, not a lint property. A check, if the owner wants one: a
warn-tier `[relax.implicit-width]` at the two `.emp` producers
(`lower_m68k_abs_sym` when `pinned.is_none()`, `lower_unsized_branch`), opt-in
per module the way `@as_compat` is; about thirty lines, and it would fire on
the whole corpus today because bare-symbol absolutes are the house idiom.

## (h) The six retired s4lint invariants (aeon LS-14a)

| s4lint | sigil today | verdict |
|---|---|---|
| E001 unsized branch | `[branch.missing-size]` under `@as_compat` (`lower/code.rs:381..390`); elsewhere an unsized branch is the relaxed `.s`/`.w` ladder by language design (`:345..400`) | enforced where a port asks for it; not a gap |
| E002 mul/div outside a hot path | no rule; sigil's `mul_const`/`mul_bounded` constructs (`lower/code.rs:106..130`, `mul_lower`) make the lowering choice explicit and cost-modelled; a raw `mulu` mnemonic is accepted | game-side convention (aeon's own ruling: the comment is the enforcement); a `[mul.raw]` warn per module would be cheap but sigil should not own the hot-path policy |
| E006 VDP write without the Z80 stopped | `[bus.vdp-write-unstopped]`, the `[bus.*]` inference net (`crates/sigil-frontend-emp/src/z80_bus.rs:1..45`, which names itself "the sigil-native absorption of s4lint's E006/E007/E008/E011"); reported by `--report contracts` (`main.rs:1610..1620`); same caveat as s4lint for register-indirect VDP writes | ALREADY enforced as analysis; NOT in the build gate's zero-firing list (`main.rs:1345..1351`). LS-14a's "no mechanical statement" is wrong for sigil. Cheap: one `empty_gate("[bus.*]", ..)` row, after confirming the corpus is zero-firing with `--report contracts` (needs a tree; not measured here) |
| E007 unpaired stopZ80/startZ80 | `[bus.stopped-at-return]` / `[bus.start-without-stop]` / `[bus.double-stop]`, same net; plus the declared tier `requires/grants(<bus ctx>)` with `[context.*]` firings, which ARE build-gated (`main.rs:1349`) | as E006 |
| W021 writes outside declared Clobbers | 68k: per-proc `[proc.*]` lowering diagnostics (`lower/proc.rs:391`, `:845`) and the closure `[proc.clobber-undeclared]` zero-firing build gate (`main.rs:1347`); Z80: `[call.clobbers-incomplete]` over the linked resident blob (`seam1::z80_clobbers_report`, gate `crates/sigil-cli/tests/z80_clobbers_incomplete.rs`, reference-dependent, strict-gated, not the build gate) | enforced on both CPUs; the Z80 half is a test gate, promoting it to the build gate is a small change if wanted |
| W024 debug-only macro outside `__DEBUG__` | the `.emp` `assert` construct is itself DEBUG-gated at evaluation (`eval/asm.rs:1074..1100`, `debug_gate`: `DEBUG` must be defined, the construct lowers by its value), so a debug-only construct cannot sit outside a gate; any other debug-only helper is the game's `if DEBUG == 1 {}` | game-side convention; sigil owns the construct's gate and nothing more |

## (k) A check-only invocation for one module's guards

What exists today:

* `sigil build --aeon . --game X --extra-entry <module>`: evaluates the named
  module's comptime guards inside the real build (`native.rs:1901..1912`);
  still the full build.
* `sigil emp <entry> --root <dir>`: multi-module lower, `place_sequential`,
  `resolve_layout`, `check_link_asserts` (`main.rs:473..492`). It decides
  LinkAsserts, but against a placement that is not the ROM's and without the AS
  residual or the harvested equ carriers, so every cross-namespace `extern()`
  folds Poison ("not defined in this link"). Honest only for externs a pure
  `.emp` closure defines.
* `sigil test`: per-module `test` blocks (`eval::run_module_tests`), no link.
* `--report ram|contracts`: no link.

The cheapest honest shape: `sigil build --aeon . --game X [--extra-entry M]
--check` that runs exactly what `resolve_frozen_sections` runs
(`native.rs:3798..3830`: `emit_generated`, `assemble_as_side`, `build_emp`,
the declared chain, `resolve_layout`), then `check_link_asserts` +
`declared_chain_drift_verdict` + the inapplicable allowlist, and stops: no
`link`, no `emit_rom`, no listing, no convsym, no fixheader, contract closure
gate optional. That is the same post-relaxation placement the ROM gets, so
every LinkAssert is decided against final addresses, and `--extra-entry` adds
the one module's comptime guards. Two honesty notes: `emit_generated` still
WRITES the sound artifacts into the tree (a check should either accept that or
take a no-write flag), and I could not time it without a tree (not allowed in
this parcel); it removes link + emit + appendix + closure, and keeps lowering +
AS residual + the relaxation fixpoint, so the controller should measure before
anyone quotes a number.

What it could NOT prove: anything `emit_rom` refuses (region containment and
budget, overlap), image bounds, the checksum, and the closure gate if skipped.
It does not lose guard truth: every LinkAssert folds over symbols
(`sigil-link/src/lib.rs:377..382`), and after `resolve_layout` every symbol has
its final VMA; the only placement the check never sees is the convsym appendix
after the ROM end, which no guard reads. The scope caveat: "one module's
guards" cannot be honest for LinkAsserts, whose externs resolve against the
whole program; the saving a check-only buys is the tail of the pipeline, not
its width.

## Totals, branch, SHAs

* harness: 47 test binaries, 422 passed, 0 failed, 1 ignored
  (`SIGIL_ALLOW_PARTIAL=1`; reference-dependent rows unmeasured, expected).
* sigil-cli: 155 test binaries, 697 passed, 0 failed, 1 ignored (same caveat).
* `cargo clippy -p sigil-harness -p sigil-cli --all-targets -- -D warnings`: clean.
* Branch `parcel/link-assert-reporting`; base `52bc7844`; fix `df454b0f`; this
  note is the next commit (SHA in the final report).

## Open

* aeon `tools/emp_expect_fail.py`: the LINK rows can drop `NATIVE_DEBUG=1`
  (`REAL DRIFT:` is gone) and count `[Error]` like the comptime rows; the
  `FIRED: N error(s)` header still matches their regex meanwhile.
* The `{:?}` and first-only sites listed under (j) item 1, none of them guards.
* Dialect unification across the three CLI paths (owner call).
* `[bus.*]` as a build-gate row (needs a zero-firing measurement first).
* The `--check` shape under (k).
