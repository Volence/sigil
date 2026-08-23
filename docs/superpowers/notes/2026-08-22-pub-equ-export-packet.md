# `pub equ` export + the clippy `collapsible_match` block — 2026-08-22

Branch `fix/pub-equ-export`, two independent parcels, one commit each.

| commit | parcel |
|---|---|
| `d5308f7b` | `pub equ` exports, and an import binds its bare link name |
| `fac1503a` | fold the byte-string arity condition into its match guard |

---

## Parcel 1 — `pub equ` does not export

### The defect, re-derived

`crates/sigil-frontend-emp/src/parser.rs:546` accepts `pub` on an `equ` and threads it
into `ast::EquDecl::is_pub`, whose doc comment reads *"Whether this equ is exported
(`pub equ`)"*. `crates/sigil-frontend-emp/src/resolve/imports.rs` `item_pub_name()` had
arms for Data, Proc, Offsets, Dispatch, Script, Const, ComptimeFn, Context, Struct, Enum,
Bitfield, Newtype and Vars — and no `Item::Equ` arm, so a `pub equ` fell to `_ => None`
and never reached the export index.

Spec text, read at empyrean `origin/main` (`docs/SIGIL_SPEC2_LANGUAGE.md:749`, last
touched by `0955a91`), not through a working tree:

> `pub equ` adds module visibility like every other `pub` item; the link symbol exists
> regardless (that is the construct's purpose).

Compiler bug, not a spec error. No evidence the omission was deliberate: the sibling
machinery (`pub_equ_names`, `nonpub_equ_names`, the `[equ.collision]` uniqueness check)
is all built and all consistent with `pub equ` being visible.

### The one-line fix is NOT sufficient — the finding that changed the shape of the parcel

Every other exported item is renamed to its module-qualified `canonical` symbol
(`module.id.Name`) so two modules' same-named items cannot collide in the flat link
table. A `pub equ` is the exception: its definition keeps the **bare** spelling, because
a still-`.asm` consumer names it unqualified across the link seam. `ResolveEnv::build`
already encodes that — a module's own `pub equ` maps to **itself**.

So adding only the `item_pub_name` arm makes `use m.a.{WIDTH}` bind `WIDTH` to
`m.a.WIDTH`, a symbol nothing defines. Measured, by applying exactly that one-line change
and running the new CLI gate:

```
error: unresolved symbol `m.a.WIDTH` for fixup in section text at offset 2
```

`ExportIndex` therefore also records which exports are equs, and the new
`ExportIndex::import_target()` binds those to the bare name. It is applied on all three
binding paths — the `use{…}` list arm, the glob arm, and the prelude overlay in
`ResolveEnv::build` — because all three previously called `canonical()` directly.

This is sound only because a `pub equ` name is unique across the whole program, which
`build_program` already enforces with the `[equ.collision]` diagnostic
(`resolve/mod.rs:713`), whose own text says *"a `pub equ` is a plain cross-seam link
symbol, so its name must be unique across the program"*.

### `collect_exported` — the wrinkle question

**No matching change needed.** The `Vars` region form is special-cased there because one
item exports MANY names (its field/mark/alias labels). An `equ` exports exactly one name,
so `item_pub_name` carries it. `collect_exported` already recurses into `section {}`
bodies, matching `collect_pub_equs`'s recursion, so a section-nested `pub equ` exports
consistently with how the rename map already treats it.

`collect_defined` also needs no `Equ` arm and must not get one: equs enter the rename map
through the dedicated `pub_equ_names` / `nonpub_equ_names` loops, which map them to
themselves / to `$module$NAME`. An `Equ` arm in `collect_defined` would map a module's own
pub equ to `module.NAME` and reintroduce exactly the dangling-symbol failure above.

### Red-first evidence

Four gates, all in **existing** test files (no new `crates/*/tests/*.rs` file).

`crates/sigil-cli/tests/module_resolution.rs`:

- `pub_equ_is_importable_and_keeps_its_bare_link_name` — end-to-end through the `sigil emp`
  binary; asserts the emitted bytes are `30 3C 00 20 4E 75` (`move.w #$0020,d0` + `rts`),
  so the reference reached the definition's *value*, not merely a symbol that resolved.
- `private_equ_is_not_importable` — control arm.

`crates/sigil-frontend-emp/tests/resolve_imports.rs`:

- `pub_equ_import_binds_the_bare_link_name` — unit; asserts `env.resolve("WIDTH") ==
  Some("WIDTH")` and explicitly `!= canonical("mod.a", "WIDTH")`.
- `private_equ_is_not_exported` — control arm.

**Red before the fix**, source reverted with `git checkout HEAD -- …imports.rs`, tests in
place:

```
---- pub_equ_import_binds_the_bare_link_name stdout ----
importing a `pub equ` must not error, got [Diagnostic { level: Error,
  message: "module `mod.a` has no `pub` name `WIDTH`", primary: Span { … } }]
test result: FAILED. 11 passed; 1 failed; 0 ignored
```

```
---- pub_equ_is_importable_and_keeps_its_bare_link_name stdout ----
importing a `pub equ` must compile, stderr:
  /tmp/.tmp8XGzZs/m/b.emp:2:1: error: module `m.a` has no `pub` name `WIDTH`
  /tmp/.tmp8XGzZs/m/b.emp:1:1: error: unknown symbol `WIDTH`
test result: FAILED. 1 passed; 1 failed; 0 ignored; 42 filtered out
```

Both control arms pass before AND after — that is their job. They are proven load-bearing
by poison instead.

### Poison runs

**Poison A — over-export (drop the `if e.is_pub` guard, export every `equ`).** The
positive arms both stay green; only the control arms catch it:

```
---- private_equ_is_not_exported stdout ----
assertion failed: !idx.is_exported("mod.a", "WIDTH")

---- private_equ_is_not_importable stdout ----
stderr was: error: unresolved symbol `m.a.WIDTH` for fixup in section text at offset 2
```

**Poison B — the brief's one-line fix (`import_target` returns `canonical` always).** The
control arms stay green; only the positive CLI arm catches it:

```
---- pub_equ_is_importable_and_keeps_its_bare_link_name stdout ----
importing a `pub equ` must compile, stderr:
  error: unresolved symbol `m.a.WIDTH` for fixup in section text at offset 2
```

Each half of the fix is charged by a different arm, and neither poison is green.

### Matcher uniqueness

`private_equ_is_not_importable` matches on ``module `m.a` has no `pub` name `WIDTH` ``;
`private_equ_is_not_exported` compares the whole message string. Exactly one production
site can emit that phrase:

```
$ grep -rn "has no \`pub\` name" --include='*.rs' --include='*.md' --include='*.emp' .
crates/sigil-frontend-emp/src/resolve/imports.rs:355   ← the only source site
docs/superpowers/notes/2026-07-08-item9b-implementation-notes.md:171   ← prose
docs/superpowers/plans/2026-07-07-spec2-plan7-item4-module-resolution.md:461   ← plan text
```

Confirmed by Poison A: when the guard is dropped, the private-equ CLI arm's stderr changes
to `unresolved symbol …` and the matcher goes red rather than absorbing a different
diagnostic.

### Live-corpus check

`pub equ` declaration counts in aeon, anchored at line start (`grep -cE '^[[:space:]]*pub
equ'`) so prose about the construct is not counted as a declaration:

| file | declarations |
|---|---|
| `engine/debug/error_handler.emp` | 45 |
| `engine/system/boot_data.emp` | 2 |
| `games/sonic4/data/effects/scene_registry.emp` | 21 |
| `games/sonic4/data/generated/ojz/act1/effects_scenes.emp` | 2 |
| `games/sonic4/data/generated/ojz/act1/sec_block_blobs.emp` | 1 |
| `engine/level/scene_dsl.emp` | 0 (its one hit is a comment) |

71 declarations across 5 files. An unanchored `grep -c` returns 46/2/22/3/1 and a phantom
hit in `scene_dsl.emp`, because it counts the comments that talk about the construct.

**Zero of the 71 are consumed by an `.emp` `use{…}` list** — established in the aeon tree
by the aeon lane (it is not answerable from sigil), so nothing shipped goes through the
path this parcel repairs, and the change cannot regress a shipping consumer.

**But an `.emp`→`.emp` consumer of a `pub equ` DOES ship, by a different route.**
`engine/system/boot.emp:142,144` reference `Z80_SOUND_SIZE` and `Z80_IDLE_SIZE` from
`engine/system/boot_data.emp` as **bare names with no `use` at all**. That works because
aeon builds through `build_program_open_embed` (`sigil-harness/src/native.rs:1548,2083`),
whose `closed = false` skips `report_unresolved`, so an unresolved bare name passes through
the rename map untouched and meets the equ's plain link symbol at link time. Verified by
construction: the same program under the closed `sigil emp` path fails
`unknown symbol \`WIDTH\``.

That shipped route is precisely what makes the bare-name binding load-bearing, and it is
preserved: a `pub equ`'s definition and its bare references are untouched by this change.

---

## Parcel 2 — clippy `collapsible_match`

### Reproduced first, on the pre-fix tree

```
error: this `if` can be collapsed into the outer `match`
   --> crates/sigil-frontend-emp/src/eval/const_arity.rs:153:25
    = note: `-D clippy::collapsible-match` implied by `-D warnings`
error: could not compile `sigil-frontend-emp` (lib) due to 1 previous error
error: could not compile `sigil-frontend-emp` (lib test) due to 1 previous error
```

One error, workspace-wide, pre-existing.

### The equivalence argument

The arm's entire body was a single `if`, so its condition moves into the arm guard:

```rust
Value::Str(s)
    if matches!(**elem, Ty::Prim { width: 1, .. })
        && s.is_ascii()
        && s.len() != *n => { self.error(…); }
```

The `match value` has exactly three arms: `Value::Array(elems)`, this `Value::Str(s) if …`,
and `_ => {}`. A `Value::Str` cannot match the `Value::Array` pattern, so when the extended
guard fails the only arm left to take is `_ => {}` — the same no-op as falling off the end
of the old `if`. Nothing follows the match inside the `Ty::Array` arm. The rewrite is
therefore semantics-preserving in every case, and no `#[allow]` was needed.

No diagnostic text moved: the `format!` string
`array length mismatch: expected {n} element(s), got {}` is byte-identical, which matters
because `crates/sigil-cli/tests/const_arity_cli.rs` pins it by exact text and aeon fixtures
assert on the same wording.

Targeted re-runs of the code this touches:

```
crates/sigil-cli/tests/const_arity_cli.rs          2 passed; 0 failed; 0 ignored
crates/sigil-frontend-emp/tests/const_array_arity  10 passed; 0 failed; 0 ignored
```

including `const_non_ascii_string_reports_no_arity` and
`const_string_byte_array_length_is_checked`, the two arms whose behaviour a bad collapse
would have moved.

Final: `cargo clippy --workspace --all-targets -- -D warnings` finishes clean.

---

## Suite

```
### pwd=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a89234eeced9ab114
### head=fac1503afc5d75304644e31ccf45f59a065e1137
### branch=fix/pub-equ-export
### dirty=0
### date=2026-08-22T21:04:10-04:00
### cargo_exit=0
```

Command: `AEON_DIR=/home/volence/sonic_hacks/.aeon-landing SIGIL_STRICT_GATE=1 cargo test
--release --workspace --no-fail-fast`. The stamp is written before cargo appends, so a run
from the wrong worktree would be visible in the artifact.

| metric | value |
|---|---|
| passed | 3839 |
| failed | 0 |
| ignored | 4 |
| `FAILED` occurrences in log | 0 |
| `skip:` lines in log | 0 |
| `^error:` lines in log | 0 |
| `test result:` lines summed | 336 binaries |

**Reconciliation against the tree, not against the remembered bar:**
`git grep -c '#\[test\]' HEAD -- '*.rs'` sums to **3843** = 3839 passed + 4 ignored. ✔

Bar was 3835/0/4; this branch adds exactly the 4 new gates → 3839/0/4.

**Each new gate is present in the stamped log** (`grep -c`, all ≥ 1):

```
pub_equ_is_importable_and_keeps_its_bare_link_name = 1
private_equ_is_not_importable                      = 1
pub_equ_import_binds_the_bare_link_name            = 1
private_equ_is_not_exported                        = 1
```

**Source-gate self-audit** (`scripts/nightly_source_gates.sh`'s classifier block, replayed
against this worktree): `gates=116 unclassified=0`. No new `crates/*/tests/*.rs` file was
added, and neither touched test file matches the lane's
`AEON_DIR|aeon_dir|reference_tree|--aeon` selector, so the backstop's classification is
untouched.

---

## Not done, and why

- **No emulator.** Nothing here needs runtime confirmation: both parcels are
  compile/link-time, and the CLI gate asserts emitted bytes.
- **`cargo fmt --check` is not a usable gate on this repo** — it reports diffs across many
  untouched files (`sigil-cli/build.rs`, `src/main.rs`, `src/bin/emp_census.rs`, …) on
  master. New code follows the surrounding style by hand.
- A qualified reference to an imported `pub equ` (`m.a.WIDTH`) still does not resolve — see
  the gap ledger.
