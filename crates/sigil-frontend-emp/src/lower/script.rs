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
        resume_targets: Vec::new(),
        named_resumes: std::collections::HashMap::new(),
        body_labels: collect_body_labels(&decl.body, diags),
        loop_count: 0,
        refuse_script: false,
        wait_widths: resolve_wait_widths(file, decl, diags),
        diags,
    };
    let mut flat = Vec::new();
    ctx.walk(&decl.body, &mut flat);
    let resume_targets = std::mem::take(&mut ctx.resume_targets);
    let refuse_script = ctx.refuse_script;
    if refuse_script {
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
    let members = std::iter::once(resume_name(0))
        .chain(resume_targets)
        .enumerate()
        .map(|(k, raw)| DispatchMember {
            name: format!("R{k}"),
            target: DispatchTarget::Label(Expr::Str(owner.local_symbol(&raw), decl.span)),
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
    // A body ENDING in `wait_frames` is a fallthrough the compiler KNOWS
    // about (trio review M3): the expansion's own `beq` targets its trailing
    // `__wfdone$k` label, so control reaches the `}` every time the park
    // completes — the last-mnemonic heuristic can't see it (the last
    // instruction is the epilogue jbra), so check the statement shape.
    let ends_in_wait = matches!(decl.body.last(), Some(ScriptStmt::WaitFrames { .. }));
    if !as_compat && (ends_in_wait || !super::proc::ends_in_terminator(&buf, placement.cpu)) {
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
/// distinct `__resume$k` / `__loop$d` hygienic labels; `refuse_script`
/// records whether ANY bare yield lacked an effective epilogue (D9.6).
struct Desugar<'a> {
    slot: &'a ResumeSlot,
    encoding: ast::DispatchEncoding,
    script_epilogue: Option<&'a ScriptLabel>,
    /// Raw LOCAL names of resume-table members 1.. in first-need order
    /// (D2.30(b)): a bare yield appends its own `__resume$k`; a named
    /// `yield .x` appends (or joins) the USER label `x`. Member 0 (the
    /// entry) is prepended by the caller.
    resume_targets: Vec<String>,
    /// `yield .x` targets already in the table → their member index
    /// ("becomes OR JOINS a member").
    named_resumes: std::collections::HashMap<String, usize>,
    /// Every user label defined anywhere in the script body (pre-pass) —
    /// the domain a `yield .x` target must come from.
    body_labels: std::collections::HashSet<String>,
    loop_count: usize,
    /// Refuse the whole script (no emission): a bare yield without an
    /// epilogue (D9.6), a named-resume domain error, a wait_frames width or
    /// range failure — any already-diagnosed condition where partial
    /// emission could expose a skewed resume table.
    refuse_script: bool,
    /// Pre-resolved slot widths for the body's `wait_frames` statements, in
    /// walk order (width resolution needs the evaluator's struct layouts, so
    /// it runs BEFORE the walk — `None` = already-diagnosed failure).
    wait_widths: std::collections::VecDeque<Option<u8>>,
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
                ScriptStmt::WaitFrames { n, slot, span } => {
                    self.desugar_wait_frames(n, slot, *span, out);
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

    /// `yield .label` (D2.30(b)) — "frame over; next frame, continue at
    /// `.label`": store the TARGET's member ordinal (pre-scaled), exit via
    /// the epilogue. The target becomes (or joins) a resume-table member; NO
    /// resume point is minted at this site (code after it is reached only by
    /// branching to a label) — the zero-cost park (the old `yield` + `jbra`
    /// pair paid a wasted jump on resume).
    fn desugar_named_resume_yield(&mut self, r: &ScriptLabel, span: Span, out: &mut Vec<AsmStmt>) {
        if !self.body_labels.contains(&r.name) {
            err(
                self.diags,
                span,
                format!(
                    "`yield .{}` names a resume label that is not defined in this script's body — the next frame must land on a label of THIS script",
                    r.name
                ),
            );
            self.refuse_script = true; // refuse the whole script (no emission)
            return;
        }
        // Check-then-allocate (trio review m5): "members exist ⇔ emission
        // happened" shouldn't lean on the whole-script refusal backstop.
        let Some(epilogue) = self.script_epilogue else {
            err(
                self.diags,
                span,
                "[script.no-epilogue] `yield .label` needs an epilogue in scope — declare one with `shows <label>` on the script (D9.6)"
                    .to_string(),
            );
            self.refuse_script = true;
            return;
        };
        let idx = match self.named_resumes.get(&r.name) {
            Some(&idx) => idx,
            None => {
                let idx = self.resume_targets.len() + 1;
                self.resume_targets.push(r.name.clone());
                self.named_resumes.insert(r.name.clone(), idx);
                idx
            }
        };
        let ordinal = (idx as i128) * self.encoding.scale();
        out.push(yield_store(ordinal as i64, self.slot.offset, &self.slot.reg, span));
        let target =
            if epilogue.local { format!(".{}", epilogue.name) } else { epilogue.name.clone() };
        out.push(jbra(&target, span));
    }

    /// `wait_frames #N, <slot>` (D2.30(c)) — EXACTLY the documented tick
    /// idiom, machine-expanded (byte-identical to the hand-written spelling;
    /// same frame accounting — #64 parks 63 drawn frames, proceeds on the
    /// 64th tick):
    ///
    /// ```text
    ///         move.<w> #N, <slot>
    /// __wf$k: subq.<w> #1, <slot>       ; the hidden resume member
    ///         beq      .__wfdone$k
    ///         <yield .__wf$k> ; self-resuming park (D2.30(b))
    /// __wfdone$k:
    /// ```
    fn desugar_wait_frames(&mut self, n: &Expr, slot: &Operand, span: Span, out: &mut Vec<AsmStmt>) {
        let Some(entry) = self.wait_widths.pop_front() else {
            // The pre-pass and the walk visit the same statements in the same
            // order — an empty queue here is walker drift, a compiler bug,
            // never a source error (trio review m1: it must not silently
            // vanish the script).
            err(
                self.diags,
                span,
                "internal: wait_frames width queue exhausted — the pre-pass and the \
                 desugar walk disagree about statement order (compiler bug)"
                    .to_string(),
            );
            self.refuse_script = true;
            return;
        };
        let Some(width) = entry else {
            // Width resolution already diagnosed (unknown field / bad reg /
            // unsupported width) — refuse the script without emission.
            self.refuse_script = true;
            return;
        };
        // Comptime-VISIBLE out-of-range literals are catchable HERE, width in
        // hand (trio review M1): 0 underflows into a full ~2^width-frame
        // wrap, a value past the width truncates silently (`#300` on a u8
        // slot parks 44 frames), and a negative literal is a wrap spelled
        // differently. (A bad value behind a const still arrives unevaluated
        // — that half stays recorded in the tranche notes.)
        let literal = match n {
            Expr::Int(v, _) => Some(*v),
            Expr::Unary { op: ast::UnOp::Neg, expr, .. } => match expr.as_ref() {
                Expr::Int(v, _) => Some(-v),
                _ => None,
            },
            _ => None,
        };
        if let Some(v) = literal {
            let max = (1i64 << (8 * width)) - 1;
            if v < 1 || v > max {
                err(
                    self.diags,
                    span,
                    format!(
                        "`wait_frames #{v}` is outside this {}-bit timer slot's range \
                         (1..={max}) — 0/negative wraps into a ~{}-frame park, and an \
                         over-wide value truncates silently",
                        8 * width,
                        max + 1
                    ),
                );
                self.refuse_script = true;
                return;
            }
        }
        let Some(epilogue) = self.script_epilogue else {
            err(
                self.diags,
                span,
                "[script.no-epilogue] `wait_frames` draws the object every parked frame — declare an epilogue with `shows <label>` on the script (D9.6)"
                    .to_string(),
            );
            self.refuse_script = true;
            return;
        };
        let k = self.resume_targets.len() + 1;
        let wf = format!("__wf${k}");
        self.resume_targets.push(wf.clone());
        let done = format!("__wfdone${k}");
        let sz = if width == 1 { "b" } else { "w" };
        out.push(sized_instr("move", sz, Operand::Imm(n.clone()), slot.clone(), span));
        out.push(AsmStmt::Label { name: wf.clone(), export: false, span });
        out.push(sized_instr("subq", sz, Operand::Imm(Expr::Int(1, span)), slot.clone(), span));
        out.push(AsmStmt::Instr(InstrLine {
            mnemonic: vec![TextOrSplice::Text("beq".into())],
            size: None,
            operands: vec![Operand::Plain {
                expr: path_expr(&format!(".{done}"), span),
                size: None,
                span,
            }],
            dispatch_bound: None,
            discards: None,
            span,
        }));
        // The self-resuming yield: store __wf$k's ordinal, exit via epilogue.
        let ordinal = (k as i128) * self.encoding.scale();
        out.push(yield_store(ordinal as i64, self.slot.offset, &self.slot.reg, span));
        let target =
            if epilogue.local { format!(".{}", epilogue.name) } else { epilogue.name.clone() };
        out.push(jbra(&target, span));
        out.push(AsmStmt::Label { name: done, export: false, span });
    }

    /// `yield [label]` → store the scaled ordinal into the resume slot, `jbra`
    /// the epilogue, then define `__resume$k` (R9b.5 / D9.6). The effective
    /// epilogue is the per-site override, else the `shows` declaration, else a
    /// `[script.no-epilogue]` error at the yield's span.
    fn desugar_yield(&mut self, site: &Option<ScriptLabel>, span: Span, out: &mut Vec<AsmStmt>) {
        let k = self.resume_targets.len() + 1; // member 0 is the entry.
        self.resume_targets.push(resume_name(k));
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
            self.refuse_script = true;
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

/// Pre-pass for D2.30(c): resolve each `wait_frames` slot's field WIDTH, in
/// walk order. The slot operand is `field(aN)` — `aN` must be one of the
/// script's `*Struct` params and `field` a 1- or 2-byte field of that struct
/// (the width drives the expansion's `.b`/`.w` suffixes). Resolution needs
/// the evaluator's struct layouts, so it runs here (mirroring
/// `discover_resume_slot`) rather than inside the type-free desugar walk.
/// A failed resolution is diagnosed here and queued as `None` (the walk then
/// refuses the script without emission).
fn resolve_wait_widths(
    file: &ast::File,
    decl: &ast::ScriptDecl,
    diags: &mut Vec<Diagnostic>,
) -> std::collections::VecDeque<Option<u8>> {
    fn visit(
        stmts: &[ScriptStmt],
        f: &mut impl FnMut(&Expr, &Operand, Span),
    ) {
        for st in stmts {
            // Explicit variant list (trio review m3): this MUST visit in the
            // same order `Desugar::walk` does — a new nested-body variant has
            // to be handled here consciously, not skipped by a wildcard.
            match st {
                ScriptStmt::WaitFrames { n, slot, span } => f(n, slot, *span),
                ScriptStmt::Loop { body, .. } => visit(body, f),
                ScriptStmt::Asm(_) | ScriptStmt::Yield { .. } => {}
            }
        }
    }
    let mut sites: Vec<(Operand, Span)> = Vec::new();
    visit(&decl.body, &mut |_n, slot, span| sites.push((slot.clone(), span)));
    if sites.is_empty() {
        return std::collections::VecDeque::new();
    }
    crate::eval::run_on_eval_stack(|| {
        let mut probe = crate::eval::Evaluator::with_file(file);
        let mut out = std::collections::VecDeque::new();
        for (slot, span) in sites {
            out.push_back(wait_slot_width(&mut probe, decl, &slot, span, diags));
        }
        // Field-space resolution errors ([operand.unknown-field] /
        // [operand.ambiguous-field]) land on the probe — surface them.
        diags.append(&mut probe.diags);
        out
    })
}

/// One `wait_frames` slot: `field(aN)` → the field's byte width (1 or 2).
fn wait_slot_width(
    probe: &mut crate::eval::Evaluator,
    decl: &ast::ScriptDecl,
    slot: &Operand,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Option<u8> {
    // Shape: DispInd { disp: Path(field), inner: Ind[(Path(aN))] }.
    let (field, reg) = match slot {
        Operand::DispInd { disp: Expr::Path(fp), inner, .. } if fp.segments.len() == 1 => {
            match inner.as_ref() {
                Operand::Ind { parts, .. } if parts.len() == 1 => match &parts[0].0 {
                    Expr::Path(rp) if rp.segments.len() == 1 => {
                        (fp.segments[0].clone(), rp.segments[0].clone())
                    }
                    _ => (fp.segments[0].clone(), String::new()),
                },
                _ => (fp.segments[0].clone(), String::new()),
            }
        }
        _ => {
            err(
                diags,
                span,
                "`wait_frames` parks on a NAMED struct field: the slot must be spelled `field(aN)` (e.g. `timer(a0)`)"
                    .to_string(),
            );
            return None;
        }
    };
    // The register must be one of the script's `*Struct` params.
    for (pname, pty, pspan) in &decl.params {
        if *pname != reg {
            continue;
        }
        let ast::Type::Ptr(inner) = pty else { continue };
        let inner_ty = probe.resolve_type(inner);
        let Some(sname) = probe.struct_name_for_offsetof(&inner_ty, *pspan) else { continue };
        // The SAME field space ordinary operands use (D6.A3): direct fields
        // plus in-scope overlays over this struct — a park timer usually
        // lives in the object's `vars …: sst_custom` overlay, not in Sst
        // itself. A miss/ambiguity is diagnosed by the probe
        // ([operand.unknown-field] / [operand.ambiguous-field]) and surfaced
        // by the caller.
        let (_disp, size) = probe.resolve_field_disp(&sname, &field, span)?;
        return match size {
            1 => Some(1),
            2 => Some(2),
            other => {
                err(
                    diags,
                    span,
                    format!(
                        "`wait_frames` slot `{field}` is {other} bytes — a park timer \
                         must be a u8 or u16 field"
                    ),
                );
                None
            }
        };
    }
    err(
        diags,
        span,
        format!(
            "`wait_frames` slot register `{reg}` is not one of this script's `*Struct` params — the timer must live in the object's own state"
        ),
    );
    None
}

/// Pre-pass for D2.30(b): every user label defined anywhere in the script
/// body (loops included) — the domain a `yield .x` resume target must come
/// from ("the next frame must land on a label of THIS script").
fn collect_body_labels(
    stmts: &[ScriptStmt],
    diags: &mut Vec<Diagnostic>,
) -> std::collections::HashSet<String> {
    // Explicit variant list (trio review m3): a future ScriptStmt with a
    // nested body must be a compile error here, not a silent walk desync.
    fn walk(
        stmts: &[ScriptStmt],
        out: &mut std::collections::HashSet<String>,
        diags: &mut Vec<Diagnostic>,
    ) {
        for st in stmts {
            match st {
                ScriptStmt::Asm(AsmStmt::Label { name, export, span }) => {
                    let _ = export;
                    if !out.insert(name.clone()) {
                        // Early, spanned duplicate detection (trio review m4)
                        // — the alternative is a spanless link-time
                        // "symbol redefined".
                        err(
                            diags,
                            *span,
                            format!(
                                "label `.{name}` is defined twice in this script body — \
                                 resume targets and branches need one definition"
                            ),
                        );
                    }
                }
                ScriptStmt::Asm(AsmStmt::If { then, els, span, .. }) => {
                    // Comptime-`if` branches (tranche 5) hold `AsmStmt` only —
                    // a `yield` is a `ScriptStmt` and cannot nest inside one —
                    // but LABELS can, and a resume/branch target defined under
                    // an `if` would only conditionally exist. Refuse rather
                    // than half-support: hoist the label out of the `if`.
                    let mut nested = Vec::new();
                    collect_if_labels(then, &mut nested);
                    if let Some(els) = els {
                        collect_if_labels(els, &mut nested);
                    }
                    if let Some(first) = nested.first() {
                        err(
                            diags,
                            *span,
                            format!(
                                "label `.{first}` is defined inside a comptime `if` \
                                 in a script body — a resume/branch target must \
                                 exist unconditionally; define it outside the `if`"
                            ),
                        );
                    }
                }
                ScriptStmt::Asm(_) => {}
                ScriptStmt::Loop { body, .. } => walk(body, out, diags),
                ScriptStmt::Yield { .. } | ScriptStmt::WaitFrames { .. } => {}
            }
        }
    }
    let mut out = std::collections::HashSet::new();
    walk(stmts, &mut out, diags);
    out
}

/// Labels defined anywhere under a comptime-`if`'s branches (recursively) —
/// the script-body refusal above needs their names for its diagnostic.
fn collect_if_labels(stmts: &[AsmStmt], out: &mut Vec<String>) {
    for st in stmts {
        match st {
            AsmStmt::Label { name, .. } => out.push(name.clone()),
            AsmStmt::If { then, els, .. } => {
                collect_if_labels(then, out);
                if let Some(els) = els {
                    collect_if_labels(els, out);
                }
            }
            _ => {}
        }
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
                disp_spliced: false,
                field_size_override: None,
                span,
            },
        ],
        dispatch_bound: None,
        discards: None,
        span,
    })
}

/// Synthesize a two-operand sized instruction (`move.b #N, slot` /
/// `subq.w #1, slot`) — the `wait_frames` expansion's building block.
fn sized_instr(mnemonic: &str, sz: &str, src: Operand, dst: Operand, span: Span) -> AsmStmt {
    AsmStmt::Instr(InstrLine {
        mnemonic: vec![TextOrSplice::Text(mnemonic.into())],
        size: Some(TextOrSplice::Text(sz.into())),
        operands: vec![src, dst],
        dispatch_bound: None,
        discards: None,
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
        dispatch_bound: None,
        discards: None,
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
