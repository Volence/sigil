# 2026-07-30 — FLIP STAGE 2 · THE 6×2 MATRIX CHECKPOINT (both blockers closed)

Status: **the two ruled blockers are CLOSED; the six-proof preamble is COMPLETE —
all six targets prove `native == golden` at BOTH layers (assembled anchor + full
file). The flip commit WAITS for the overseer's go (it is not this executor's).**

Sigil `flip-stage2` tip `a4b782d` (2 commits on `4a594f8`): `b3f82e8` (blocker 1,
the parallax `:=` capability) + `a4b782d` (blocker 2, the DAC internal-bank-align
recompute + the six-proof off-canonical gates). **ZERO aeon commits** — aeon
read-only at `28098af`; the asl default is inherently unchanged (no aeon-touching
commit), all four shapes at their goldens by construction.

## Blocker 1 — the parallax `:=` relocation capability (config_a + config_b)

An AS reassignable set-symbol (`set`/`:=`) whose RHS names/chains to a section
LABEL was baked to its this-pass VMA at every use site. `engine/parallax_macros.inc`
does `P_DBG := deformBg` then `dc.l P_DFG, P_DBG` INSIDE a config record the
chainer relocates, so every folded deform-table pointer went stale — config_b's
last diffs, all +0x32.

FIX (`crates/sigil-frontend-as/src/eval.rs`): a `:=` whose set-substituted RHS
references a label records its `relax_safe_fold`ed SYMBOLIC snapshot in
`set_sym_symbolic`; `expr_refs_label` + `relax_safe_fold` consult it so EVERY
symbolic-emit site keeps the label symbolic through placement (the linker
relocates). Per-use snapshot semantics fall out of emission order; chains resolve
via the root splice; reassigning to a label-free value reverts to baking. Read
only on the deferral pass (`keep_labels_symbolic`-gated) → ordinary-pass bytes
unchanged.

RED-first: `crates/sigil-frontend-as/tests/set_sym_relocates.rs` (dc.l-past-grown-
jsr, two-label snapshot, chain, const t24 control) reproduced the stale `[0,1,0,4]`
bake; now `[0,1,0,6]`. **config_b assembled anchor closes to 0 diffs.**

## Blocker 2 — config_a DAC band (the internal-bank-align recompute)

DIAGNOSED FIRST (`2026-07-30-flip-stage2-config-a-dac-band-diagnosis.md`). The
overseer's mirror hypothesis is FALSIFIED: config_a's DAC band is byte-identical to
the s4.debug golden (mirror = a 64-byte RAM proc, not a DAC feature). Actual cause:
the two DAC banks sit inside ONE pure-data section (HeightMaps → art → `align
$8000` → DAC blip → `align $8000` → DAC shared → `align $8000` → MovingTrucks). Its
INTERNAL `align $8000` pads were baked for the section's AS-RESIDUAL base (0x257ba);
the chainer re-pins it at its true frozen base (0x257c6, +0xC), so every internal
bank boundary landed +0xC — DAC content at 0x4800c/0x5000c (proof: native
`[0x4800c..]` == golden `[0x48000..]`, 2816/2816). asl aligns to ABSOLUTE
N-multiples base-independently; the baked Fill does not. The general form of
blocker 3 (its trim handled only the TRAILING align).

FIX (`crates/sigil-harness/src/native.rs` `recompute_bank_aligns`): replay
baked-vs-true absolute positions in parallel; rewrite each `>= 0x8000` align pad so
content resumes on the same absolute bank boundary. Word-aligns / bulk fills
(`< 0x8000`) untouched → the section HEAD is unmoved. Byte-neutral for the pinned
sonic4 path (baked == true, the loop is a no-op).

## THE 6×2 MATRIX (pre-flip, both layers GREEN)

| target | anchor (asl golden prefix, header-neutral) | full file (sigil-canonical CRC/len) | gate |
|---|---|---|---|
| sonic4 plain  | ✅ `[0,ASSEMBLED_LEN)` == e5765873 | ✅ 2198deb2 / 395374 | native_rom + native_full_rom |
| sonic4 debug  | ✅ == dab4f06c | ✅ 1d895fcb / 402696 | native_rom + native_full_rom |
| config_a      | ✅ `[0,0x5f65a)` == b4a6756d prefix | ✅ **80e602df / 402742** | native_offcanonical_{rom,full} |
| config_b      | ✅ `[0,0x434d0)` == 92776720 prefix | ✅ **9eb2e8a1 / 286904** | native_offcanonical_{rom,full} |
| demo plain    | ✅ `[0,0x11224)` == 18c64002 prefix | ✅ **0646d4bf / 76851** | native_offcanonical_{rom,full} |
| demo debug    | ✅ `[0,0x11224)` == b0475a59 prefix | ✅ **7e4a358a / 77244** | native_offcanonical_{rom,full} |

Full-file functional truth per off-canonical target (the S1.4 blessed pattern):
determinism (2nd build byte-identical), `de b2` presence + per-target size floor
(demo floors `DEMO_APPENDIX_FLOOR` = 0x1000, its engine-only set packs ~0x1a0f),
convsym load-bearing spot-check through the REAL consumer, and a per-target t24
doctored-address negative control. The asl-derived golden `.bin` blobs remain the
ANCHOR source; the sigil-canonical full-file blob freeze rides the flip commit's
golden-freeze stage.

## STRICT COUNTS

- **Worktree env** (`AEON_DIR=<aeon>/.worktrees/flip-stage2`): **2938 / 0 / 1
  ignored**, exit 0. (= prior 2923 + 15 new: 4 `set_sym_relocates`, 4 off-canonical
  anchor, 7 off-canonical full-file.)
- **Main-checkout env** (`AEON_DIR=<aeon>` master `bcb8f64`): **2905 / 33 / 1**,
  `--no-fail-fast`. All 33 reds are EXACTLY the `-D`/`const` collision class and
  nothing else — every one fails with `[defines.collision] 'MAX_RING_BUFFER' /
  'VRAM_RING_PLACEHOLDER' / 'COLLECTED_WINDOW_SLOTS' is provided by -D and declared
  by the module`: the harness supplies the game-config as `-D` (matching aeon
  precursor `4c03d43`, which promotes those `const`s to comptime inputs), but the
  master aeon still DECLARES them `const`. NO red has any other cause (grep of the
  run confirms the only error type is the collision; every byte/anchor/drift string
  in the log belongs to a PASSING test). Branch-inherent, accepted per the ruling.

  The 33: native_rom_{plain,debug}, native_full_sonic4_{plain,debug},
  deb2_appendix_negative_controls, declared_chain_{plain,debug},
  chained_resume_{plain,debug}, config_b_frozen_placement_exact,
  {config_a,config_b}_{anchor_matches_golden,full_file,doctored_control},
  demo_{plain,debug}_anchor_matches_golden, demo_{plain,debug}_full_file,
  demo_doctored_control, entity_window_{,debug_}region_matches_reference,
  two_module_ownership_flip_{plain,debug}, rings_{,debug_}region_matches_reference,
  snd_combo_matches_as_twin, doctored_game_mirror_fires_its_guard,
  mixed_tranche{8,9}_{,debug_}rom_matches_assembled_reference.

## The valve

Additive capability + gates only; ZERO aeon commits; sonic4's five native gates +
whole worktree strict green; asl default inherently byte-identical; the row-91 DSM
witness untouched; t24 on every new gate. Both ruled blockers closed with byte-level
evidence — no unsound approximation. **The flip commit remains the overseer's; it
WAITS for the go.**
