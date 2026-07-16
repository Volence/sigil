# Tranche 17 — `plane_buffer.emp` STEP-1 checkpoint (→ Fable gate, before step 2)

**Branches:** `port-tranche17` both repos (aeon `90de072`, sigil `17eaa8e`).
Not pushed (checkpoint, not merge). Paired-state strict run with `AEON_DIR`
→ `.worktrees/port-tranche17` (seeded via `tools/seed-worktree.sh`).

Step 0 was gated PASS-WITH-RIDERS; this is the step-1 transcribe + gate.
**NO demanded features — a clean transcribe** (the gated finding held).

---

## Gate-artifact list (each gate → its executed artifact)

| Gate | Artifact | Result |
|---|---|---|
| region byte gate, plain | `plane_buffer_port::plane_buffer_region_matches_reference` | PASS (byte-identical to s4.bin @ `$405E`, len `$29C`) |
| region byte gate, debug | `plane_buffer_port::plane_buffer_debug_region_matches_reference` | PASS (byte-identical to s4.debug.bin @ `$4C28`, len `$29C`) |
| 3rd ownership FLIP, plain | `plane_buffer_port::two_module_ownership_flip_plain` | PASS |
| 3rd ownership FLIP, debug | `plane_buffer_port::two_module_ownership_flip_debug` | PASS |
| negative probe | `plane_buffer_port::doctored_plane_buffer_size_fires_its_guard` | PASS (fires + NAMES `PLANE_BUFFER_SIZE`) |
| gate in `engine.inc` | `SIGIL_EMP_PLANE_BUFFER` (resume orgs `$42FA`/`$4EC4`) | present; org composes (see mixed-build) |
| region pin | `pins::PLANE_BUFFER` + `pins::PLANE_BUFFER_BASE` (repin-generated) | present |
| gate-off neutrality | full DEBUG-first + plain rebuild, gate OFF | plain **453087/b335bdc6**, debug **461110/827e18c4** (canonical, unchanged) |
| mixed-build acceptance | AS build with `SIGIL_EMP_PLANE_BUFFER` defined | org gate composes: **0 org/overlap errors**; the only 5 errors are the expected `.emp`-provided symbols (`VInt_DrawLevel`, `Draw_TileColumn`×2, `Draw_TileRow_FromCache`×2) that the sigil splice supplies — the called-leaf pattern (gap-ledger row 1049: no standalone whole-ROM mixed test) |
| paired-state strict | `SIGIL_STRICT_GATE=1 AEON_DIR=<worktree> cargo test --workspace` | **2262 / 0** (2257 baseline + 5 new) |
| repin --check | `repin_pins::pins_rs_is_current` | PASS (0 pins changed on regen) |
| kill-list rows | row 5 (`plane_buffer.asm` twin) + row 6 (`PLANE_BUFFER` pin), same commit | done |

Clippy: workspace clean of NEW warnings; the 2 pre-existing warnings are both in
`struct_field_disp_plus_n.rs` (shared-struct-module batch, untouched here) — flagged, not mine.

---

## The FLIP (§6c) — mechanism confirmation

`two_module_ownership_flip_{plain,debug}` compiles the real `plane_buffer.emp`
(placed at `PLANE_BUFFER`) **+** the real `section.emp` (placed at `SECTION`),
each with its `engine.structs` + `engine.constants` ambient, and links ONE image
over the union. section's synthetic label list **DROPS**
`Draw_TileColumn`/`Draw_TileRow_FromCache` (now plane_buffer.emp-owned — reusing
the existing `DRAW_TILE_COLUMN` `$4066`/`$4C30` + `DRAW_TILE_ROW_FROM_CACHE`
`$4188`/`$4D52` pins). section's 4 `jbsr→bsr.w` bytes byte-match the reference
ONLY when each disp lands on plane_buffer.emp's pinned symbol VMA — the flip,
proven per shape. Topology = unidirectional (section→plane_buffer; plane_buffer
is a leaf), jbsr mechanism (entity_window template) — as the note §6 predicted.
Consistency check: `Plane_Buffer_Reset` (region base `$405E`) is 8 bytes, so
plane_buffer.emp's `Draw_TileColumn` resolves to `$4066` = the `DRAW_TILE_COLUMN`
pin exactly, no fixup needed.

---

## Rulings discharged (binding, from the step-0 gate)

- **KEEP both no-caller procs** — `Draw_BG_TileColumn` (§4.2 forward-scaffolding)
  and `Plane_Buffer_Reset` both ported faithfully; byte-region includes them.
  The two PRE-REGISTERED findings are logged for their steps, NOT touched at step 1:
  (a) step-3(b): `Plane_Buffer_Reset`'s "call each frame after drain" header claim
  — carried verbatim at step 1, rewrite pending at step 3(b) comment-claim audit;
  (b) step-5 counter/cache audit: the stale-drain-across-act-transition question —
  pending at step 5.
- **Row 1052 (VDP-macro shared home) NOT claimed** — plane_buffer has zero
  `vdpComm`/`vdpCommReg`; `VInt_DrawLevel` receives precomputed command longs and
  writes raw `$8Fxx` register words + builds command longs at runtime. Trip-check honored.
- **§7 typed-VDP candidates deferred to step 3(a)/4** (the `vdp_rw_cmd` shuffle
  template + `$8Fxx` reg-word spelling) — not surfaced as step-1 work.

---

## Two spelling findings surfaced during transcription (existing-feature workarounds; no scope expansion)

Both had byte-identical workarounds using only EXISTING language features, so the
note §4 "everything already in the corpus" premise held — no park-before-proceed
was owed. Reporting for the record; both are step-3(a) material:

1. **Link-time abs base — `.l` override can't defer (row 1004, known).**
   `lea (Tile_Cache_Nametable).l` fails at lower (`unknown name`; the `.l`
   override comptime-folds, but the base is a link-time extern). Fixed by the BARE
   form `lea Tile_Cache_Nametable[+TILE_CACHE_NT_SIZE]` (note §4's listed spelling;
   width rule → abs.l, byte-identical). This is EXPECTED, not a surprise.

2. **Compound-CONST displacement `A-B(An)` misparses as a call (NEW, ledgered).**
   `lea VDP_CTRL-VDP_DATA(a6), a5` reads `VDP_DATA(a6)` as a call. Generalizes the
   item-4 `.field+N(An)` finding to plain consts (same "name-before-`(An)` = call"
   rule). Shipped workaround: a single derived `const VDP_CTRL_OFF = VDP_CTRL -
   VDP_DATA` (a single name normalizes to a displacement, parser.rs:1979) →
   `lea VDP_CTRL_OFF(a6), a5`, byte-identical `4BEE 0004`. Language ask logged
   (retires the const when the compound-disp grammar lands).

---

## One tooling change (forced, minimal)

`repin.rs` `SymbolSpec` gained an optional `const_name` override. The `Plane_Buffer`
RAM base symbol's `upper_snake` is `PLANE_BUFFER`, colliding with the `plane_buffer`
REGION const (the resolver hard-rejects const-name collisions). The base pins as
`PLANE_BUFFER_BASE` via the override. (Offsets already take their name verbatim;
this brings symbols in line.) Byte-neutral; `repin_pins` green.

---

## What each pass added (step-1 only — steps 2–6 pending)

**Step-1 demanded features:** NONE (clean transcribe — the gated finding).

**Probe outcomes:** gate-off neutrality (canonical CRCs), mixed-build org
composition (0 org errors), flip proof (per shape).

**Findings (neither step-3 nor step-5 — surfaced by the transcribe):** the two
spelling findings above (row-1004 restatement + the NEW compound-const disp gap);
the repin `const_name` addition; the row-5 kill-list enumeration lag (load_object
/entity_window/section/collision_lookup/tile_cache body-twins never appended —
flagged for a backfill sweep).

**Deferred to their steps (pre-registered):** Plane_Buffer_Reset header rewrite
(3b), act-transition stale-drain audit (5), typed-VDP asks (3a/4).

---

**Next (on Fable's gate PASS):** step 2 (modernize — bare Bcc / jbra / the bare
absolute-EA sweep for the remaining `(X).w` RAM refs / brace-indent / step-2
checklist), then steps 3–6, with step-5 charter = ledger row 1066 (Probe A on the
shipped tip BEFORE locking step-5 design, per the note §8 protocol).
