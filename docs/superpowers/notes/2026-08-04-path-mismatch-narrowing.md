# 2026-08-04 — `[module.path-mismatch]`: the rule narrows to the file stem

Small follow-up to the warning-tier parcel
(`notes/2026-08-04-warning-tier.md`), which measured the corpus for the first
time and found this lint was **84% of the whole default tally line** — 93 of 111
firings on `sonic4` plain — with every single firing being a convention the
codebase deliberately adopted.

**Ruled by Volence, 2026-08-04: narrow the rule.**

## Why the old rule could not stay

`expected_id_from_path` transcribed a file's whole directory chain into a dotted
id and demanded the module header reproduce it. That makes the on-disk LAYOUT and
the module NAMESPACE the same decision. This codebase separates them on purpose:
`games.sonic4.constants` lives at `games/sonic4/config/constants.emp` because
`config/` is a layout detail and `games.sonic4` is the namespace. 93 of 122
modules disagree with their directory chain deliberately.

A lint that reports the convention as a defect is not enforcing a rule — it is
disagreeing with a decision already made. And it was the top line of the tally
the warning-tier parcel had just made visible, so the very first thing a reader
would learn is that most of the number is noise. **That is the invisibility
failure in a new costume:** the parcel's whole purpose is a line people act on,
and a line that is 84% known-noise trains people to discount it.

## The new rule

The id's **last segment** must equal the **file stem**. Everything above the last
segment is a free semantic namespace.

That keeps the half a stale rename actually breaks — the file a reader opens is
named after the module they looked up — and drops the half that was fighting the
architecture. `file_stem_of` replaces `expected_id_from_path` (single caller).

The message now states the real obligation instead of proposing a whole-id
rewrite:

```
[module.path-mismatch] module `engine.s4lz` ends in `s4lz` but its file is
`engine/compression/s4lz_decompress.emp` — the last id segment and the file
stem must agree (rename the file or the header)
```

## Measured

| | before | after |
|---|---|---|
| `module.path-mismatch` firings | 93 | **12** |
| whole tally line (`sonic4` plain) | 111 | **30** |

The 12 survivors are genuine disagreements, and the list is short enough to read:
`engine.s4lz` in `s4lz_decompress.emp`, `engine.zx0` in `zx0_decompress.emp`,
`engine.compression_vectors` in `debug/generated/vectors.emp`,
`games.sonic4.parallax_configs` in `data/parallax/configs.emp`, and eight
act-suffixed OJZ data modules whose ids carry an `_act1` the filename does not
(`games.sonic4.ojz_bg_anim_act1` in `bg_anim.emp`). Each is a real "the file is
not named after the module" case. **None is fixed here** — this parcel changes
the rule, not the corpus; the 12 are now a short actionable list rather than
being buried under 81 false positives.

## Tests

`resolve_manifest::indexes_modules_by_header_and_lints_path_mismatch` gains the
corpus shape as a NEGATIVE: `games/sonic4/config/constants.emp` declaring
`module games.sonic4.constants` — a flat id under a deeper directory, which must
now be silent. **Non-vacuity is carried by the total count**, which stays
`assert_eq!(warnings.len(), 1)`: under the old whole-path rule that same file
fired, so the assertion would read 2 and the test fails. The positive case
(`misplaced/here.emp` declaring `engine.objects.sst` — stem `here` vs last
segment `sst`) is unchanged and still fires, so both directions are pinned by one
test.

`path_mismatch_lint_carries_its_id` is unchanged and still passes — the id tag
survives the rewording, which is what the warn-tier gate keys on.

## Gates (own-run, chain 43)

Seven targets, built in `capture_goldens.sh` order, compared with `cmp`:
`s4` `a4db281b`/413276 · `s4.debug` `f05f5b86`/423404 · `demo` `12289484`/91224 ·
`demo.debug` `18e5ec7f`/93963 · `config_a` `a55d8335`/423781 · `config_b`
`c639b01a`/304812 · `lean` `5a6ef417`/379822 — **all seven IDENTICAL**.
Diagnostics-only change, so byte-neutrality is the expected result and it is
measured, not assumed.

`repin --check`: pins.rs unchanged. `refreeze --check`: OK, tip `cheat-flag`,
chain len 43.

The warn-tier frozen baseline (`warn_tier_lint_ids_match_the_frozen_baseline`)
pins the firing lint-id SET per shape, not counts — `module.path-mismatch` still
fires, so the set is unchanged and the gate needed no edit. That is the ratchet
design working as intended on its first real test: a parcel that changes 81
firings touches no baseline, where a count ratchet would have demanded a
rubber-stamp.

## Step-3 (language/tooling) vs step-5 (engine)

**Step 3.** A lint that encodes a *convention* must be validated against the
corpus that lives under it before it ships, not after. This one was correct as
written and wrong as applied: nothing about it was buggy, it simply asserted a
policy the project had not adopted. The warning tier is what made that visible —
the lint had presumably been wrong since the day it was written and cost nothing
observable, because nobody could see it fire. **The first measurement of a new
surface is worth more than the surface.**

**Step 5.** The 12 survivors split into two engine-side classes worth a decision:
the two `*_decompress.emp` files and `debug/generated/vectors.emp`, where
renaming the FILE is the obvious fix; and the eight OJZ act-suffixed data
modules, where the id carries an `_act1` discriminator the per-act directory
already expresses (`.../ojz/act1/bg_anim.emp`). The latter is really a question
about whether generated per-act modules should be id-disambiguated at all once
the act is in the path — a multi-act question, so it should wait for a second
act to exist. Ledgered, not fixed.

**Neither bucket.** This is the third plan item this session found to be in a
different state than the plan said (T1 was already shipped; B′-0b's rider
prediction was wrong; this lint was enforcing a retired convention). The
pre-dispatch verification habit keeps paying.
