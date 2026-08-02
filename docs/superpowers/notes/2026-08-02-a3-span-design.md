# Parcel A3 — the comptime emitted-span primitive: design

The A1/A2 arc's third parcel (spec §3). Two adoption targets share ONE root
gap (gap-ledger row 1654 / 1805 / 1911): `.emp` cannot fold an emitted table's
own byte span at comptime, so a hand literal stands in where the deleted AS
`End-Start` guards measured emission automatically. This note fixes the
mechanism per target, the alternatives weighed, and why the chosen shape is the
cheapest honest one.

## The mechanism: `span(ProcName)` — a comptime emitted-byte-span builtin

A single-segment, non-shadowable comptime builtin (the `here()` / `cycles()` /
`embed()` family in `eval/call.rs`). `span(P)` names a proc `P` in the current
module and returns its EMITTED byte length as a `Value::Int`.

**How it measures.** It reuses the REAL body lowering — `eval_asm_owned` (the
same entry the actual proc emission runs through, `lower::proc::eval_proc_body`
→ the identical `lower_dc`). The resulting `CodeBuf`'s items are summed:
`CodeItem::Inline` contributes its `DataBuf::size` (the running per-cell byte
sum `stream_data` serializes), `CodeItem::Label` contributes 0. This is not a
re-derivation of dc widths — it is the actual lowering's own byte count, so the
value is the emitted span by construction.

**Why it is honest with unresolved operands.** A `dc.w Label` cell lowers to a
`Cell::SymRef` of width 2 whether or not `Label` resolves — the byte length is
structural. So `span(FmVolEnv_Ptrs)` = 6 even when the vol-env body labels are
still link-deferred (the standalone `eval_pub_consts` path the seam uses). A
`dc.b <comptime int>` cell needs its value only for range-checking, not for its
1-byte contribution.

**Pure-data restriction (scope wall).** `span` measures pure-data proc bodies
only: any `CodeItem::Instr` in the measured body is a loud error
(`[span.not-data]`) — a code proc's byte length is a relaxation/link-time fact,
out of the demand. Labels are allowed (0 bytes). This keeps the primitive
scoped to "an emitted table body" (row 1654's exact ask) and refuses to quietly
mis-answer for anything larger.

**Side-effect hygiene.** The measurement is a pure query: `self.diags` and
`self.dropped_instrs` are snapshotted and restored around the throwaway
lowering, so a malformed body is reported ONCE by the real emission, never
doubled by the measurement. `asm_counter` is allowed to advance (monotonic
label hygiene — advancing it further is harmless).

### Alternatives weighed

- **A general `section.len` / `end_label - start_label` comptime fold** (row
  1654's other phrasings). Rejected: a trailing local label is a link-time
  value (`z80_init`'s `.code_end`), so making `end - start` comptime-foldable
  means teaching the linker's placement into the evaluator — disproportionate
  surgery for a 2-file demand. `span(Proc)` reads the body the evaluator
  already lowers, no placement needed.
- **Convert the ptr tables to `offsets`/`table` constructs and use `.count`.**
  Rejected: those emit `dc.w target - Base` (forward emission) / typed cells —
  a different byte shape than the absolute `dc.w FmVolEnv_01` pointers the
  resident writers read at fixed $8000-window addresses. Changing the emitted
  bytes violates the identity bar.
- **A `Proc.span` path derived-fact** (like `Table.count`). Rejected as a
  surface choice, not a capability one: it would require a proc to be a
  first-class path receiver everywhere `eval_path` runs. A non-shadowable
  builtin is the smaller, more local addition and matches the existing
  comptime-query family.

## Target 1 — `dac_sample_tab.emp:59`

`ensure(10 * 9 == DAC_SAMPLE_COUNT * DacSample_len, …)` becomes
`ensure(span(DacSampleTable) == DAC_SAMPLE_COUNT * DacSample_len, …)`. The `10*9`
hand literal — whose own comment names this missing primitive as the blocker —
is retired for the MEASURED 90-byte emitted span. The guard now catches the
table body emitting the wrong NUMBER of descriptors, exactly what the AS
`(DacSampleTable_End - DacSampleTable) <> …` guard did and the literal could not.

## Target 2 — `FMVOLENV_COUNT` / `PSGVOLENV_COUNT`

Currently ungoverned literals in `seam1.rs::seam_emit_config` (3 / 0x0B) with no
guard. The driving data (`FmVolEnv_Ids` 3 bytes / `PsgVolEnv_Ids` 11 bytes,
`FmVolEnv_Ptrs` / `PsgVolEnv_Ptrs`) lives in the GENERATED
`sound_tables_z80.emp`. The counts become pub consts of that module, DERIVED:

```
pub const PSGVOLENV_COUNT = span(PsgVolEnv_Ids)   // db id-list, 1 B/env → count
pub const FMVOLENV_COUNT  = span(FmVolEnv_Ids)
```

Count derives from the id-list span (1 byte per env), the primary source. The
generator additionally emits the REVIVED internal guard the deleted AS twin
carried (`if (…_Ptrs_End - …_Ptrs) <> COUNT*2`), now a real compile-time ensure:

```
ensure(span(PsgVolEnv_Ptrs) == PSGVOLENV_COUNT * 2, "PsgVolEnv_Ptrs entry count mismatch vs PsgVolEnv_Ids")
ensure(span(FmVolEnv_Ptrs)  == FMVOLENV_COUNT  * 2, "FmVolEnv_Ptrs entry count mismatch vs FmVolEnv_Ids")
```

So id/ptr desync — previously only "generator-structural" (row 1879) — is again
a build error on the `.emp` itself. Because `sound_tables_z80.emp` is generated,
the pub consts + ensures are emitted by `tools/gen_sound_tables.py`
(`emit_emp_z80`) and the file regenerated; pub consts and item-position ensures
emit ZERO bytes, so the 855-byte section is byte-identical.

Seam side: a memoized `sound_tables_authority_consts(aeon)` evals the module's
pub consts; `resolve_consts` consults it (authority-first) and the two
`seam_emit_config` keys are deleted. The counts now flow from the data, not a
hand literal — drift between the resident scan count and the emitted table is
structurally impossible.

## Identity bar

Byte-identical ×6. `span(DacSampleTable)` == 90 == old `10*9`;
`span(*VolEnv_Ids)` == 3 / 11 == old literals (transition-proven by the ensures
firing green + the seam resolving to the same values). New surface gets negative
tests: a doctored-length DacSampleTable and a doctored PsgVolEnv_Ptrs must fail
their ensures loudly.
