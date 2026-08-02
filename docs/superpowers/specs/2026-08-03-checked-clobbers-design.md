# S2-D6 · The checked-clobbers lint — full mechanical closure (design)

**Provenance:** the lens sweep's structural headline (adjudication packet §4 S-1;
C2-seat-1's noticing clause). C2's corpus audit found NO live gate-blind bug but
named the dominant remaining gate-blind surface: **prose-only reliance on a
callee's actual register writes being a subset of its declared
clobbers/preserves** — the "LOAD-BEARING & INVISIBLE" a5/a6 hoist across
TileCache_DecompressBlock→S4LZ_DecompressDict (collision-plane corruption if ever
violated), the exhaustive-license preserves under children/dplc (AllocDynamic a0,
Sound_PlaySFX d1/a0, QueueDMA_* a3), the splice-template prose contracts. The
byte gate structurally cannot see this class; one mechanical check closes it.

**What exists (verified in source, lower/proc.rs):** a HEURISTIC
`[proc.clobber-undeclared]` warning already ships: per-instruction write
detection via `writes_dest_register` (a hand-maintained mnemonic string list) +
`instr_written_regs` (last-operand dest, `(An)+`/`-(An)` side effects, the
dbcc-family counter), with sp-discipline exemption. Its own comments declare the
deferral ("full register dataflow is deferred to S2-D6") and the load-bearing
hole ("a newly-supported write-form (`bchg`, `roxl`, …) will silently escape the
lint until added HERE"). `check_out`/`[proc.out-unwritten]` share the detector.

## 1 · The four upgrades (v1 scope, 68k only)

**U1 — ISA-derived write model (kill the parallel string list).** Move the
per-mnemonic write effects INTO the ISA `Mnemonic` table (or an exhaustive
`match` over it that fails to compile when a variant is added): each mnemonic
declares which operands it writes + its side effects. Covers today's escapees
by construction: `movem` (register-list loads), `exg` (both), `link`/`unlk`
(An + sp), `bchg`, `roxl`/`roxr`, `negx`, `abcd`/`sbcd`/`nbcd`, `movep`.
Deliverable includes a census diff: the heuristic's write-set vs the ISA
model's, over the whole corpus — every newly-detected write is either a real
latent finding or a model bug; adjudicate each.

**U2 — call-edge propagation (the actual sweep demand).** At every direct call
(`jsr`/`jbsr`/`bsr` to a resolvable proc symbol, and `invoke` through the game
contract), union the CALLEE's declared write surface (clobbers ∪ outs ∪
non-preserved params) into the caller's computed write set. Soundness is
edge-by-edge: the callee's own declaration is verified by ITS lint run, so the
graph needs no global fixpoint — each proc is checked against its body plus its
callees' CONTRACTS. Typed computed calls (`jsr (aN,dN.w) as StateType`) use the
TYPE's contract (this is what the typed-dispatch machinery exists for).
Untyped computed targets = unanalyzable: hard diagnostic unless annotated (U4).
Splices (comptime-fn Code) are already inline post-splice — analyzed as body
text, no special casing; their prose contracts become checked facts for free.

**U3 — escalation: warning → strict-gated error.** Rollout: (a) land U1+U2
warning-level; (b) run the corpus census, fix or annotate every hit (C2's audit
predicts near-zero true violations — the prose contracts were found honest
TODAY; the lint keeps them honest TOMORROW); (c) flip `[proc.clobber-undeclared]`
to Error under SIGIL_STRICT_GATE (a plain build keeps the warning, adoption
taste). The flip commit is the closure moment for the C2 class.

**U4 — the annotated escape hatch, refuse-loud.** For genuinely unanalyzable
sites: `@allow("clobbers.unanalyzable", "<reason>")` at the proc — reason string
REQUIRED (mirrors the module-scope @allow machinery, one rule). The lint report
lists every annotation so the surface stays audited, never silent. Expected
census: single digits (raw computed jumps outside the typed-dispatch idiom).

## 2 · Explicitly out of scope (v1)

- **CC/SR as a tracked resource** — the CC-clobber-between-test-and-Bcc class is
  C2 checklist #2 (audited clean); modeling CC liveness is a different, larger
  analysis. Ledger a row; do not let it bloat this parcel.
- **Z80 side** — different ISA table, different discipline (the sound stack is
  timing-locked and contract-audited); a z80 variant waits for demand.
- **Memory effects** (writes through pointers) — register contracts only.
- **preserves-completeness inversion** ("declared clobber never actually
  written") — a tidiness lint, not a soundness one; note as follow-up, the
  census output makes it nearly free later.

## 3 · Acceptance bars (implementation parcel)

1. U1 census diff over the corpus, every delta adjudicated in the report.
2. Unit tests: each newly-modeled mnemonic (movem both directions, exg, link,
   the bit/rotate family) + call-edge propagation (direct, jbsr, typed-dispatch,
   contract invoke) + the unanalyzable diagnostic + @allow pass-through.
3. The three named prose contracts become CHECKED: a doctored
   S4LZ_DecompressDict that writes a5 must FAIL the caller's lint (negative
   probe — the sweep's load-bearing example, proven fireable).
4. Corpus green at error level under strict; byte-neutral ×6 (lint-only parcel;
   any fix a census hit forces is its own adjudicated commit).
5. `check_out` keeps sharing the detector — no drift between the two lints.

## 4 · Sequencing

Per ruling R-C: this spec rides in parallel with the sweep parcels (now all
landed); the implementation parcel is next in the porter queue, BEFORE the
round-2 dry panel (the panel then sees the closed class) or in parallel with it
(the panel is read-only) — overseer's call at dispatch. A1/A2 after.
