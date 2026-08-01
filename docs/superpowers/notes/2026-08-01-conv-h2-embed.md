# conv-h2 — the `embed` construct + the boot_data STOP + Parcel J deletes

**Parcel:** H2 (Opus porter; overseer countersigns/merges).
**Repos:** aeon `conv-h2-embed` · sigil `conv-h2-embed` (worktree `.worktrees/opt-sweep`).
**Date:** 2026-08-01.

Three pieces. Headline: **the `embed(...)` data expression SHIPPED** (reconciled
to the ratified spec), **Parcel J (#25/#26/#27) DELETED** (byte-neutral,
six-CRC identity held), and **#12 boot_data STOPPED per the embed spec §3 STOP
rule** — the blob-embed half is now expressible but the org-$3FE hole half is
not, with numbers below.

---

## 1 — The `embed` construct (sigil)

`embed(...)` already existed at T1 (`cf1ac0c4`, `eval_embed` in
`crates/sigil-frontend-emp/src/eval/sandbox.rs`) as the comptime data
expression returning `Value::Data` inside the capability sandbox, with a
capture ledger. conv-h2 reconciled that shipped surface to the ratified spec
(`docs/superpowers/specs/2026-08-01-embed-construct-design.md`, now SHIPPED-
stamped).

### Shipped surface vs spec — deviations (all deliberate)

| Spec §1/§2 item | Shipped | Verdict |
|---|---|---|
| `embed("p")` data expression, length inferred | `pub data X = embed("p")` — the `Value::Data` lowering length-infers when the annotation is omitted | **AS SPEC** (via omit, see next row) |
| `[u8; _]` inferred-length spelling | `_` in array-length position parses as an ordinary path, NOT an inference hole (`layout.rs::resolve_type` → `eval_const_index`) | **NOT the `_` spelling.** The spec's own §1 contingency governs: v1 = omit-the-annotation to infer, or `[u8; N]` to assert. REPORTED. |
| `[u8; N]` asserts EXACT size → `[embed.length-mismatch]` | routes through the general data-item size check `[emit.size-mismatch]` (`emit.rs::lower_data_value`), already actual-vs-declared | **DEVIATION (better-not-same):** the general check is strictly more general (covers `bytes([...])`, `++`, everything); a bespoke embed code would be narrower for no gain. KEPT `[emit.size-mismatch]`. |
| slice `embed("p", offset, len)` (positional) | `embed("p", skip: N, len: M)` (NAMED) | **DEVIATION:** no consumer needs slicing (boot_data embeds the whole blob); named form was shipped+tested at T1. KEPT named. |
| missing/unreadable → `[embed.not-found]` naming RESOLVED ABSOLUTE path | **FIXED** from T1's `[embed.read] <relative>` → `[embed.not-found] <resolved abs>` | **AS SPEC** (this parcel's code change) |
| `[embed.range]` out-of-range slice | present, unchanged | AS SPEC |
| build freshness on a changed binary | `record_capture` logs each embed's SHA-256 + byte length (append-only ledger) | AS SPEC (tracked) |
| comptime byte-reading OUT | OUT | AS SPEC |

### Code change (this parcel)

`sandbox.rs::eval_embed`: the read-failure arm now emits
`[embed.not-found] cannot read <resolved.display()>` (absolute path) instead of
`[embed.read] cannot read <relative>`. One line of contract, matching spec §2.

### Tests (`tests/sandbox_embed.rs` — 5 → 9)

- `embed_missing_file` — retargeted to `[embed.not-found]`.
- `embed_missing_file_names_resolved_path` (NEW) — asserts the message contains
  the resolved absolute path `<vectors>/does_not_exist.bin`.
- `embed_infers_length_without_annotation` (NEW) — `pub data BootBlob =
  embed("embed_fixture.bin")` → size 12, no annotation.
- `embed_explicit_length_exact_ok` (NEW) — `[u8; 12]` over the 12-byte fixture
  lowers clean.
- `embed_explicit_length_mismatch_rejected` (NEW) — `[u8; 8]` over the 12-byte
  fixture → size-mismatch naming both 12 and 8.
- `subcommands.rs::sigil_emp_error_exits_nonzero` — retargeted to
  `[embed.not-found]`.

Byte-exact emission proof: `embed_full_file` / `embed_with_skip_and_len` assert
`Cell::Bytes` against the known `embed_fixture.bin` (bytes `0x00..=0x0B`).

---

## 2 — #12 boot_data → `.emp`: STOPPED (embed spec §3)

### The port map (what boot_data.asm is)

`engine/system/boot_data.asm` (131 lines) is the `BootData` table walked by ONE
sequential `(a5)+` cursor from the `.emp` boot code (`boot.emp:89 lea
BootData(pc), a5`). Its geometry, in order:

1. movem preload — 3 words (`d5-d7`: VDP cmd base, RAM clear lcnt, Z80 bus val)
   + 5 longs (`a0-a4`: Z80_RAM, Z80_BUS_REQUEST, Z80_RESET, VDP_DATA, VDP_CTRL)
   — **these 5 longs are LINK labels** (a flat blob can't hold them; `.emp`
   `data` / `dc.l Label` can).
2. `BootData_VDPRegs` — 24 VDP register bytes (`vdp_init.emp:27` reads these
   cross-seam).
3. VRAM DMA-fill command (`dc.l vdpComm(0, VRAM, DMA)`).
4. **the resident Z80 blob** — `BINCLUDE z80_sound_blob[_debug].bin` between
   `Z80_Sound_Start`/`Z80_Sound_End`, `Z80_SOUND_SIZE = End - Start`
   (SOUND-ON arm) **OR** `org $3FE` over the z80_init idle-program hole
   (SOUND-OFF arm).
5. `align 2` + PSG silence (4 bytes) + post-DMA VDP commands (`dc.w
   vdpReg(...)` + 2×`dc.l vdpComm`). `BootData_End`.
6. the layout-assert WALL (`if … fatal`) locking every waypoint offset the
   blind `(a5)+` cursor depends on.

Consumers of its exported symbols (would become cross-`.emp` refs on port):
`boot.emp` (`BootData`, `Z80_SOUND_SIZE`, `Z80_IDLE_SIZE`), `vdp_init.emp`
(`BootData_VDPRegs`).

### Blob-embed half: UNBLOCKED

The #12 census blocker was "`.emp` has NO binary-embed." `embed(...)` now
supplies it: the SOUND-ON blob would be `pub data Z80_Sound_Start =
embed("engine/sound/generated/z80_sound_blob.bin")` (root-relative, the SAME
generated file the AS `BINCLUDE` reads — seam-1 emit). The assert wall would be
`ensure(...)` calls. That half expresses cleanly.

### org-$3FE hole half: DOES NOT EXPRESS → STOP

The SOUND-OFF arm does `org $3FE`, which in the AS model MOVES the assembly
cursor forward WITHOUT emitting bytes, leaving `$3D8..$3FE` (38 bytes,
`Z80_IDLE_SIZE = $3FE-$3D8`) to be filled by a SEPARATELY chained module —
`z80_init.emp`'s `Z80_IdleProgram` (placed by the frozen chainer at `0x3d8`).

`.emp`'s data surfaces are `align N` and a byte-count `pad`
(`ast::DataItem::Pad{count}`) — **there is NO absolute-`org`/hole surface.** A
`pad(38)` would EMIT 38 fill bytes, and `z80_init.emp` is ALSO chained into
`$3D8` → module overlap, not a hole another module overlays.

**This arm is LIVE for 2 of the 6 identity targets:** demo + demo.debug build
`SOUND_DRIVER_ENABLED=0` (`build.sh:28`, `games/demo/main.asm:6`), take the
`else` arm, and their ROMs carry the 38-byte `Z80_IdleProgram` at `$3D8`
(verified: `demo.bin` 9bb8c993/90506, `demo.debug.bin` bc7678d0/93006 both
built and matched this parcel). demo includes `engine.inc` → `boot_data.asm`
(`games/demo/main.asm:44`).

Reproducing the hole in native placement needs a CONDITIONAL two-module split
(pre-hole `boot_data` head + post-hole tail, with the chainer overlaying
`z80_init` between them for the no-sound shape, one contiguous embed module for
the sound shape) — two new per-shape pins, a shape-varying native registry, all
under the six-CRC identity bar. That is a large native-placement restructure,
**exactly the shape the embed spec §3 STOP rule reserves** ("if the org-hole
shape needs more, STOP with the finding").

**Decision: STOPPED.** boot_data.asm stays AS-residual. The embed construct is
proven against a golden binary via the fixture tests instead (its spec §3 proof
obligation is met by `sandbox_embed.rs`, not by moving the file). Follow-up
candidate ledgered in the gap-ledger: an `.emp` absolute-org / reserved-hole
surface (or a chainer inter-module-hole mechanism) is the prerequisite for #12.

boot_data.asm was NOT modified (no half-port). Its `BINCLUDE` stays — the AS
frontend already has it; `embed` is the `.emp`-side spelling, not needed inside
an AS file.

---

## 3 — Parcel J (#25/#26/#27): DELETED

Volence pre-ruled DELETE (census tail decision). The three PARKED editor-export
`.asm` files under `games/sonic4/data/editor/ojz/act1/export/`:

- #25 `act_descriptor.asm` (217 lines)
- #26 `entity_data.asm` (159 lines)
- #27 `vram_bases.asm` (8 lines)

### Unwired-proof (before deleting)

- **The build uses OTHER copies.** `games/sonic4/main.asm:156-157` includes
  `data/generated/ojz/act1/entity_data.asm` and
  `data/levels/ojz/act1/act_descriptor.asm` — the generated/levels copies, NOT
  the editor export copies. `tools/seed-worktree.sh` likewise references the
  generated copy.
- **Every symbol these three files define is externally unreferenced.** The 47
  labels/equates they define (`ojz_act1_Descriptor`, `ojz_Sec{0..8}[_Objects/
  _Rings/_TypeTable]`, `ojz_SEC{0..8}_VRAM`, `ojz_act1_Sections`) were swept
  across the whole aeon tree (`.asm`/`.emp`/`.inc`) EXCLUDING the export dir:
  ZERO external hits. (Distinct names from the built copies' `OJZ_*` uppercase
  symbols.)
- **No sigil test or tool includes the export path.** The only matches for
  `editor/ojz/act1/export` in sigil are prose in design/handoff notes, not
  build includes.

### Identity

Deletes remove files NOT in any build → byte-neutral by construction, PROVEN by
the full six-target CRC set below.

---

## Six-target FULL-CRC identity (post-parcel)

All full-file CRC32/size, matching the CHAIN-10 anchors exactly (full identity,
not appendix-only — no anchors-only drift):

| target | CRC32 / size | proof |
|---|---|---|
| s4.bin | ff9037f2 / 412127 | direct `./build.sh` |
| s4.debug.bin | 06680f0b / 421958 | direct `DEBUG=1 ./build.sh` |
| demo.bin | 9bb8c993 / 90506 | direct `./build.sh demo` (org-$3FE arm) |
| demo.debug.bin | bc7678d0 / 93006 | direct `DEBUG=1 ./build.sh demo` (org-$3FE arm) |
| config_a | 2485eab3 / 422297 | golden gate `native_offcanonical_*`, post-delete |
| config_b | d6d23298 / 303501 | golden gate `native_offcanonical_*`, post-delete |

## Gates (failures-first)

- **Strict:** `SIGIL_STRICT_GATE=1 AEON_DIR=<aeon> cargo test --workspace` →
  **2874 passed / 0 failed / 4 ignored** (HEAD baseline 2870 + 4 new embed
  tests). Zero failures.
- **Clippy:** `cargo clippy -p sigil-frontend-emp -p sigil-cli --tests` — no new
  warnings in the three edited files (pre-existing workspace warnings untouched).

## step-3 (retrospect) / step-5 (engine)

- **step-3:** the `embed` spec's `_` inference and `[embed.length-mismatch]`
  asks are both subsumed by existing, more-general `.emp` machinery (omit-to-
  infer; `[emit.size-mismatch]`). Recorded as spec deviations rather than new
  code — the language stays smaller. The one real gap (an `.emp` org/hole
  surface) is the true #12 blocker, ledgered.
- **step-5:** none — no engine bytes changed (boot_data STOPPED, J is dead-file
  removal). The port did NOT invent a native two-module boot_data split under
  budget/identity pressure; that is the correct non-hack call.

## Retirements / re-homes

- #25/#26/#27 retired (deleted). #12 remains AS-residual with an UPDATED blocker
  (org/hole surface, not binary-embed).

## Kill-list / gap-ledger

- Kill-list: no new twin scaffolding introduced (boot_data.asm is pre-existing
  AS-residual, already census-tracked; not a new mirror).
- Gap-ledger: `embed` `_`-inference spelling (nice-to-have) + an `.emp`
  absolute-org / inter-module reserved-hole surface (the #12 prerequisite)
  jotted.
