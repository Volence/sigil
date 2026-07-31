# 2026-07-30 — STAGE 3 OPENING · design gate (the keystone re-baseline + the folded remainder)

Status: **DESIGN NOTE — design gate only. No implementation, no twin deletion, no
build change, no aeon modification.** Sigil branch `stage3`, worktree
`.worktrees/stage3`. Masters at open: aeon `51cbcb6` / sigil `a7d8d0f`. Aeon
READ-ONLY for this gate.

Baseline (from the Stage-2 close packet, not re-verified here — read-only gate):
`sigil build` IS the build; the four mainline targets at the sigil-canonical
full-file pins `2198deb2/395374` · `1d895fcb/402696` · `0646d4bf/76851` ·
`7e4a358a/77244`; the two off-canonical at `80e602df/402742` (config_a) ·
`9eb2e8a1/286904` (config_b); PRIMARY assembled anchors `e5765873/dab4f06c` ·
`cfda98d3/20c5571d` UNMOVED; strict 2853/0 (2 ignored); six golden `.bin` +
four `offcanonical_sizes/*.txt` in `crates/sigil-harness/golden/`; re-freeze
machinery = `capture_goldens.sh` + `capture_offcanonical_sizes.sh` (one-step).

Cites: `.rs`/ledger/kill-list are sigil `a7d8d0f`; `.asm`/`.emp`/`.inc` are aeon
`51cbcb6`; kill rows numbered from `twin-scaffolding-kill-list.md`.

---

## §0 — THE FRAME: what "the Stage-3 opening item" IS

The Stage-2 close deferred a tail behind its OWN designed gate. That gate is the
**keystone re-baseline** (kill rows 93/94): the five remaining AS-side code files
(`player_common`, `test_player`, `test_enemy`, `act_descriptor.asm`,
`z80_init.asm`) whose flips are proven CRC-MOVING at the full-file/appendix layer
(the appendix shrinks ~1389 B; the Frozen chainer misplaces a config_a pointer at
0x11412). This note designs THAT event and the folded remainder around it, then
STOPS for the overseer's countersign. The re-baseline itself needs explicit
overseer authorization to move the six pinned goldens; nothing here moves a pin.

The load-bearing invariant that makes a re-baseline SAFE: **the flip moves ONLY
the full-file/appendix layer — the six assembled anchors do NOT move.** Every
`.emp` keystone region is already byte-proven by its surviving `_port` gate; the
chainer fix (row 94) keeps the config_a assembled anchor exact; so the only thing
that changes is the sigil-canonical convsym deb2 appendix (name mangling). Any
re-baseline where an *assembled anchor* moves is a BUG, not a re-baseline — that is
the discipline (§6).

---

## §1 — ORDERED EXECUTION PLAN (parcels · gates · rollback)

Seven parcels. P0/P1 are independent and reversible; **P2 is the authorized
re-baseline** (the opening item proper, morning-report gated); P3–P6 are the
folded remainder. Order rationale: the chainer fix (P1) MUST precede the keystone
flip (P2); tools deletion (P0) is independent and clears the deck; the demangler
(P3) rides P2's re-freeze window; Phase-B (P4) and the ownership flips (P5/P6)
follow once placement authority and the export mechanism are settled.

### P0 — TOOLS PHYSICAL DELETION (independent · byte-neutral · no re-freeze)
Delete `aeon/tools/{asl, as.msg, cmdarg.msg, ioerrs.msg, p2bin, fixheader}` and
`tools/.cache` (asl's cache). KEEP `convsym` (it consumes sigil's listing — not the
retired toolchain). See §4 for the full disposition + the ONE cost this incurs.
- **Precondition:** confirm no gate/script/`bin` SHELLS a deleted tool in a path
  that CI or the build runs. Known live shell-out: `sigil-frontend-as/src/bin/
  gen_snippet_vectors.rs` (`tools/asl`, §4) — a MANUAL regenerator, not a CI gate;
  its committed vectors survive, only regeneration dies. `fixheader` is redundant
  (the checksum is folded into `emit_rom`, close-packet §0). `capture_goldens.sh`
  is now sigil-driven (build.sh = `sigil build`), so the re-freeze machinery does
  not need asl.
- **Gate:** all six targets hold at their pins (stale-artifact-trap-guarded
  six-target proof); strict green; `grep` sweep proves no residual live shell-out.
- **Rollback:** `git revert` (binaries live in aeon history).

### P1 — THE ROW-94 CHAINER FIX (TDD · byte-neutral to all six · no re-freeze)
The Frozen chainer (`native.rs::true_bases_by_index`, `SizeSource::Frozen`) derives
a section's true base from a contained frozen label, or — for a LABEL-LESS data blob
— by contiguity from its frozen neighbour (`native.rs:962-1000+`). When the `.emp`
keystones enter the config_a chained set they change the section composition, so a
label-less config_a data blob's contiguity derivation misplaces (anchor diverges at
0x11412, sig `0x12` != gold `0x13`).
- **TDD shape:** the bug only manifests when the keystones chain, which is not the
  shipped config today. So the red is a NEW test fixture that places the keystones in
  the config_a chained set (a test-only profile, NOT the shipped build) and asserts
  the config_a **assembled anchor** `3d9bac53`. Red = 0x11412 divergence. Fix the
  contiguity/label derivation. Green = anchor exact.
- **The soundness bar:** the fix must be byte-neutral for ALL SIX CURRENT targets
  (keystones still AS-side — the fixture is test-only) AND correct for the
  keystones-chained future. The current config_a/config_b/demo pins CANNOT move.
- **Gate:** red→green fixture test + six current pins unchanged + strict.
- **Rollback:** `git revert` (the fix is chainer-internal; nothing shipped changed).

### P2 — THE KEYSTONE FLIP === THE AUTHORIZED RE-BASELINE (morning-report gated)
For each of `player_common`, `test_player`, `test_enemy`, `act_descriptor`,
`z80_init`: add its `SIGIL_EMP_<X>` to `code_gate_defines()`; add its `ModuleSpec`
to the registry; drop it from `as_owned_keystones` (all three sonic4/config_a/
config_b profiles). `z80_init` additionally gets off-canonical native placement +
the numeric size mirror (§2). The always-emitted zero-byte HEADERS stay AS-side
(surviving cross-seam readers: camera.emp `PL_STATE_ADDR`, the drift guards,
test_animated `DplcV`, entity_data `ObjDef` refs) — the flip is add-gate +
add-ModuleSpec + drop-from-keystones, NOT a plain delete.
- **Retires:** `as_owned_keystones` field + `STAGE1_INAPPLICABLE_GUARDS` allowlist
  (native.rs:807, and the `enforce_inapplicable_allowlist` tests) — they exist only
  while the keystones are AS-owned. The keystone `_port` region gates TRANSFORM to
  "built region == frozen-golden slice"; `z80_init_port` survives (§6).
- **Assembled anchors HOLD** (chainer fixed in P1 + the `.emp` regions `_port`-proven);
  **full-file CRCs MOVE** (appendix shrinks ~1389 B — `.emp` locals mangle
  `$module$Proc$local`, which convsym DROPS). → **RE-FREEZE #1**: overseer-authorized;
  `capture_goldens.sh --write` + `capture_offcanonical_sizes.sh` (the size tables
  move because the keystones now chain). PROVENANCE.md entry (§6).
- **Gate:** six-target stale-artifact-trap-guarded proof AT THE NEW full-file CRCs;
  **assembled anchors confirmed UNMOVED** (the safety invariant); strict green; t24
  verbatim on every surviving golden gate + the row-91 witness.
- **Rollback:** `git revert` — but the working build is sigil-solo; the twins live in
  history. This is the parcel whose plan goes to Volence's eyes before it runs.

### P3 — THE APPENDIX DEMANGLER + FILTER POLICY (byte-neutral to ROM · re-freeze)
A listing-side demangler in `sigil-link::listing` / `native::build_native_rom_with_
listing` that rewrites `$module$Parent$local` → `Parent.local` BEFORE `emit_listing`,
so the ~906 mangled locals SURVIVE convsym's name parser and the appendix grows
676 → ~1394 symbols (ledger S1.4 row (i)). Settle the `asmN.*`/`__offsets$…` FILTER
policy (row (ii)) as a cheap pre-req. Byte-neutral to the ROM (listing-only). →
**RE-FREEZE #2** (appendix grows). **Co-scheduling ruling in §1-Q1 below.**
- **Gate:** ROM byte-identical (assembled anchors AND full-file pins hold except the
  appendix); appendix symbol count 676→~1394; convsym keeps the demangled names.
- **Rollback:** `git revert` (listing-only, ROM untouched).

### P4 — THE PHASE-B CARGO (row 95 · architectural · declared-sizes terminal settlement)
Section-split the AS-residual at natural boundaries; grow `sigil.map.toml` into the
placement manifest (§3); make the map+chainer the placement authority; retire
repin's asl-`.lst` parse (row 34, `repin.rs` `Listing`) + the pins; re-derive the
four `offcanonical_sizes/*.txt` from SIGIL's own layout (the declared-sizes
doctrine's terminal settlement — the last asl-derived constants retire, §6).
- **Gate:** all six at their pins with placement sourced from the map (not baked
  orgs); the size tables re-derived match; strict green.
- **Rollback:** `git revert` — map-driven placement is additive until the baked
  resume orgs (row 6/58) actually delete; delete those LAST.

### P5 — CONSTANTS + STRUCTS OWNERSHIP FLIPS (byte-neutral · gated on the OQ-5 spike)
`engine/constants.asm` (rows 1/2/12/14/17/19/20) and `engine/structs.asm` +
its code-arm (rows 7/8/11/15/25) → the `.emp` (`engine.constants` / `engine.structs`
+ the sst/act/EntityScanState overlays) BECOMES the definition; residual-AS readers
take exported equs. **DEPENDS on OQ-5**: the `.emp`→residual-AS equ-export path must
work for residual DATA files (config, generated tree, parallax, demo game-side,
macros.asm), not just the now-deleted code twins. Scope as two parcels; gate each on
a one-file OQ-5 spike first. Byte-neutral → golden-anchored byte gates + drift-guard
retirement, no oracle A/B.
- **Rollback:** `git revert` (byte-neutral; the export is additive).

### P6 — THE GAME-CONSTANTS `.emp` MODULE (split: untyped now / typed deferred)
Born to absorb SONG_*/SFXID_*/VRAM_*/BUTTON_A-B-C-START/PPHYS mirror truths (rows
18/22/54/62/65/76/77/79) → `config/constants.asm` + `config/sound_ids.asm` can
retire. **Split:** the untyped bulk moves now (byte-neutral); the 2 typed
`SFXID_RING_*` mirrors (row 10 tail) DEFER to the post-flip language round
(typed-extern grammar). BUTTON_A/B/C/START hoist into `engine.constants.emp` is its
own ripple (pinned engine-constants count + every consumer's guard tally).

### DEFER TO THE POST-FLIP LANGUAGE ROUND (not opening items)
Rows 21/53 (the `.emp`-native diagnostics-runtime rewrite — needs row 52's
link-time-equ-off-external-base capability); rows 4/18/22 (the `.b`/`.w` imm-link
deferral — the reverse-seam ordinal + game-config flips); rows 9/45 (the
game-contract-hook mechanism); row 10-typed (typed-extern grammar). Each is a named
language dep, scoped in the flip design §5, not this gate.

---

## THE SIX ANSWERS

### Q1 — THE RE-BASELINE GATE (row 93) + the demangler co-landing
The ordered plan is **P1 (chainer fix, TDD, zero pin movement) → P2 (the five flips,
assembled anchors held, ONE authorized re-freeze) → P3 (demangler)** as detailed
above. The chainer fix is TDD-able WITHOUT flipping the shipped build by driving the
red from a test-only profile that chains the keystones and asserts the config_a
assembled anchor `3d9bac53`; the shipped six pins never move because the fixture is
test-only, so the fix lands byte-neutral and P2's anchor then holds by construction.

**Should the demangler land in the SAME re-baseline?** Both ways:
- **FOR (one appendix change):** the keystone flip and the demangler touch ONLY the
  appendix/full-file layer and are byte-neutral to ROM behaviour; both need a
  six-golden re-freeze. They are the SAME name-mangling surface — the keystone
  locals (`$module$Proc$local`, which convsym DROPS) are exactly the names the
  demangler rescues (`Parent.local`, which convsym KEEPS). Landing them together =
  ONE re-freeze, ONE PROVENANCE entry, ONE morning-report review, and the appendix
  reaches its FINAL shape in one step. Separate = the appendix moves twice (shrink,
  then grow) and a lone intermediate "shrunk appendix" (676 minus the keystone
  locals) is a strictly-worse resting point than either endpoint.
- **AGAINST (separate):** the keystone flip is a CORRECTNESS/architecture event
  (chainer fix + reverse-seam + registry surgery, bounded by the assembled-anchor
  bar); the demangler is a UX polish with its OWN unsettled policy (row (ii):
  `asmN.*`/`__offsets` — emit-all vs filter vs prefix-policy). Coupling an
  authorization to move the pins to an open cosmetic debate widens the blast radius,
  and if the policy is wrong you re-freeze again anyway.
- **RECOMMEND:** settle the demangler filter policy (row (ii)) as a cheap pre-req,
  then land P2 and P3 as **two sequential commits in one review window with ONE
  combined re-freeze at the end** — same appendix surface, no doubly-moved CRC, no
  worse intermediate resting state. **Fallback:** if the policy debate runs long, ship
  P2's re-freeze ALONE and let P3's re-freeze follow. The keystone flip must NEVER
  wait on the demangler; the demangler SHOULD ride the same window when ready.

### Q2 — z80_init native placement + the reverse-seam / comptime-wall
Placement is already PROVEN: t28's `mixed_offcanonical_rom::mixed_z80_init_config_b_
rom_matches_reference` places `z80_init.emp` at 0x3D8 == pure-asl, byte-exact. P2
does this in the SHIPPED config_b/demo profile registries (add the `ModuleSpec`,
collapse boot_data.asm's `SIGIL_EMP_Z80_INIT` else-arm `org $3FE` + `Z80_IDLE_SIZE`).
The one real obstacle is the comptime wall: `z80_init.asm` DEFINES `Z80_IdleProgram`,
consumed by boot_data.asm's same-file comptime `if (Z80_IdleProgram-BootData) <> 54`
— a cross-seam `.emp` link symbol CANNOT satisfy a same-file comptime `if` (the
row-52 "no equ off a link-external base at comptime" wall). **Justified alternative
(recommended):** do NOT try to fold a link symbol at comptime. Keep the size as a
NUMERIC AS-side constant that boot_data.asm folds at its own assemble time, and give
`z80_init.emp` a self-`ensure(sizeof(idle_body) == 54)` — the idle-body size is a
compile-time property of z80_init.emp, checked THERE, and mirrored numerically
AS-side under a drift ensure (the row-47/row-52 numeric-pin class, and the row-46
BootData cursor wall already stays AS-side by the same logic). A link-label export of
`Z80_IdleProgram` remains available for any RUNTIME (non-comptime) AS reader, but the
assert wall specifically stays numeric. The numeric mirror collapses when boot_data.asm
itself ports (a later stage).

### Q3 — THE PHASE-B CARGO (row 95) + the final `sigil.map.toml`
Sequence: (1) **section-split** the AS-residual (config, generated tree, parallax,
demo game-side, macros.asm invokers) at natural boundaries so each is a named,
map-placeable section; (2) **grow `sigil.map.toml`** into the placement manifest —
the region LMAs + the section ORDERING that main.asm/engine.inc's gate-resume `org`s
encode today become named orderings (SPEC2:199), and the `$20000` object-bank
`if * > $20000 / error` (engine.inc:653-662) becomes a per-section size-report /
region-budget check; (3) make the DECLARED-ORDER CHAINER the placement authority
(it already computes every base — the map holds the declared ORDER + the declared
per-region SIZES = the exact asl spans the chainer's soundness needs, bases stay
COMPUTED); (4) retire repin's `.lst` parse (row 34) + re-derive the size tables from
sigil's layout; (5) delete the row-6/58 residual placement literals LAST. The final
map's shape:
```
fill = 0x00
[[region]]  name = "rom"        lma_base = 0        size = 0x400000   # ROM terminus
[[region]]  name = "object_bank" lma_base = 0x10000 budget = 0x10000  # was: if * > $20000 / error → size report
[[region]]  name = "z80_moving_trucks_bank" lma_base = 0x60000 size = 0x8000 vma_base = 0x8000 kind = "z80_bank"
# per-game declared section ORDER (what the main.asm gate sequence encodes) +
# declared per-region SIZES (exact asl spans; the chainer's relaxation anchor) —
# BASES stay computed by the chainer, not pinned.
[order.sonic4]  sections = ["vectors","boot",...,"object_bank","sound_bank_head",...]
```
Exact schema (how much is declarative vs computed) is OQ-6, an execution detail once
the native driver reads the map as authority. Recommend: map holds geometry + order +
declared sizes; the chainer computes bases; budgets become size reports.

### Q4 — TOOLS PHYSICAL DELETION (Volence nothing-retained applied)
**DIES:** `tools/asl` (2.4M) + its message catalogs `as.msg`/`cmdarg.msg`/`ioerrs.msg`
+ `tools/.cache`; `tools/p2bin` (replaced by `emit_rom` reading the map); `tools/
fixheader` (the checksum is folded into `emit_rom` — already redundant). **STAYS:**
`tools/convsym` — it consumes sigil's sigil-canonical `.lst` listing (the deb2
appendix); it is a symbol-format converter sigil's own listing feeds, NOT part of the
retired toolchain (row 34 retires convsym's asl-listing SOURCE, not convsym). The `.py`
generators / `s4budget.py` / `s4lint.py` / `salvador` / `bin/` are the art/sound/level
+ lint pipeline, out of scope. **THE ONE COST TO FLAG (Volence's OQ-2 was overruled —
asl-binary NOT kept):** `sigil-frontend-as/src/bin/gen_snippet_vectors.rs` shells
`tools/asl` to MINT the ISA golden-vector corpus — the flip design §2.3 named that
corpus "the ONE piece of independent-asl witness that survives" for new post-flip
code, and recommended keeping it RICH. Deleting the asl binary means the committed
vectors stay valid (CI never needs asl) but **no NEW asl-derived vectors can be minted**
— extending the corpus for a new instruction shape a post-flip optimization introduces
is no longer possible from this repo. Under nothing-retained this is an accepted loss;
it is surfaced as OQ-A so the overseer rules it with eyes open.

### Q5 — THE REMAINING STAGE-3 SET (scope each)
- **constants + structs ownership flips (P5):** parcel-sized, byte-neutral, but GATED
  on the OQ-5 export-to-residual-AS spike (prove `.emp`→residual-AS equ export on one
  representative residual DATA file first). Two parcels (constants is the largest
  mirror block; structs carries the sst/act/EntityScanState overlays + the code-arm).
- **game-constants `.emp` (P6):** parcel-sized but SPLIT — untyped bulk now
  (byte-neutral), typed `SFXID_RING_*` deferred to the language round (typed-extern
  grammar).
- **debug-runtime deps (rows 21/53):** DEFER — a rewrite (the `.emp`-native
  diagnostics runtime owning the message format), and row 52's numeric ErrorHandler
  pin needs link-time-equ-off-external-base (language dep). Not an opening item.
- **structs.asm code-arm:** folded into the P5 structs ownership-flip parcel.

### Q6 — STRICT/GATE IMPACT + re-freeze provenance discipline
Per parcel: **P0** no gate impact once the shell-out sweep is clean (removes dead
binaries; costs the vector-regen capability, Q4). **P1** adds the chained-keystone
fixture test (strict +≈1), byte-neutral, no re-freeze. **P2** retires
`as_owned_keystones` + `STAGE1_INAPPLICABLE_GUARDS` (the `enforce_inapplicable_
allowlist` tests go); the keystone `_port` gates transform to golden-slice comparands;
`z80_init_port` survives; **RE-FREEZE #1** moves the six `.bin` + four size tables.
**P3** byte-neutral to ROM; **RE-FREEZE #2** grows the appendix 676→~1394. **P4**
retires repin's `.lst` parse + the pins; re-derives the size tables from sigil's own
layout — the declared-sizes doctrine's TERMINAL settlement (the last asl-derived
constants retire). **P5/P6** byte-neutral; drift-guard negative-probe tests retire
with their ensures. **Re-freeze discipline:** each re-baseline is overseer-authorized,
ONE event, driven by `capture_goldens.sh --write` + `capture_offcanonical_sizes.sh`
AFTER the stale-artifact-trap-guarded six-target proof; a PROVENANCE.md entry records
the moved full-file CRCs + sizes, the UNMOVED assembled anchors (the primary
provenance — the invariant that makes the re-freeze safe), the appendix symbol delta,
and the cause. At P4 the entry additionally records that the `offcanonical_sizes/*.txt`
are re-sourced from sigil's OWN resolve, closing the asl-derived-constants era.

---

## COUNTERSIGN RULINGS (2026-07-30 — gate passed at 8b3e19d)

The overseer countersigned all six answers. The six rulings, now binding on
execution:

- **OQ-A → BOTH halves hold.** `tools/asl` + catalogs + `p2bin` + `fixheader`
  DELETE (nothing-retained governs the REPO, not the ecosystem). asl is the public
  Macro Assembler AS, so `gen_snippet_vectors.rs` keeps/gains an `ASL_BIN` env-var
  hook with a fail-loud "install AS and point `ASL_BIN` at it to mint new vectors"
  message; the golden-vector README documents this as the corpus-extension path.
  The committed corpus stays the frozen independent witness; extension survives
  out-of-repo; nothing is retained in-tree. (Fold this into P0.)
- **OQ-B → demangler policy READY; P2+P3 COMBINE into one re-freeze window.** The
  filter policy: (1) `$module$Parent$local` → `Parent.local` KEEP; (2) comptime-table
  names that demangle to source-meaningful names (`__offsets$…$Ani_Sonic$Balance`
  class — anim-table entries a debugger user wants) demangle and KEEP; (3) pure
  compiler-plumbing synthetics (`asmN.*` block-internal scopes, `__align` internals)
  DROP (noise in a backtrace). Net: one appendix change, one re-freeze; 676 → the
  demangled+filtered set.
- **OQ-C → CONFIRMED.** The test-only chained-keystone profile is the right red
  fixture; shipped pins frozen through P1.
- **OQ-D → CONFIRMED.** The numeric-mirror path (`Z80_IDLE_SIZE=54` AS-side +
  `.emp`-side self-`ensure`), not the wall relax.
- **OQ-E → CONFIRMED.** map = region geometry + declared order + declared sizes;
  chainer computes bases; budgets become size reports; final schema settles at P4.
- **OQ-F → CONFIRMED.** The OQ-5 spike must prove the export path against residual
  DATA readers (config, generated tree, parallax, demo game-side) before P5.

**Execution authorized in order:** P1 (chainer fix, TDD, shipped pins frozen) →
P2+P3 (the five flips + demangler, ONE combined re-freeze — a NAMED commit the
overseer countersigns before it lands: run the six-target stale-trap-guarded proof,
show the new values + the unmoved assembled anchors, THEN freeze) → P4 (Phase-B +
repin retirement + size-table re-derivation) → the tools-deletion commit (with the
OQ-A hook) → P5/P6 per their gates. **Mandatory checkpoints:** (1) after P1;
(2) BEFORE the re-freeze commit; (3) at close. Standing discipline: strict green at
every boundary (failures-first); the six assembled anchors are the invariant that
NEVER moves; t24 on everything new; plain-spoken standalone commits; kill rows
same-commit; the valve stands.

## OPEN QUESTIONS (load-bearing; for the countersign / morning report) — RULED ABOVE

- **OQ-A (the ISA vector-corpus regen — the real cost of physical asl deletion):**
  Volence ruled asl-binary NOT kept, overruling the design's OQ-2. But
  `gen_snippet_vectors.rs` shells `tools/asl` to MINT the ISA golden-vector corpus —
  §2.3's named surviving independent-asl witness for NEW post-flip code. Deleting asl
  keeps the committed vectors valid but ends the ability to add NEW asl-derived
  vectors. Accept the loss (nothing-retained is absolute), OR keep ONLY the asl
  binary out-of-tree for manual vector regen (not in the build, not in the repo)? This
  is the one place the nothing-retained ruling trades away a correctness witness; it
  should be ruled explicitly, not by default.

- **OQ-B (demangler co-scheduling — needs the row-(ii) policy first):** the
  recommendation to combine P2+P3 into one re-freeze window depends on settling the
  `asmN.*`/`__offsets` filter policy (emit-all / filter / prefix-policy) as a cheap
  pre-req. Is the policy call ready to make now, or does it need its own round? If not
  ready, P2 re-freezes alone (the keystone flip does not wait).

- **OQ-C (P1 chainer-fix red without a shipped flip):** the row-94 fix is TDD-driven
  from a TEST-ONLY profile that chains the keystones to reproduce the 0x11412
  divergence, with the shipped six pins held. Confirm this fixture shape is acceptable
  as the red — i.e. that adding the keystones to a scratch profile to exercise the
  chained path, without committing the flip, is the intended way to isolate the
  chainer fix from the re-baseline.

- **OQ-D (z80_init comptime wall — numeric mirror vs relax):** the recommendation
  keeps `Z80_IDLE_SIZE = 54` as a numeric AS-side constant with a `.emp`-side
  self-`ensure`, dodging the link-external-comptime-fold wall (row 52 class). The
  alternative is relaxing/re-homing boot_data.asm's assert wall entirely into `.emp`.
  Confirm the numeric-mirror path (cheaper, precedented) over the relax.

- **OQ-E (Phase-B map schema — declarative vs computed, = OQ-6 carried):** how much of
  the per-shape/off-canonical org geometry (rows 6/58) the grown `sigil.map.toml`
  states declaratively vs the chainer computes as link outputs. Recommend: map holds
  region geometry + section order + declared sizes; chainer computes bases; budgets
  become size reports. Settle at P4 once the native driver reads the map as authority.

- **OQ-F (OQ-5 export mechanism scope — carried):** P5's ownership flips need the
  `.emp`→residual-AS equ-export path working for residual DATA files (config, generated
  tree, parallax, demo game-side), not just the now-deleted code twins. Confirm the
  spike covers the residual data readers before scheduling P5.
