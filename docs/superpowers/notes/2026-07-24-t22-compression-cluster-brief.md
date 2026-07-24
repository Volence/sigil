# TRANCHE 22 BRIEF — compression cluster conversion (LEAN tranche)

**Dispatch: overseer-cut 2026-07-24 (Volence present; t21 merged and swept).
Single-lane.** Fourth tranche under the corrected LEAN amendment.

**Why this cluster and not boot:** boot.asm's `BootData` table assembles the
Z80 sound-driver SOURCE inline mid-table — the sequential `(a5)+` walk runs
through the blob, so the boot port needs a dedicated step-0 seam ruling
(embed-.bin-twin vs chained-section split vs .asm data tail) before any code.
That ruling is t23's opening work; this tranche clears boot's last non-.emp
callee (CompressionSelfTest) so t23's boot is a pure-callee port plus the
one seam question.

**Canonical sources (read before cutting code):**
`docs/superpowers/notes/campaign-port-loop.md` (re-read at EVERY step
boundary), the t21 close packet (`notes/2026-07-24-t21-close-packet.md` —
freshest template: probe-first step 0, twin-parity gate class, panel
adjudication shape), campaign gap-ledger + kill-list for the step-0 sweep.

## Scope (port ORDER MATTERS)

1. **`engine/compression/s4lz_decompress.asm`** (218 L; `S4LZ_DecompressDict`
   + `S4LZ_Decompress` shared-body fall-through entries + internal
   `TileDelta_Undo`) → `s4lz_decompress.emp` — FIRST.
2. **`engine/compression/zx0_decompress.asm`** (112 L; `ZX0_Decompress`)
   → `zx0_decompress.emp` — SECOND (independent; order with file 1 is
   convenience, not dependency).
3. **`engine/debug/compression_selftest.asm`** (94 L; `CompressionSelfTest`,
   whole-file `ifdef __DEBUG__`) → `compression_selftest.emp` — LAST (it
   calls `S4LZ_DecompressDict` + `Art_Decompress`; porting last makes both
   normal .emp→.emp imports).

Full loop `0 → 1 → 2 → (3 → 4 → 5)* → 6 → merge`; steps 0/1/2 per file;
one dry-panel; step 6 once; one close packet.

## Mechanics (standing bars — t21 values)

- Branch `port-tranche22` both repos, worktrees `.worktrees/port-tranche22`;
  **seed aeon worktree's `games/sonic4/data/editor/` by rsync from main and
  verify canonical CRCs before any code**.
- **Canonical: plain `4745cbc3`/421157 · debug `0b7c4804`/429202** (masters
  aeon `0a17462` / sigil `4a42ed8`). Strict baseline **2553/0** paired,
  AEON_DIR at the branch tree. One shape per build invocation.
- cwd resets every Bash call — cd explicitly; explicit-path commits only;
  never push; failures-first test output; oracle MCP is overseer-only —
  named probe lists for anything needing live measurement.

## THE HEADLINE OBLIGATION — three ownership flips (kill rows 30/38/39 DIE)

All three compression extern decls die this tranche, each with its
persisted two-module link test (t15/t20/t21 template), decl deleted
same-commit, kill row updated same-commit:

- **`S4LZ_Decompress (a0, a1) clobbers(d0-d3/a2-a3) out(a0, a1)`** under
  **load_art.emp:22** (row 38).
- **`ZX0_Decompress (a0, a1) clobbers(d0-d1) out(a0, a1)`** under
  **load_art.emp:24** (row 39; a2/d2 movem-saved inside — the .emp contract
  says what the .asm PROVES, and the movem gives it enforcement teeth).
- **`S4LZ_DecompressDict (a0, a1, a4: *DictBase, d4) clobbers(d0-d3/a0/a2-a4)
  out(a1)`** under **tile_cache.emp:18** (row 30 — NOTE the row's history:
  the spec §3 draft UNDER-read the .asm's clobbers; the decl follows the
  .asm to the letter. The .emp proc signature carries the decl's contract
  EXACTLY; any tightening the movem structure proves is a step-3 surfaced
  finding, never silent).

## Step-0 hazard pre-sweep (overseer findings — verify and complete;
const-keyed trip-check owed, and B1-t21's lesson applies: sweep ALL .emp
files for local mirrors, not just the shared-const homes)

**s4lz_decompress.asm**
- **Entry-to-entry fall-through**: `S4LZ_DecompressDict` falls through into
  `S4LZ_Decompress` — TWO PUB procs sharing a body by fall-through. This is
  NOT t21's P6 (bsr to local tail); nearest precedent is t20's
  `export .transfer:` Owner.label class. Design the .emp spelling in the
  step-0 note BEFORE code; probe at the real binding class if the spelling
  is unproven (demanded-feature path if it fails).
- **`ifdebug` per-line gating** (dict-length even assert, v3 version-byte
  assert) — statement-level `if DEBUG == 1` in .emp; shape-dependent bytes
  MID-proc. The byte gate binds both shapes; twin keeps `ifdebug`.
- **`assert.w`/`assert.b` macros** → .emp `assert` construct (t19 shipped;
  byte-matched the twin macro first try then).
- **The rebase arithmetic** (`adda.w d4, a4` / `suba.l a1, a4` =
  dict_end − dest_start over the full address space) and the LZ window/
  overlap-copy loops are C2-lens material — port faithfully, no cleverness.
- `TileDelta_Undo` internal-only (zero external callers — verified); stays
  non-pub.
- Type layer: `a4: *DictBase` rides the decl's spelling; d4 dict length /
  d3 uncompressed size are LOG candidates (A4-i-gated).

**zx0_decompress.asm**
- Self-contained single proc; a2/d2 movem-saved (proven-preserves upgrade
  candidate — t20 load_art precedent — adjudicate at step 3, surface not
  silent).
- Bit-stream state machine: C2 material (carry-threading through the
  bit-buffer — the CCR-adjacency census line runs per the checklist).

**compression_selftest.asm**
- **Whole-file `__DEBUG__` → the region exists ONLY in the debug shape.**
  t19/t21 shipped shape-DEPENDENT region lengths; a shape-ABSENT region may
  be new machinery (repin.toml/pins/engine.inc gate semantics for a region
  with no plain-shape counterpart). Step-0 probe/design item; if the
  harness needs a feature, TDD per the demanded-feature law.
- Callees after files 1-2: `S4LZ_DecompressDict` (.emp), `Art_Decompress`
  (.emp since t20) — normal imports, zero externs. `assert.w d0, eq,
  #CSELF_PAYLOAD_SUM` → .emp assert.
- The golden-vector DATA (`CSelf_*` blobs, `tools/gen_compression_vectors.py`
  emits them + `CSELF_PAYLOAD_SUM`/`CSELF_DICT_LEN`) stays generated —
  locate its emission site; the port is code-only. If the generator emits
  .asm only, the .emp side consumes the symbols cross-seam (equ/extern
  class) — name the spelling in the step-0 note.
- The poison-between-vectors discipline and the byte_compare tail are
  load-bearing test semantics — the comments are contract, keep
  present-tense.

## Step-5 / panel notes

- **Heat**: `S4LZ_DecompressDict` is ON the tile_cache streaming path
  (ledger rows 1057/1064/1066: DecompressBlock incl ~3.4k cyc/f warm,
  cold-crossing spikes; the t16 ruling was decompress is NOT the lever —
  prefetch amortizes). `S4LZ_Decompress`/`ZX0_Decompress` are load-time
  (Level_LoadArt path); selftest is boot-cold debug-only. **NO behavior or
  performance re-work of the streaming interplay in this port** — the
  unified-prefetch charter owns it; step-5 runs the interrogation lines and
  expects measured no-cut. If any candidate ≥~1k cyc/f appears, it comes to
  the overseer as a named probe first.
- Panel: A1+B1+C1+C2+**C3 active** (the streaming-path interplay + the
  VBlank-window context of Tile_Cache_Fill are C3's named inputs; C2 gets
  the LZ window/rebase/bit-stream arithmetic as its named input; C1 gets
  the t16 cycle numbers to check the port's comments against).
- Step-6 candidates: whatever the fall-through-entry .emp spelling becomes
  (dma_queue's export-label tails are the analog); proven-preserves
  upgrades ripple to the flip-side decls' documentation.

## Acceptance

Per-file step-1 gate lists with named artifacts (byte gates both shapes —
selftest's debug-only region per its resolved machinery; region pins;
mixed-build acceptance; negative probes; gate-off CRCs; THREE
ownership-flip link tests); full paired strict green from the branch tree
at every byte-changing commit; dry = full 3→4→5 circuit empty then a clean
panel round; step-6 enumeration; close packet per house format; ledger/kill
rows same-commit (rows 30/38/39 killed; any new extern rows born). STOP at
the merge gate — the overseer countersigns (fresh strict, dual rebuild,
hot-path second look on s4lz_decompress.emp) and runs the merge ceremony +
PROVENANCE re-baseline. Checkpoint discipline (a)/(b)/(c): STOP after steps
0-2 with the raw-data report before entering the loop.
