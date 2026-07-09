# Port #2 tranche (controllers.asm + math.asm) — COMPLETE through step 2, awaiting checkpoint

First tranche run under the ratified 4-step loop (transcribe → modernize → retrospect →
implement). Both branches UNMERGED.

- **sigil `port-tranche2`** (worktree, off master `f620e56`, 13 commits, head `b2d4ee1`):
  pc-rel EAs (`7c64a76`) → byte gates + embed_base + jsr/jmp cross-seam deferral (`0e934a8`) →
  org-aware relaxation (`2aea71d`/`a2678eb`) → step-2 wiring (`36965b8`/`4dc890c`) → I1
  diagnostic fix (`b2d4ee1`) + docs/ledger commits.
- **aeon `sigil-emp-tranche2`** (main tree, off `ac7e290`, 4 commits, head `2496958`):
  transcriptions + gates (`7009343`) → strict-nesting indent (`eff8630`) → ruled banners
  (`e8c9f3c`) → step-2 modernize (`2496958`: engine/constants.emp twin, `use` + jbsr +
  @as_compat off controllers/math, typed sine table).

**Validation:** strict gates **200/0, zero skips** — controllers/math port gates both shapes,
five negative probes (incl. the constants drift guard), **six-module full-ROM mixed gates
byte-identical to both reference ROMs** (the org-aware acceptance), hblank + sound suites +
m1d full-ROM AS gates intact. Workspace 147/147 suites (~1841 tests), clippy clean. Aeon
gate-off byte-neutrality sha256-proven ×3 builds at T2 (references restored byte-identical).
Reviews: T1 (pc-rel) two-verdict APPROVED incl. the accepted cross-section deviation; T4
(org-aware) two-verdict APPROVED after the deepest probe set of the campaign; whole-branch
two-prong APPROVED (I1 Important fixed on-branch; code-sense verdict on all three files:
"WANT IT").

## What the tranche shipped beyond the two files

1. **68k PC-relative EAs** — `Sym(pc)` + `Sym(pc,Xn.size)`, AS-parity byte-pinned;
   cross-section resolves (reviewed + accepted: same VMA-distance seam as branches).
2. **`embed_base`** — `embed("../…")` paths resolve from the module's own dir while
   `include_root` stays the security boundary (escape-probed).
3. **Cross-seam `jsr`/`jmp` deferral** — targets undefined in the AS compile unit defer to
   the linker (the real `jsr GetSineCosine` consumer shape) + **I1**: the unresolved-target
   link error now names the symbol with the cross-seam steer (shared helper with RelaxAbsSym).
4. **Org-aware relaxation** — orgs are position barriers partitioning sections into runs;
   the M1.C blanket refusal became a precise overrun diagnostic; unlocked the six-module
   full-ROM gate. (The campaign's biggest linker win so far.)
5. **The constants-twin pattern** — `engine/constants.emp`: `pub const` mirrors +
   `ensure(extern("X") == X)` drift guards that fail the LINK naming the constant; zero-byte
   carrier; negative probe proves exactly-one-guard-fires. THE campaign answer to cross-seam
   comptime constants (constants.asm stays authoritative until its own port at campaign end).
6. **Conventions locked:** strict bracket nesting; ruled `// ---` banners (Volence taste
   ruling); step-2 `@as_compat` removal precedent (evidence-based per file — controllers/math
   are the first files off it).

## RETROSPECT #2 (step 3 — rulings requested)

| Entry | Recommendation |
|---|---|
| Items 1–5 above + jbsr/@as_compat-off precedent | SHIPPED in-tranche — ratify with merge |
| **hblank.emp `@as_compat` removal** — proven inert during step 2, deliberately not landed unprompted | APPLY next tranche's step 2 (batch with its files) |
| **Backward-org-into-fixup ordering hazard** (T4 review discovery; pre-existing, latent — aeon's idiom never hits it) | Fix when next touching `link()` ordering; not urgent, keep OPEN |
| **build_program `report_unresolved` vs cross-seam `use` modules** (worked around in 3 test files) | Keep rule-of-three deferred; the workaround is documented + faithful today (M2 fidelity note recorded) |
| Small opens: `pc` reserved-token doc line; PcRel range-message symbol naming; abs.l destinations; `Owner.label(pc)` test | Bundle opportunistically into any tranche |
| **Stale `demo.bin`/`demo.lst` references** in the aeon tree (predate port #1 + sound merges) | Refresh at next convenient aeon build session, or pin properly in PROVENANCE |
| **Reproducibility item NARROWED**: main-tree fresh builds DO reproduce the s4 pins; the port-#1 anomaly is worktrees missing untracked generated state | Keep the "own session" for the real fix (track/pin generator outputs); severity downgraded |
| **ram.asm pre-port audit** (Volence's parallel session; uncommitted in the main-tree ledger) | Merge the note in when committed; its conditional-`vars`-fields decision gates the eventual ram tranche |

## Checkpoint asks

1. Merge sigil `port-tranche2` → master (`--no-ff`), remove worktree, delete branch.
2. Merge aeon `sigil-emp-tranche2` → master (`--no-ff`), delete branch.
3. Rule on the retrospect table.
4. **Tranche 3 proposal:** `collision_lookup.asm` (44 ln, 6 imports) + `vdp_init.asm` (47 ln)
   — the last two small code targets from the kickoff ranking; optionally interleave the
   align-consuming data quick-wins (`vram_bases`, `ojz_act_pool`, `particle_anims` — also the
   first real `offsets` + inline-bodies exercise).
