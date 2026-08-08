# The `inout` facet — spec (overseer-authored 2026-08-08)

The text of this section is the overseer's spec, landed verbatim. Measured
amendments from the implementing lane follow in their own flagged section below.

---

SEMANTICS. inout(rN) / inout(rN: T) on a proc declares: the caller provides a meaningful value of type T in rN at entry, and at every normal exit rN holds a meaningful value of T. The exit value need not equal the entry value (that would be preserves), and the proc is not obligated to write rN at all — pass-through is contract-valid. That is the semantic difference from out, and it must be asserted positively in tests.

VERIFICATION: three-valued per-path lattice, per declared register. Values Entry | Produced | Broken, seeded Entry at proc entry. Transfer rules per instruction along every path:
1. Full-width write (width >= declared; RMW at full declared width counts — add.w d0,d5 against u16 is a full-width write; adda/lea/movea.l into a declared .l-width aN is full-width) => Produced, from ANY prior state including Broken (re-production repairs; only the exit value matters).
2. Partial write (narrower than declared — the addq.b #1,d5-against-u16 exhibit) => Broken, from ANY prior state including Produced. This is the rule that kills the vacuous form the campaign proved would verify InsertSpriteMasks' old addq.b: entry bytes must never blend with written bytes.
3. Call to callee with unconditional out(rN) => Produced.
4. Call to callee with out(rN if cc) => Broken (conditional production is not production; fail-safe over-fire).
5. Call to callee with rN in clobbers (no unconditional out) => Broken.
6. Call to callee with inout(rN) => state UNCHANGED — neither produces nor breaks. Entry stays Entry, Produced stays Produced. This is the composition rule; TileDelta_Undo is the corpus exhibit that forces it (an inout proc passing its live value through an inout helper with no local write must verify). Verify TileDelta_Undo's actual shape in the corpus and cite it in the notes file; if its shape differs from this description, that is a measured amendment, not a license to change the rule.
7. Callee with preserves(rN) / callee not touching rN => unchanged.
Join at merge points: Broken absorbs; Entry-join-Produced may be represented as either (operationally the lattice collapses to Broken/not-Broken because every transfer treats Entry and Produced identically — keep the three names for diagnostics; the implementation may exploit the collapse).
Exit check: at every normal exit (rts + the falls_into boundary per existing out_verify rules), state != Broken. Entry and Produced both pass. @noreturn/exit_diverges paths exempt via the same machinery out_verify uses.

CLOBBERS/PRESERVES RULING. inout(rN) is a third disposition, mutually exclusive with both: inout(rN) + rN-in-clobbers = contract ERROR (clobbers says caller may not rely on the exit value; inout says the opposite — unlike out(rN if cc), inout implies NOTHING about clobbers membership, it forbids it). inout(rN) + rN-in-preserves = contract ERROR (preserves promises exit==entry; inout deliberately does not). Caller-side: a call to an inout(rN) callee is a value-changing use — it breaks the CALLER's preserves(rN) unless saved/restored; for the caller's own out/inout lattice it is state-preserving per rule 6.

WHY PARAM SEEDING IS EARNED BACK (the kill rule). out_verify Finding 2 rejected seeding params because entry garbage could masquerade as production. inout re-admits the entry value ONLY because the facet makes it an explicit caller obligation. What keeps it honest — three MANDATORY probes: (a) an inout(d5: u16) proc whose only write is addq.b #1,d5 must FAIL (partial-write-breaks); (b) a proc calling a callee that clobbers rN must FAIL; (c) the non-vacuity pair proving inout does not share out's path — a no-write pass-through proc PASSES under inout(rN) and the SAME proc FAILS under out(rN).

SCOPE GUARDS. No inout(rN if cc) — reject at parse if cheap, else document as unspecified. 68k only tonight — Z80 inout out of scope. Function-pointer types: if any fn-ptr type carries contracts for these procs, same grep-the-contract-text bar as any declaration change.

---

## Measured amendments (lane-inout, 2026-08-08)

Every amendment below is a fact measured against the corpus/implementation, flagged
as the spec instructed.

**A1 — TileDelta_Undo is NOT an inout proc; rule 6 has no corpus exhibit.**
Measured: `engine/compression/s4lz.emp:232` declares
`proc TileDelta_Undo (a0: *u8, d0: u16) clobbers(d0-d1/a0-a1)` — no `inout`, no
`out`. There is no inout-through-inout composition anywhere in the corpus (grep
`inout` over aeon `engine/` + `games/` returns nothing but this parcel's two
adoptions, and neither calls an inout callee). Rule 6 is therefore validated
SYNTHETICALLY, in `tests/inout_verify.rs::rule6_inout_callee_is_state_preserving`
(pass-through through an inout helper PASSES; an inout helper does NOT repair a prior
break). The rule is implemented exactly as written; only its cited exhibit differs.

**A2 — the input side is discharged by a param requirement, not a new read-side
dataflow.** `inout(rN)` requires rN to be a declared param
(`[proc.inout-not-param]`, ERROR). This routes the "caller provides rN at entry"
obligation through the EXISTING param → D1b machinery (a param already forces a
reaching definition at every call site), so no `calls.rs` change was needed. The new
work is purely the exit side (`verify_inout`).

**A3 — caller-side crediting folds inout into the out maps.** An inout register is
written/produced by the callee exactly as an out is, so it is folded into
`ProcNode.out` (→ closure effective clobbers, D1c, §6-declared) and into
`callee_uncond_out`. It is then SUBTRACTED from the `check_out` obligation (checked
by `verify_inout` instead) and, once verified, UNIONED into the credit map D1b /
out-verify credit read. Net effect: moving `d5`/`a4` from `out` (parcel A) to
`inout` is caller-side-invisible — zero D1b/D1c/§6/preserves change — which is why
the parcel is byte-neutral and the strict suite is otherwise untouched.

**A4 — two rules added beyond the spec's two.** Besides `inout∩clobbers` and
`inout∩preserves`, the implementation also errors on `inout∩out`
(`[proc.inout-out-overlap]` — one register, one exit disposition) and rejects
`inout` on a Z80 proc (`[proc.inout-z80-unsupported]`, per the 68k-only scope
guard). Conditional/flag forms inside `inout(...)` are rejected at parse.

**A5 — unknown / indirect callee ⇒ Broken.** The spec enumerates rules 3–7 for
KNOWN callees; an indirect call or a callee with no contract in the corpus is
treated as Broken for every tracked register (fail-safe, matching `out_verify`'s
"unknown target credits nothing"). A ⊤ effective effect expands to the whole
register file, so it clobbers every tracked register.

**A6 — width via `collect_inout_widths`.** `inout(rN: T)` charges the exit check at
`sizeof(T)` through a dedicated width map (the `collect_out_widths` analogue), so
`inout(d5: u16)` verifies `addq.w` and breaks `addq.b`. An address register is
pinned to `.l` whatever its type; `inout(a4: *u8)` resolves to `.l` (a pointer),
raising no unresolvable-type firing.

**A7 — the residue moved, measured as a SET DIFF.** Four rows retired from
`OUT_UNVERIFIED_BASELINE` — `("DrawRings","a4"|"d5")` and
`("InsertSpriteMasks","a4"|"d5")`. After the rebase onto lane-cfg (which retired
the eight `Collision_Probe` rows) the merged `OUT_UNVERIFIED_BASELINE` is EMPTY —
every declared 68k register out verifies. The new `INOUT_UNVERIFIED_BASELINE` is
also EMPTY (both procs verify). `Z80_OUT_UNVERIFIED_BASELINE` (6 rows, lane-z80's)
is untouched. The typed-out-slot pin is `28` (lane-cfg's 30 typed Collision outs
minus the two `d5` slots that became inout). All measured across every one of the
seven shipped shapes.

**A8 — the inout-credit LAUNDERING probe (soundness of the fold).** The design
credits a callee's verified `inout(rN)` as production for a CALLER's `out(rN)`
(folded into the verified-credit union `verify_out` reads). Measured that this is
SOUND: a laundering `proc P () out(d5)` whose only touch of `d5` is `jbsr Q` (with
`Q inout(d5)`), where P never defines `d5`, is CAUGHT by D1b
`[call.input-undefined]` on `d5` at the call — because `inout` forces `d5` to be a
param of Q (`[proc.inout-not-param]`) and D1b charges every call site to define a
callee's params. The measurement also confirms `verify_out` ALONE was fooled (P's
`out(d5)` does NOT appear in the out residue), so D1b is the load-bearing catch, not
a coincidental double-cover; and the honest counter-case (`P2` that does
`moveq #0, d5` before the call) passes with D1b silent. The catch lives in BOTH the
CI gate (`corpus_input_undefined_is_empty_the_error_gate`) and the build gate
(`run_contract_gate`'s `[call.input-undefined]` empty_gate) — same `input_firings`.
Witness: `tests/corpus_contracts.rs::inout_credit_laundering_is_caught_by_d1b`.
