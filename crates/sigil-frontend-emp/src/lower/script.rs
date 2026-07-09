//! Lower an [`Item::Script`](crate::ast::Item::Script) (Plan 7 #9b — D9.2,
//! D9.6, rulings R9b.1–R9b.12 in the 9b plan). A script desugars to:
//!
//! 1. a HIDDEN dispatch-encoded resume table at the script's name (member 0 =
//!    the entry segment; one member per yield), synthesized as a
//!    [`DispatchDecl`] and evaluated by the SHIPPED `eval_dispatch_with_root`
//!    (Str targets carry the resume labels' final hygienic names verbatim);
//! 2. ONE flattened proc-shaped body — `yield` becomes
//!    `move.w #<scaled ordinal>, <resume_off>(aP)` + `jbra <epilogue>` +
//!    a `__resume$<k>` label; `loop {}` becomes `__loop$<d>` + `jbra` back —
//!    lowered through the SHIPPED `eval_proc_body` + `lower_code_buf` path,
//!    so user labels work ACROSS yield boundaries (single hygiene scope).
//!
//! The resume slot (D9.3): the unique `ScriptPc`-typed field of the unique
//! `*Struct` address-register param; the stored value is the member ordinal
//! pre-scaled by the encoding (long_ptrs scripts store the ×4 ordinal WORD —
//! the slot is uniformly 2 bytes; the engine indexes the table with it).

use crate::ast::{
    self, AsmStmt, DispatchDecl, DispatchMember, DispatchTarget, Expr, InstrLine, Operand, Path,
    ScriptLabel, ScriptStmt, TextOrSplice,
};
use crate::layout::{eval_dispatch_with_root, Ty};
use crate::lower::hygiene::Owner;
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::IrBuilder;
use sigil_span::{Diagnostic, Level, Span};

/// Lower one `script` (R9b.1–R9b.12): resume-slot discovery, `yield`/`loop`
/// desugar onto proc-body statements, the hidden resume table, then the
/// flattened body through the SHIPPED proc machinery. Mirrors
/// [`lower_dispatch_item`](super::lower_dispatch_item)'s 7-arg shape (post-9a):
/// `placement`, `as_compat`, the `builder`/`diags`, and the module-wide
/// `asm_counter` threaded across every proc-shaped body for label hygiene.
pub(super) fn lower_script_item(
    file: &ast::File,
    decl: &ast::ScriptDecl,
    placement: &super::Placement,
    as_compat: bool,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
    asm_counter: &mut u32,
) {
    // 1. Guard: 68k only in v1 (mirror `[dispatch.non-68k]`), BEFORE any work —
    //    a Z80 script does not exist (R9b.9). The hidden table is emitted as a
    //    68k dispatch and the body is 68k code; refuse a Z80 section outright.
    if placement.cpu != Cpu::M68000 {
        err(
            diags,
            decl.span,
            "[script.non-68k] a script is a 68k idiom (its hidden resume table \
             is a 68k dispatch table); Z80 scripts are not supported"
                .to_string(),
        );
        return;
    }

    // 2. Resume-slot discovery (R9b.3 / D9.3): among the params, exactly ONE
    //    address-register `*S` where struct `S` has exactly ONE `ScriptPc`
    //    field. The field's offset is the store displacement; its size MUST be
    //    2 (the slot is a word). Errors return WITHOUT emission (table-less
    //    failure — mirror how `lower_dispatch_item` returns on a guard failure).
    let Some(slot) = discover_resume_slot(file, decl, diags) else {
        return;
    };

    // 3. Desugar the script body to a flat `Vec<AsmStmt>` + the yield count. A
    //    bare `yield` with no effective epilogue is `[script.no-epilogue]`
    //    (D9.6) — reported per site; we keep walking to collect every instance
    //    but emit NOTHING if any errored (simplest correct behavior).
    let mut ctx = Desugar {
        slot: &slot,
        encoding: decl.encoding,
        script_epilogue: decl.epilogue.as_ref(),
        yield_count: 0,
        loop_count: 0,
        had_no_epilogue: false,
        diags,
    };
    let mut flat = Vec::new();
    ctx.walk(&decl.body, &mut flat);
    let yield_count = ctx.yield_count;
    let had_no_epilogue = ctx.had_no_epilogue;
    if had_no_epilogue {
        return; // at least one bare yield had no epilogue — refuse the whole script.
    }

    // 4. The resume labels' FINAL hygienic names (R9b.11): compute them through
    //    the SAME `Owner` API the evaluator's `eval_asm_owned` uses, with the
    //    SAME module id (`file.module.path.segments.join(".")`) and the script
    //    name as the owner. A `__resume$k` is a non-export proc-local label, so
    //    its emitted symbol is `Owner::Proc.local_symbol("__resume$k")` — the
    //    exact string the body's label definition also renames to (one source
    //    of truth; a mismatch surfaces as an unresolved `$m$…` link symbol).
    let owner = Owner::Proc {
        module: file.module.path.segments.join("."),
        name: decl.name.clone(),
    };

    // 5. Synthesize + lower the HIDDEN resume table (R9b.2): member 0 = the
    //    entry (`__resume$0`, prepended to the body below), members 1..=n = the
    //    yield resume points. Each row targets the resume label's final name
    //    verbatim via `DispatchTarget::Label(Expr::Str(..))` — the Str arm of
    //    `eval_dispatch_with_root` passes it through unchanged. Lower it EXACTLY
    //    as `lower_dispatch_item`'s table half (eval → stream → define base
    //    label → emit).
    let members = (0..=yield_count)
        .map(|k| DispatchMember {
            name: format!("R{k}"),
            target: DispatchTarget::Label(Expr::Str(owner.local_symbol(&resume_name(k)), decl.span)),
            span: decl.span,
        })
        .collect();
    let table = DispatchDecl {
        public: decl.public,
        name: decl.name.clone(),
        encoding: decl.encoding,
        members,
        span: decl.span,
    };
    let (buf, mut ds) = eval_dispatch_with_root(file, &table, placement.include_root, placement.defines);
    diags.append(&mut ds);
    if let Some(buf) = buf {
        let (bytes, fixups, mut stream_diags) =
            super::data::stream_data(&buf, placement.cpu, decl.span);
        diags.append(&mut stream_diags);
        builder.define_label(&decl.name);
        // D2.29 amendment: a script is CODE by construction (the word table
        // shifts every resume segment odd with it), so an odd final address
        // is the error-tier [layout.odd-item], same as a proc.
        super::record_odd_item_assert(
            file,
            builder,
            placement.cpu,
            as_compat,
            super::OddItemKind::Code,
            &decl.name,
            decl.span,
        );
        builder.emit_data(&bytes, fixups, decl.span);
    }

    // 6. The flattened body: prepend the entry resume label (`__resume$0`, the
    //    top of the body = member 0), then lower it through the SHIPPED proc
    //    path (eval_proc_body reuses the param `*S` binding + label hygiene;
    //    lower_code_buf reuses the backend). Single evaluation of the whole
    //    flattened body means user labels (`.tick`) resolve ACROSS yield
    //    boundaries for free (R9b's single-hygiene-scope rationale).
    let mut body_stmts = Vec::with_capacity(flat.len() + 1);
    body_stmts.push(AsmStmt::Label { name: resume_name(0), export: false, span: decl.span });
    body_stmts.extend(flat);
    let (buf, mut ds, next_counter) = crate::eval::eval_proc_body(
        file,
        &decl.name,
        &decl.params,
        &body_stmts,
        decl.span,
        *asm_counter,
        placement.cpu,
        placement.defines,
    );
    *asm_counter = next_counter;
    diags.append(&mut ds);
    let Some(buf) = buf else { return };
    super::lower_code_buf(&buf, placement.cpu, as_compat, builder, diags);

    // 7. Fallthrough (R9b.9): a script body that reaches its closing `}` without
    //    an unconditional terminator runs into whatever follows — the
    //    proc-flavored `[proc.undeclared-fallthrough]`'s script mirror. Silenced
    //    under `@as_compat` (parity, not expectation), like every modernization
    //    lint. Uses the SHARED `ends_in_terminator` (last-mnemonic heuristic).
    if !as_compat && !super::proc::ends_in_terminator(&buf, placement.cpu) {
        diags.push(Diagnostic {
            level: Level::Warning,
            message: format!(
                "[script.fallthrough] script `{}` can reach its closing `}}` without an \
                 unconditional terminator — it will run into whatever follows it",
                decl.name
            ),
            primary: decl.span,
        });
    }
}

/// The resume slot's location: the byte offset of the `ScriptPc` field and the
/// param register spelling (`a0`…`a6`) whose pointee owns it. The desugared
/// yield store is `move.w #<ordinal>, <offset>(<reg>)` (R9b.5).
struct ResumeSlot {
    /// The `ScriptPc` field's byte offset within the pointee struct.
    offset: i64,
    /// The param register spelling the store addresses through (e.g. `a0`).
    reg: String,
}

/// Resume-slot discovery (R9b.3 / D9.3): find the UNIQUE address-register param
/// typed `*S` where struct `S` has a UNIQUE `Ty::Newtype("ScriptPc")` field of
/// width 2. The type resolution reuses `eval_proc_body`'s param-binding pattern
/// (resolve the pointee via a scratch evaluator whose diagnostics are dropped —
/// its only job is silent type resolution; decl-site errors belong elsewhere),
/// then `layout_of_struct` gives the fields' `(name, ty, offset, size)`.
///
/// Errors (each returns `None`, no partial emission):
/// - `[script.no-resume-slot]`: no `*S`-address-param with any ScriptPc field.
/// - `[script.ambiguous-resume-slot]`: two candidate params, or two ScriptPc
///   fields in the pointee.
/// - `[script.resume-width]`: the ScriptPc field's size ≠ 2.
fn discover_resume_slot(
    file: &ast::File,
    decl: &ast::ScriptDecl,
    diags: &mut Vec<Diagnostic>,
) -> Option<ResumeSlot> {
    crate::eval::run_on_eval_stack(|| {
        let mut probe = crate::eval::Evaluator::with_file(file);
        // Candidate (reg, offset, size) tuples across all params. More than one
        // is `[script.ambiguous-resume-slot]`.
        let mut candidates: Vec<(String, i64, usize, Span)> = Vec::new();
        for (pname, pty, pspan) in &decl.params {
            // Params are register spellings (§5.1); only address registers may
            // hold a `*S` pointer we index through. A `*S` on a data register is
            // ill-formed as a resume-slot carrier, so skip non-address params.
            let is_addr = crate::value::Reg::from_name(pname)
                .is_some_and(|r| matches!(r.to_string().as_bytes().first(), Some(b'a')));
            if !is_addr {
                continue;
            }
            let ast::Type::Ptr(inner) = pty else { continue };
            let inner_ty = probe.resolve_type(inner);
            let Some(sname) = probe.struct_name_for_offsetof(&inner_ty, *pspan) else {
                continue;
            };
            let layout = probe.layout_of_struct(&sname, *pspan);
            // ScriptPc fields in THIS pointee (R9b.4: the newtype is recognized
            // by NAME in field-type position — a prelude type, not a builtin).
            let pc_fields: Vec<_> = layout
                .fields
                .iter()
                .filter(|f| f.ty == Ty::Newtype("ScriptPc".to_string()))
                .collect();
            match pc_fields.as_slice() {
                [] => {}
                [f] => candidates.push((pname.clone(), f.offset as i64, f.size, *pspan)),
                _ => {
                    // Two ScriptPc fields in the pointee — ambiguous which is the
                    // resume slot (R9b.3).
                    err(
                        diags,
                        *pspan,
                        format!(
                            "[script.ambiguous-resume-slot] script `{}` param `{pname}` points at \
                             `{sname}`, which has more than one `ScriptPc` field — the resume slot \
                             must be unique",
                            decl.name
                        ),
                    );
                    return None;
                }
            }
        }
        match candidates.as_slice() {
            [] => {
                err(
                    diags,
                    decl.span,
                    format!(
                        "[script.no-resume-slot] script `{}` has no address-register `*Struct` \
                         param whose pointee has a `ScriptPc` field — a script needs a typed \
                         resume slot to save its yield point (D9.3)",
                        decl.name
                    ),
                );
                None
            }
            [(reg, offset, size, span)] => {
                // Width check (R9b.3): the slot is a word; a wider/narrower
                // ScriptPc (e.g. a user `newtype ScriptPc = u32`) breaks the
                // `move.w` store, so guard it here.
                if *size != 2 {
                    err(
                        diags,
                        *span,
                        format!(
                            "[script.resume-width] script `{}`'s resume slot is {size} bytes wide — \
                             a `ScriptPc` resume slot must be 2 bytes (a word)",
                            decl.name
                        ),
                    );
                    return None;
                }
                Some(ResumeSlot { offset: *offset, reg: reg.clone() })
            }
            _ => {
                err(
                    diags,
                    decl.span,
                    format!(
                        "[script.ambiguous-resume-slot] script `{}` has more than one \
                         address-register `*Struct` param whose pointee has a `ScriptPc` field — \
                         the resume slot must be unique",
                        decl.name
                    ),
                );
                None
            }
        }
    })
}

/// The mutable state threaded through the desugar walk (R9b.5 / R9b.6): the
/// resume slot + encoding drive each yield's store; the two counters mint the
/// distinct `__resume$k` / `__loop$d` hygienic labels; `had_no_epilogue`
/// records whether ANY bare yield lacked an effective epilogue (D9.6).
struct Desugar<'a> {
    slot: &'a ResumeSlot,
    encoding: ast::DispatchEncoding,
    script_epilogue: Option<&'a ScriptLabel>,
    yield_count: usize,
    loop_count: usize,
    had_no_epilogue: bool,
    diags: &'a mut Vec<Diagnostic>,
}

impl Desugar<'_> {
    /// Flatten a `ScriptStmt` sequence into proc-body `AsmStmt`s in order.
    fn walk(&mut self, stmts: &[ScriptStmt], out: &mut Vec<AsmStmt>) {
        for stmt in stmts {
            match stmt {
                ScriptStmt::Asm(a) => out.push(a.clone()),
                ScriptStmt::Loop { body, span } => self.desugar_loop(body, *span, out),
                ScriptStmt::Yield { epilogue, resume, span } => {
                    if let Some(r) = resume {
                        self.desugar_named_resume_yield(r, *span, out);
                    } else {
                        self.desugar_yield(epilogue, *span, out);
                    }
                }
            }
        }
    }

    /// `loop { … }` → `__loop$d:` + flattened body + `jbra .__loop$d` (R9b.6).
    /// Allocate `d` BEFORE walking the body so a nested loop gets a distinct id.
    /// The hidden label and the back-edge `jbra` both carry the LOOP
    /// STATEMENT's span (quality review, T2 fold-in): a link-time
    /// `[branch.out-of-reach]` on the back-edge then points at the `loop { }`
    /// site instead of rendering at byte 0.
    fn desugar_loop(&mut self, body: &[ScriptStmt], span: Span, out: &mut Vec<AsmStmt>) {
        let d = self.loop_count;
        self.loop_count += 1;
        let label = format!("__loop${d}");
        out.push(AsmStmt::Label { name: label.clone(), export: false, span });
        self.walk(body, out);
        // `jbra .__loop$d` — a dot-local reference (the probe showed a dot-local
        // operand keeps the leading dot inside the single path segment).
        out.push(jbra(&format!(".{label}"), span));
    }

    /// `yield .label` (D2.30(b)) — lands in the next commit; refuse loudly
    /// so this intermediate state can never ship silently wrong bytes.
    fn desugar_named_resume_yield(&mut self, _r: &ScriptLabel, span: Span, out: &mut Vec<AsmStmt>) {
        err(
            self.diags,
            span,
            "[script.named-resume] `yield .label` is not built yet (D2.30(b) — next commit)"
                .to_string(),
        );
        self.had_no_epilogue = true; // reuse the refuse-whole-script path
        out.push(AsmStmt::Label { name: resume_name(self.yield_count + 1), export: false, span });
        self.yield_count += 1;
    }

    /// `yield [label]` → store the scaled ordinal into the resume slot, `jbra`
    /// the epilogue, then define `__resume$k` (R9b.5 / D9.6). The effective
    /// epilogue is the per-site override, else the `shows` declaration, else a
    /// `[script.no-epilogue]` error at the yield's span.
    fn desugar_yield(&mut self, site: &Option<ScriptLabel>, span: Span, out: &mut Vec<AsmStmt>) {
        let k = self.yield_count + 1; // member 0 is the entry; yields are 1-based.
        self.yield_count = k;
        let epilogue = site.as_ref().or(self.script_epilogue);
        let Some(epilogue) = epilogue else {
            err(
                self.diags,
                span,
                "[script.no-epilogue] a bare `yield` needs an epilogue — declare one with \
                 `shows <label>` on the script or write `yield <label>` per site; an object \
                 that never draws is the footgun (D9.6)"
                    .to_string(),
            );
            self.had_no_epilogue = true;
            // Still emit the resume label so counters/table rows stay aligned;
            // the whole script is refused before emission anyway.
            out.push(AsmStmt::Label { name: resume_name(k), export: false, span });
            return;
        };
        // The store: `move.w #<ordinal>, <offset>(<reg>)`. The ordinal is the
        // member index pre-scaled by the encoding (R9b.2: long_ptrs stores the
        // ×4 ordinal WORD, not a pointer — the slot is uniformly 2 bytes). The
        // displacement is the NUMERIC field offset (an int literal), so the
        // store is independent of bare-field-access rules. Both synthesized
        // InstrLines carry the YIELD SITE's span (quality review, T2 fold-in):
        // a link-time `[branch.out-of-reach]` on the epilogue jbra then renders
        // at the `yield` site instead of at byte 0.
        let ordinal = (k as i128) * self.encoding.scale();
        out.push(yield_store(ordinal as i64, self.slot.offset, &self.slot.reg, span));
        // The epilogue exit: `jbra <label>` (global ident) or `jbra .<label>`
        // (dot-local), mirroring the parser's operand shapes exactly.
        let target = if epilogue.local {
            format!(".{}", epilogue.name)
        } else {
            epilogue.name.clone()
        };
        out.push(jbra(&target, span));
        // The resume point: the engine's saved PC lands here next frame.
        out.push(AsmStmt::Label { name: resume_name(k), export: false, span });
    }
}

/// The `__resume$<k>` hidden proc-local label name for resume member `k`
/// (member 0 = the entry segment). The `$` makes it un-writable by users; it
/// rides ordinary proc-local hygiene (R9b.11).
fn resume_name(k: usize) -> String {
    format!("__resume${k}")
}

/// Synthesize `move.w #<ordinal>, <offset>(<reg>)` — the yield store (R9b.5).
/// The operand shapes mirror the probe EXACTLY: an `Operand::Imm(Int)` and an
/// `Operand::DispInd { disp: Int(offset), inner: Ind { parts: [(Path(reg), None)] } }`.
/// `span` is the originating `yield` SITE's span (quality review, T2 fold-in):
/// stamped on the whole `InstrLine` (and its sub-expressions) so a link-time
/// diagnostic against this store renders at the yield, not at byte 0.
fn yield_store(ordinal: i64, offset: i64, reg: &str, span: Span) -> AsmStmt {
    let inner = Operand::Ind {
        parts: vec![(path_expr(reg, span), None)],
        size: None,
        span,
    };
    AsmStmt::Instr(InstrLine {
        mnemonic: vec![TextOrSplice::Text("move".into())],
        size: Some(TextOrSplice::Text("w".into())),
        operands: vec![
            Operand::Imm(Expr::Int(ordinal, span)),
            Operand::DispInd {
                disp: Expr::Int(offset, span),
                inner: Box::new(inner),
                span,
            },
        ],
        span,
    })
}

/// Synthesize `jbra <target>` where `target` is a bare ident (global) or a
/// `.local` (the leading dot lives inside the single path segment, per the
/// probe). A single `Operand::Plain { expr: Path([target]) }` operand. `span`
/// is the originating yield/loop SITE's span (quality review, T2 fold-in): a
/// link-time `[branch.out-of-reach]` on this jbra renders there instead of at
/// byte 0.
fn jbra(target: &str, span: Span) -> AsmStmt {
    AsmStmt::Instr(InstrLine {
        mnemonic: vec![TextOrSplice::Text("jbra".into())],
        size: None,
        operands: vec![Operand::Plain { expr: path_expr(target, span), size: None, span }],
        span,
    })
}

/// A single-segment `Expr::Path` (the probe's operand shape for a bare
/// register / label / dot-local). The segment string carries the name verbatim
/// (`.top` keeps its leading dot). `span` is the caller's site span.
fn path_expr(seg: &str, span: Span) -> Expr {
    Expr::Path(Path { segments: vec![seg.to_string()], span })
}

/// Push an error diagnostic at `span`.
fn err(diags: &mut Vec<Diagnostic>, span: Span, message: String) {
    diags.push(Diagnostic { level: Level::Error, message, primary: span });
}
