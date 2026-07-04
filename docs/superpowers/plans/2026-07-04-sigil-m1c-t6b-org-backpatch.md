# Sigil M1.C — T6b: `org` Back-Patch Support Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. TDD, asl-diff-gated.
> HIGH-LATITUDE: touches `sigil-ir` (new fragment), `sigil-link` (flatten), and the front-end.
> CI runs `cargo clippy --workspace --all-targets -- -D warnings`.

**Goal:** Implement AS `org` — both the within-section **back-patch** (`org pscStart / dc.b
count / org pscEndPos`, used by 11+ parallax scenes via `parallax_section_end`) and the two
absolute `org`s in `main.asm` — byte-exact vs asl. The spec anticipates a `Fragment::Org`
(§7.1, SIGIL_CORE_SPEC:218 `Org { target: Expr, fill: u8, span }`).

## The 4 sites (verified)
- `games/sonic4/main.asm:25` `org 0`, `:141` `org $10000` — **absolute** origin/positioning.
- `engine/parallax_macros.inc:195/197` `org pscStart` … `org pscEndPos` — **back-patch**:
  reserve a placeholder byte at `pscStart`, emit the section body (counting bands), then `org`
  back to overwrite the count into the placeholder, then `org` forward to `pscEndPos` to resume.

## Verified asl back-patch semantics (probe to reconfirm)
`dc.b 0,1,2,3 / org HdrPos / dc.b 99 / org EndPos / dc.b 4` → `63 01 02 03 04` (the byte at
`HdrPos` is overwritten in place; emission resumes at `EndPos`). Re-probe with real asl to pin:
what fills a gap when `org` jumps FORWARD past the current max (fill byte — expect `0x00`)? Does
`org` to an offset ≥ current extent leave a zero gap? Does the final section length = max offset
ever written?

## Design (Org fragment + flatten replay)
Keep the front-end append-only:
- **`directive_org`**: resolve `target` to an offset (via the multi-pass env — `pscStart` etc.
  are resolved symbols by convergence); **set the IrBuilder cursor** to that offset (so labels
  after `org` get correct offsets); **emit `Fragment::Org { target, fill, span }`** as a marker.
- **IrBuilder**: add a `seek(offset)` that sets `cursor` (may move backward or forward); push the
  `Org` fragment. `current_offset()` returns the (possibly seeked) cursor.
- **sigil-link `flatten`** (and `flatten_checked`/`resolve_layout` length accounting): process
  fragments in order into a byte buffer with a WRITE CURSOR; `Org` sets the cursor to its
  resolved target; `Data`/`Fill` write at the cursor (OVERWRITING existing bytes, advancing);
  section length = highest offset written; unwritten gaps = `fill` (0x00). Confirm `resolve_layout`'s
  offset/label-shift math still holds when an `Org` is present (back-patch sections have no
  `JmpJsrSym`, per the parallax data/code split — verify and, if so, it's orthogonal).

## Absolute `org` (main.asm) — INVESTIGATE FIRST
`org 0` / `org $10000` may be section/region positioning (interacting with the memory map
`sigil.map.toml`) rather than an in-section seek. **Probe asl + check how M0/M1.B handles section
origins (phase vs org vs the map):** does `org $10000` start a new region, or seek forward within
one section (leaving a $0000-$10000 zero gap in the image)? If it maps onto existing phase/section
/map machinery, handle it there and keep `Fragment::Org` for the back-patch only. **If the
absolute-org semantics require memory-map changes, STOP and report** — don't expand scope silently.

## Steps (TDD)
- [ ] **Step 1 — probe asl** for the back-patch fill/length semantics + absolute-org behavior;
  write down the exact rules.
- [ ] **Step 2 — back-patch snippet first** (`cpu 68000`): the `dc.b 0,1,2,3 / org Hdr / dc.b 99
  / org End / dc.b 4`-style pattern with labels, → `63 01 02 03 04` (regen via `gen_snippet_vectors`).
  Also a minimal `parallax_section_end`-shaped snippet if feasible. Commit goldens.
- [ ] **Step 3 — gate fails** (`org` unrecognized).
- [ ] **Step 4 — implement** `Fragment::Org` + `IrBuilder::seek` + `directive_org` + `flatten`
  Org-replay (overwrite + max-extent length + gap fill). Add the `Org` arm to every exhaustive
  `Fragment` match (link, relax, emit_rom, emit_listing — the build will flag them). Unit-test the
  flatten overwrite + a forward-gap fill.
- [ ] **Step 5 — absolute `org`** per Step-1 findings (or escalate if it needs map changes).
- [ ] **Step 6 — gate green + suite.** asl_snippets PASS; `cargo test --workspace` PASS (esp.
  the M1.B linker/emit_rom/resolve_layout tests — the new `Org` arm must not perturb them).
- [ ] **Step 7 — `clippy --workspace --all-targets -- -D warnings` + build clean.**
- [ ] **Step 8 — commit** `feat(sigil): org back-patch (Fragment::Org + flatten replay) + absolute org (asl-gated)`.

## Self-Review
- Spec coverage: back-patch (Org fragment) + absolute org, snippet-gated. Matches the spec's
  anticipated `Fragment::Org`.
- Honest gate: goldens from real asl; the byte-overwrite is proven by a golden where the patched
  byte differs from the placeholder.
- Escalate if: absolute `org` needs memory-map changes, or `Org` + `resolve_layout` interact
  (a section with both a back-patch AND a `JmpJsrSym`).
