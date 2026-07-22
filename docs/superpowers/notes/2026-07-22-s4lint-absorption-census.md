# s4lint → sigil absorption census (2026-07-22, overseer)

**Trigger:** Volence — "there was a lot of linting we added to ./build.sh for the old
assembler; did we want to incorporate that into our assembler?" Answer: yes, and this is
the mapping. `aeon/tools/s4lint.py` (2,386 lines, 11 E + 24 W codes) runs on EVERY build
(`build.sh:87`, `--no-lint` to skip) and is currently clean over the corpus. It exists
because the old assembler couldn't know these things; sigil sees full IR + CFG + typed
contracts, so each check should end as one of: **impossible-by-construction** (language
design — best), **sigil-native diagnostic**, or **obsolete**. As ports retire `.asm`,
s4lint's surface shrinks toward zero; this census is the plan for what replaces it.

Correction for the record: the s2d6 Stage-0 phrase "s4lint W021 → moot (not a live tool)"
was sloppy — s4lint is live; W021 specifically defers to the `.emp` contract system for
ported routines (the tool's own comment says so). The adjudication was right, the wording
wasn't.

## Tier A — already absorbed / superseded by sigil (nothing to do; retire with the .asm)

| Code | Check | Superseded by |
|---|---|---|
| W021 | writes outside declared Clobbers | The contract system (D1b/closure/§5) — strictly stronger: VERIFIED, not header-comment prose. |
| E008 | macro contract violation | `.emp` proc contracts (params/clobbers/out/preserves). |
| E009 | SST field out of bounds | Typed SST field access (corpus type environment) — compile error. |
| E001 / W005 | unsized branch / branch should use .s | `jbra`/`jbsr` auto-reaching — sigil picks widths; the class cannot occur. |
| W012 | move.l to areg vs movea.l | Lowering emits the correct form; invalid forms are errors. |
| E003/E004/E005 | odd address / word access to odd / missing align after byte data | Linker layout parity (D2.29 `[layout.odd-item]` asserts). CONFIRM E005's byte-data-then-code shape is fully covered at the next touch. |

## Tier B — genuine sigil-native candidates (the absorb list, ranked)

1. **Z80-bus machine-state contract — E006 (VDP write without Z80 stopped), E007
   (unpaired stopZ80/startZ80), E011 (double stopZ80).** The old S2-D7 "machine-state
   lints" plan, now buildable on the shared `flag_check::Cfg`: model bus-state as a
   tracked flag through the CFG exactly like the cc-lattice; a VDP-port write on a path
   where the state isn't provably STOPPED fires. Highest value — this is a **crash
   class** (the hardware-constraints tier), and the CFG machinery it needs already
   exists and is battle-tested.
2. **W026 — byte-loaded register used at word/long width without extension.** The
   width-class discipline the out-verifier's Finding-1 formalized, generalized to a
   dataflow lint (track per-register "defined width" forward; a wider read without
   `ext`/full-width redefine fires). Natural extension of the existing width model;
   pairs with the G5 width-typed-outs work.
3. **E010 — SR save/restore mismatch.** §5 preserves already tracks sp-discipline and
   `preserves(sr)` exists in the grammar — confirm coverage, close any gap in §5 rather
   than porting the regex check.
4. **W023 (ifdebug CCR setup consumed by release-side conditional) + W024 (debug-only
   macro outside __DEBUG__ gate).** Debug/release seam checks — sigil owns comptime
   `if debug` and the cc-lattice, so both are precise here where s4lint is heuristic.
5. **Perf-analysis tier — E002/W014 (mul-div in/near hot paths), W010 (indexed
   addressing in loop), W022 (loop-invariant memory operand in dbf loop).** Sigil has
   real loop structure; these become a `perf-lint` pass. WARN-tier only.
6. **Peephole hints — W001 (clr on memory = RMW), W002 (cmp #0→tst), W003/W013
   (moveq range), W004 (addq/subq range), W007 (lsl#1→add), W008 (sub dn,dn), W011
   (movem single reg), W020 (bsr+rts tail).** IMPORTANT POLICY FENCE: these stay
   **WARN-tier lints, never silent transforms** — sigil's byte-identity culture requires
   predictable output; the assembler must never "helpfully" change emitted bytes. (The
   `.emp` port loop's step-2 modernization is where the human applies them.)

## Tier C — style/convention (low priority, public-release polish)

W006/W018/W019 (header comments, routine length), W015/W016/W017 (naming case). `.emp`
has its own conventions (doc-comments, brace-indent); if ported at all, they're a
configurable style pass for the public release, not campaign work.

## Sequencing recommendation

- **Not pass-3 work.** Nothing here blocks Phase 2; s4lint keeps guarding the `.asm`
  remainder exactly as today.
- Tier B items 1–4 are **language-solidification** work — the natural slot is the
  "Diagnostics-tier remainder" backlog row (roadmap §D), promoted to a real item after
  pass-3, or alongside G5 (item 2 explicitly pairs with G5's width typing).
- End-state worth stating: when the last `.asm` ports, `build.sh` drops the s4lint call
  and every surviving check is a sigil diagnostic — one tool, one truth (the same
  three-surfaces-agree principle the s2d6/flip arc enforced).
