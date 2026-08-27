# `embed()` base resolution on the ROM build path — EMBED-BASE-RULE

Queue item `EMBED-BASE-RULE`. Read-only investigation; no behaviour changed.

**Question.** Which base does a relative `embed("...")` path join onto on the path that
actually builds the ROM, and what makes `engine/system/math.emp`'s `embed("../data/sine.bin")`
resolve when a single lexical join against the aeon root would be refused?

**Short answer.** A per-module base *does* fire on the ROM path. It is supplied by
`sigil_harness::native::build_emp`, which hardcodes one module id:

```rust
let embed_base_for = move |id: &str| -> Option<PathBuf> {
    if id == "engine.math" { Some(math_dir.clone()) } else { Some(aeon_root.clone()) }
};
```

`math_dir` is `<aeon>/engine/system`. That single special case is the whole mechanism.

---

## 1. The traced call graph

Every hop below is by symbol name.

| # | Site | What it fixes |
|---|---|---|
| 1 | `aeon/build.sh` | `"${SIGIL_BUILD}" build --aeon . --native ${NATIVE_FLAGS} -o … --emit-lst …` |
| 2 | `sigil-cli/src/main.rs::parse_build_args` | `--native` is an accepted **no-op** ("native is the only build post-flip"); default target = `BuildTarget::Sonic4 { debug: false }` |
| 3 | `sigil-cli/src/main.rs::run_build_native` | canonical sonic4 with no `--extra-entry` → `native::build_native_rom_with_listing(aeon, debug)` |
| 4 | `sigil-harness/src/native.rs::build_native_rom_with_listing` | `sonic4_profile(debug).size_source` is `SizeSource::Frozen(…)` (see `sonic4_profile`), so it returns `build_rom_chained_with_listing(aeon, &sonic4_profile(debug))` immediately |
| 5 | `sigil-harness/src/native.rs::build_rom_chained_with_listing` | calls `build_emp(aeon, profile)` |
| 6 | `sigil-harness/src/native.rs::build_emp` | **`include_root: Some(aeon)`, `embed_base: Some(aeon)`, and the per-module `embed_base_for` closure above** |
| 7 | `sigil-frontend-emp/src/resolve/mod.rs::build_program_open_embed` → `build_program_with` | per module: `let module_embed_base = embed_base_for(&pm.id);` then `LowerOptions { embed_base: module_embed_base, ..opts.clone() }` |
| 8 | `sigil-frontend-emp/src/lower/mod.rs::lower_module` | packs `include_root` + `embed_base` into the `Placement` handed to each item lowerer |
| 9 | `sigil-frontend-emp/src/lower/mod.rs` `data`-item lowering | `eval_data_with_root_and_base(file, name, Some(here), placement.include_root, placement.embed_base, placement.defines)` |
| 10 | `sigil-frontend-emp/src/layout.rs::eval_data_with_root_and_base` | `ev.set_include_root(root)` **and** `ev.set_embed_base(base)` |
| 11 | `sigil-frontend-emp/src/eval/sandbox.rs::eval_embed` → `resolve_sandbox_path` | join onto `embed_base`, containment against `include_root` |

`build_native_emp` (the other public entry) also delegates to `build_emp`, so both ROM
drivers route through hop 6.

### Measured values at the call

Temporary instrumentation was added at the containment check in `resolve_sandbox_path`
(printing `path`, `include_root`, `embed_base`, `resolved`, `contained` under an env
guard), the CLI rebuilt, and a full ROM build run against the frozen aeon tree
`/home/volence/sonic_hacks/.aeon-freeze-slope` (detached at `9bba8700`). **The
instrumentation was removed before committing** — the committed tree has no probe code
(`git status` clean apart from the private `CARGO_TARGET_DIR`).

The two `math.emp` lines, verbatim:

```
path="../data/sine.bin"   include_root="…/.aeon-freeze-slope"
  embed_base=Some("…/.aeon-freeze-slope/engine/system")
  resolved="…/.aeon-freeze-slope/engine/data/sine.bin"   contained=true
path="../data/arctan.bin" include_root="…/.aeon-freeze-slope"
  embed_base=Some("…/.aeon-freeze-slope/engine/system")
  resolved="…/.aeon-freeze-slope/engine/data/arctan.bin" contained=true
```

Each fires **exactly once** in the whole build (454 `resolve_sandbox_path` calls total).

The join is legal because containment is checked against `include_root` (the aeon root),
not against `embed_base`: `<aeon>/engine/system` + `../data/sine.bin` = `<aeon>/engine/data/sine.bin`,
which `starts_with(<aeon>)`.

---

## 2. Evidence

All commands run from the investigation worktree with a private on-disk
`CARGO_TARGET_DIR` (never `/tmp`, never the shared `target/`).

**(a) The build reproduces the canonical ROM**, so the traced path is the ROM path:

```
$ ./target-embedinv/release/sigil build --aeon /home/volence/sonic_hacks/.aeon-freeze-slope -o <scratch>/probe.bin
built: sonic4 plain native ROM — crc=34c67ea6 len=718999
```

`/home/volence/sonic_hacks/.aeon-freeze-slope/s4.bin` is `crc=34c67ea6 len=718999` —
byte-identical.

**(b) The embedded blobs reach the ROM.** `engine/data/sine.bin` (640 = `$280` bytes)
appears at ROM offset `0x2868`; `engine/data/arctan.bin` (258 = `$102` bytes) at `0x2B44`.
`MATH`'s `plain_base` is `0x2850`. Not dead code.

**(c) Counterfactual — the closure is load-bearing, not merely present.** Temporarily
gating the `engine.math` arm off (so the closure returns the aeon root for every module)
and rebuilding:

```
EXIT=1
  [Error] [sandbox.path-escape] embed/import path must stay within the source directory @ SourceId(63) 1760..1778
  [Error] [sandbox.path-escape] embed/import path must stay within the source directory @ SourceId(63) 7140..7160
```

Two errors, one per `math.emp` embed — exactly the refusal the static reading predicted.
This gate was also removed before committing.

**(d) The base is the CONSUMING evaluation's module base, not the defining module's.**
On a throwaway copy of the frozen tree (`cp -a`, `.git` removed, deleted afterwards):
adding `pub const _SINE_PROBE = embed("../data/sine.bin")` to `engine/system/math.emp`,
a `use engine.math.{_SINE_PROBE}` edge in `games/sonic4/player/player_ground.emp`, and an
`ensure(_SINE_PROBE.len == $280, …)` there to force evaluation:

```
EXIT=1
error: native build (sonic4 plain): build_program: 1 error(s);
  [Error] [sandbox.path-escape] embed/import path must stay within the source directory @ SourceId(63) 1750..1768
```

The identical expression is legal inside `engine.math`'s own `data` item and refused when
evaluated from `games.sonic4.player_ground`. Control runs on the same copy (unpatched,
and with the embed routed through a module-local `const _sine_blob = embed(…)` consumed by
the `data` item) both built `crc=34c67ea6` — so the failure is caused by the consumer's
base, not by the copy or by the `const` indirection.

---

## 3. Answers

### Q1 — Does any caller on the ROM build path supply a non-constant `embed_base_for`?

**Yes, exactly one:** `sigil_harness::native::build_emp`. It is the only
`embed_base_for` in the tree that reads its `id` argument.

The complete census of `build_program*` call sites in non-test `src/`:

| Caller | `embed_base_for` | On ROM path? |
|---|---|---|
| `native::build_emp` | `if id == "engine.math" { <aeon>/engine/system } else { <aeon> }` | **yes** |
| `native::` RAM-harvest builder (the `__ram_harvest_entry__` path) | `move \|_id\| Some(aeon_root.clone())` — constant | no (RAM harvest only) |
| `resolve::build_program` / `build_program_open` | `&\|_\| opts.embed_base.clone()` — constant | no |
| `sigil-cli/src/main.rs::run_emp_program` (`sigil emp <entry> --root <dir>`) | `build_program`, `embed_base: None` | no |
| `sigil-cli/src/main.rs::run_ram_report` (`--report ram`) | `embed_base: None` | no |

### Q2 — What actually makes that embed resolve?

The hardcoded `engine.math` arm of `build_emp`'s closure (hop 6), which sets
`embed_base` to `<aeon>/engine/system` for that one module while leaving `include_root`
at the aeon root. Nothing else in the mechanism is unusual: the join is the single
lexical join, the containment check is the single `starts_with(include_root)`, and there
is no fallback.

None of the alternative candidates hold. `include_root` is exactly `<aeon>` at that call
(measured). There is no second lowering entry point for `math.emp`. It reaches the ROM by
the traced route and no other. The containment check passes for precisely the reason the
two-field split was designed for.

### Q3 — Is `embed()` root-relative, module-relative, or per-module-configurable?

**Per-module-configurable by the embedder (the Rust caller), not by the `.emp` source —
and only for `data` items.** Proposed spec wording:

> An `embed`/`import` path is relative and must not be absolute. It is joined
> **lexically** onto a *join base* chosen by the build driver, and the join result must
> lie within `include_root`, which is the containment boundary and is never the thing a
> `..` is measured against. `..` may climb above the join base as long as the result
> stays inside `include_root`.
>
> The join base is supplied per module by the driver. On the aeon ROM build
> (`sigil build --aeon <dir>`), `include_root` is `<dir>` and the join base is `<dir>`
> for every module **except `engine.math`**, whose base is `<dir>/engine/system`. So
> aeon `embed` paths are **repo-root-relative by default**, with `engine.math` the one
> module-relative exception, named by module id in `sigil_harness::native::build_emp`.
>
> The base applies to `data` items only. Every other comptime evaluation site — `const`,
> `equ`, `offsets`, `table`, `dispatch`, item guards — resolves against `include_root`
> regardless of the module's configured base. Consequently the base that applies to a
> `const … = embed(p)` is the base of whatever evaluation **forces** it, not of the
> module that **declares** it: forced from inside its own module's `data` item it sees
> that module's base; folded ambiently or forced from an importing module it sees
> `include_root`.
>
> Adding a module-relative `embed` to any module other than `engine.math`, or moving
> `engine.math`'s to a non-`data` item, is a `[sandbox.path-escape]` error.
>
> Outside the aeon driver — `sigil emp <entry> --root <dir>` and the `--report` paths —
> no join base is set, so `embed` is strictly root-relative there.

---

## 4. Findings the question did not ask for

1. **The per-module map is a hardcoded module-id string, not derived from the module's
   own file path.** Nothing generalizes it. A second module-relative `embed` anywhere in
   aeon is refused, with a diagnostic that names the *path escape*, not the missing base
   registration. (Reported, not fixed.)

2. **Only `data` items thread `embed_base`.** By signature: the `data` lowerer calls
   `eval_data_with_root_and_base(…, placement.include_root, placement.embed_base, …)`,
   while `lower_equ_item` → `eval_const_with_root_and_contracts`, `eval_offsets_with_root`,
   `layout::eval_table`, `eval_dispatch_with_root`, and `eval::guards::eval_item_guard`
   all take `include_root` only. Confirmed at runtime: every `const … = embed(…)` site in
   the trace shows `embed_base=None`.

3. **`resolve::fold_const_literal` discards its diagnostics** ("a real error surfaces at
   the const's own decl site during lowering") and returns `None` on a non-integer value.
   A `Data`-valued const whose path escapes therefore fails *silently* during ambient
   folding; it only becomes an error at a consumer that forces it (evidence (d)). A `pub
   const` module-relative embed that nothing forces builds green.

4. **A single `sigil build --aeon` uses three different `include_root`s.** 152 calls at
   `<aeon>` (the `.emp` program, hop 6), 212 at `<aeon>/games/sonic4/data/sound`, and 90
   at `<aeon>/games/sonic4/data/sound/sfx` — the last two from the pre-build artifact
   emitters (`native::ensure_generated` → `seam1::emit_sound_blob`,
   `seam2::emit_*_artifacts`), which call `lower_module` directly with `embed_base: None`
   and a narrow module-directory `include_root`. Those are a separate sandbox regime from
   the program build and are not covered by any per-module base.

5. **The isolated port oracle uses a different `include_root` than the real build.**
   `crates/sigil-cli/tests/tranche2_negative_probes.rs` pins `include_root = <aeon>/engine`
   with `embed_base = <aeon>/engine/system`; the real build uses `include_root = <aeon>`.
   Both make the join legal, so the oracle is not wrong — but it is not a witness for the
   shipping boundary, and a regression that widened or narrowed `include_root` on the ROM
   path would not show up there.

---

## 5. What I could not establish / did not cover

- **Targets other than canonical sonic4 plain were not built.** demo, `--debug`,
  `--config-a`, `--config-b`, `--lean`, and the stress shapes all reach `build_emp`
  through `build_rom_chained_with_listing` (read from the code), so the closure covers
  them by construction — but that is a code reading, not a measured run.
- **`import()` was not separately exercised.** It shares `resolve_sandbox_path`, and the
  instrumentation sat in that shared function, but every one of the 454 traced calls in
  this build came from `embed`. The rule above is asserted for `embed`; `import` shares
  the resolver and should behave identically, unverified.
- **The harness/golden suites were not run.** This was a read-only investigation; no
  behaviour changed, so no gate was owed, and none is claimed green.
- **No emulator was used** (standing invariant). Nothing here needs runtime confirmation:
  every claim is a build-time diagnostic or a ROM-byte comparison.
- The claim in finding 3 that an *unforced* `pub const` module-relative embed builds green
  was observed (the probe built clean before the `ensure` was added), but I did not
  instrument the swallow itself to prove the diagnostic was raised and discarded rather
  than never raised.

## 6. Reproducing

```sh
# from an investigation worktree of sigil, with a private on-disk target dir
export CARGO_TARGET_DIR=<worktree>/target-embedinv
cargo build --release -p sigil-cli
./target-embedinv/release/sigil build --aeon /home/volence/sonic_hacks/.aeon-freeze-slope -o /tmp/probe.bin
# -> built: sonic4 plain native ROM — crc=34c67ea6 len=718999
```

For the measured `embed_base` values, add a temporary `eprintln!` immediately before the
`resolved.starts_with(&root)` check in `Evaluator::resolve_sandbox_path` and rerun.
For the counterfactual, gate off the `id == "engine.math"` arm in `native::build_emp`.
Remove both before committing.
