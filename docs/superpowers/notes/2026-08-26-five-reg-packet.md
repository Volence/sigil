# FIVE-REG packet — the one red test and the pinned-bootstrap map (2026-08-26)

Branch `fix/five-reg` (sigil), from master `a4eac185`. Aeon witness: the landing checkout
`.aeon-landing` at `058ad606` (clean; `s4.bin` 875d591f/699223, `s4.debug.bin`
a02d36db/715114, verified before every run). No ROM byte moves; nothing under `golden/`,
`pins.rs`, `repin.toml` or `provenance.toml` changes. No `.emp` touched.

## 1. The failure and the path that minted it

`soundbankhead_pinned_bootstrap_lands_at_lma_not_vma` (`crates/sigil-cli/tests/
soundbankhead_port.rs`) failed on master with `section \`ojz_effects_editor_act1\` has no
region in the map` (`sigil-frontend-emp/src/resolve/mod.rs:849`, `place_sections`).
Reproduced at `a4eac185` (`target/repro-master.log`: 2 passed / 1 failed, exit 101).

The map that lacks the region is minted by the **PinnedBaked** path, exactly as the
derived-layout design note §3.5 predicted:

```
soundbankhead_port.rs  →  native::resolve_pinned_sections(aeon, debug)
  → assemble_native_all_gates_as_side   (= assemble_as_side(sonic4_pinned_profile))
  → build_native_emp                    (= build_emp(sonic4_pinned_profile))
      → build_emp: match profile.size_source
            SizeSource::PinnedBaked => emp_map_toml(specs, debug)   ← one region per REGISTRY ModuleSpec
            SizeSource::Frozen(_)   => emp_map_frozen(&sections)    ← one region per PRESENT section
      → place_sections(&mut sections, &map)  → "no region in the map"
```

`emp_map_toml` iterates `profile.registry` (`native::registry`), and every row carries a
`pins::Region`. `ojz_effects_editor_act1` is minted by aeon's `tools/effects_gen.py`
(`games/sonic4/data/generated/ojz/act1/effects_scenes.emp`), reached through the
`act_descriptor` registry module, emits bytes since aeon `0e34408d`, and is declared to
the map by `"section:ojz_effects_editor_act1"` (`map.toml`, SECTION-ROW). It has no
`pins::Region` and none is minted for it: `repin` derives `pins.rs` from the *shipped*
resolve, and a content-derived section is pin-less by that design. So the registry can
never learn it from its own source of truth — the bootstrap's premise ("every `.emp`
section has a committed pin") is what aeon's derived section retires.

The `sigil build` path is untouched by this: `sonic4_profile` is always
`SizeSource::Frozen` (`load_frozen_table`), `emp_map_frozen` mints a region per present
section name, and `build_native_rom_with_listing` returns into the chained driver on its
first line.

## 2. Option chosen: (c) — the PinnedBaked probe is obsolete; the catch is re-homed

Every reference to the pinned path, enumerated (`grep` over `*.rs *.py *.sh *.toml *.md`
outside `target/` and the notes/log):

| consumer | status |
|---|---|
| `soundbankhead_port.rs` `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma` | the red test — re-homed here |
| `native_chained_resume.rs` `chained_resume_{plain,debug}` (`assemble_native_all_gates_as_side` + `build_native_emp`) | `#[ignore]` since Wave-B B-0 ("RETIRED … kept for archaeology") |
| `derive_offcanon --bootstrap-canonical` → `derive_canonical_bootstrap_table` | one-shot mint of `s4.txt`/`s4_debug.txt`, done; documented **unavailable since 2026-08-01** (conv-g §7.3: the `objdefs`/org-$11D7E strict overlap); now additionally stops on the pin-less section |
| `build_native_rom_with_listing` pinned body | unreachable: the `Frozen` early-return always fires for `sonic4_profile` |
| `sigil_native_symbol_listing`, `phase_bank_lmas`, `project_memory_map` | docstring says "pinned"; the code calls `resolve_canonical_sections` (Frozen) |
| `ModuleSpec::base` / `::len` (the registry pins) | read ONLY by `emp_map_toml` |

So no live consumer of `SizeSource::PinnedBaked` exists; the registry pins serve no
shipped placement. What still *uses the profile*: the `--bootstrap-canonical` CLI flag
(broken twice over) and the two archaeology tests. Ledger row added with the retire
parcel's full kill list.

Options (a)/(b) rejected on the facts, not on taste: (a) "learn sections from the same
source as Frozen" cannot give a pin-less section a base — the only base source the
pinned path has IS a pin; (b) "add the registry row" needs a `pins::Region`, which only a
`repin` regeneration mints (`pins.rs` change → STOP per the brief) and which would then be
a hand-maintained pin for a section whose whole point is that it is derived.

**The test is not deleted.** What it was FOR — ledger 1966, the pin holds the bank's LMA
and not the `$8000` phase VMA its labels resolve at — is what the two sibling byte gates
window the reference ROM by, and it now has a probe on the layout that ships:

`soundbankhead_pin_is_the_lma_not_the_vma`: for both shapes,
`resolve_canonical_sections` (the resolve `repin` derives the pin from) must carry
`soundbankhead` with a `vma_base` present and `!= lma`, and
`pins::SOUNDBANKHEAD.{plain,debug}_base == sec.lma`. No address is written into the test
— the old `0x58000` / `Some(0x8000)` literals are gone; the expectation is the tree's.
Loud on unmeasurable: `reference_tree_for_profile` panics under `SIGIL_STRICT_GATE=1`
when the profile's inputs are absent; a missing section or `vma_base` is an `expect`.

## 3. Red-first evidence (invariant 8)

| step | log | outcome |
|---|---|---|
| re-homed probe, tree as-is | `target/green-rehomed.log` | 3 passed |
| sabotage: `pins.rs:286 plain_base: 0x58000 → 0x8000` (uncommitted) | `target/red-sabotage.log` | `soundbankhead_pin_is_the_lma_not_the_vma` FAILED: "pins::SOUNDBANKHEAD base (debug=false) must be the bank's LMA 0x58000 — where the shipped layout loads it — not its phase VMA 0x8000"; `soundbankhead_matches_reference` FAILED on its own byte assertion (it windows by the same pin) |
| `git checkout pins.rs`, re-run | `target/green-restored.log` | 3 passed; `git status` clean |

## 4. The sibling flake (`PoisonError`)

`static LOCK` was taken with `.lock().unwrap()`; when the red test panicked holding it,
whichever sibling drew the lock next failed on `PoisonError`. Measured on master state,
8 repeats (`target/poison-master.log`): 3 of 8 runs poisoned a sibling (runs 5–7; runs 6
and 7 took BOTH siblings down — 0 passed / 3 failed). Fix (commit `4e14db55`): a file
`lock()` helper, `LOCK.lock().unwrap_or_else(|p| p.into_inner())` (the `native_full_rom.rs`
idiom), used by `gate` and the probe. 8 repeats with the red test still present
(`target/poison-lockfix.log`): 8 of 8 show exactly the one known red, 0 `PoisonError`.
Under the §3 sabotage the same isolation shows: the sibling fails on its OWN assertion.
Ledger row (SECTION-ROW's, assigned to fix/rig-closure) closed here.

## 5. Suite

`SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-landing cargo test --release
--workspace --no-fail-fast`, `target/suite.log` (stamped `pwd=<worktree> HEAD=774de432
branch=fix/five-reg dirty=0 aeon=058ad606`), wall-clock 23:21:20 → 23:23:54 (2m34s),
`suite_exit=0`. Aggregated over all 341 `test result:` lines:

| | passed | failed | ignored | declared `#[test]` |
|---|---|---|---|---|
| master bar (`a4eac185`) | 3869 | 1 | 4 | 3874 |
| this branch (`774de432`) | **3870** | **0** | 4 | 3874 (`git grep -c '#\[test\]' HEAD -- '*.rs'` summed) |

3870 + 4 = 3874: every declared test ran or was ignored; zero skips (the 8 "skip"
matches in the log are passing test NAMES). The 4 ignored are unchanged
(`chained_resume_{plain,debug}`, `secondary_pin_classes_match_the_hand_typed_baseline`,
`sigil_diff_reports_byte_identity`).

Status changes (2): `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma` FAILED →
removed; `soundbankhead_pin_is_the_lma_not_the_vma` new → ok. Nothing else moved.

`cargo clippy --workspace --all-targets -- -D warnings`: `target/clippy.log`,
`clippy_exit=0`, 0 warnings. No commits between the suite build and run; the packet +
ledger commit on top of `774de432` is docs-only.

## 6. Byte identity

`git diff master --stat` touches only `crates/sigil-cli/tests/soundbankhead_port.rs`,
`docs/superpowers/notes/campaign-gap-ledger.md` and this packet. `golden/`, `pins.rs`,
`repin.toml`, `provenance.toml`: unchanged (`git status` clean after the sabotage
restore). Aeon not modified or built; `.aeon-landing` CRCs re-verified at the start.

## 7. Deferred / open

- The PinnedBaked retire parcel (ledger row, kill list there). Until it lands the
  `--bootstrap-canonical` flag stops by name on the first pin-less section — loud, not
  silent — and `resolve_pinned_sections` has no caller.
- Three docstrings in `native.rs` still say "pinned" where the code resolves the shipped
  layout (`sigil_native_symbol_listing`, `phase_bank_lmas`, `project_memory_map` neighbours);
  cosmetic, left for the retire parcel so the history reads in one diff.
