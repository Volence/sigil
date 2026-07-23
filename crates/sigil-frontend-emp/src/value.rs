//! The comptime `Value` model (Spec 2, Plan 2 — D-P2.2).
//!
//! A [`Value`] is the result of evaluating an `.emp` expression at compile
//! time. Values are pure data with no byte layout — memory layout is Plan 3.
//! Later tasks add the evaluator that produces these; this module only
//! defines the value domain, its [`Display`](std::fmt::Display) rendering, and
//! small type-introspection helpers.
use crate::ast::Expr;
use crate::eval::Env;
use crate::layout::Ty;
use sigil_span::Span;
use std::fmt;

/// A comptime value.
///
/// `PartialEq` is derived (not `Eq`): [`Value::Float`] holds an `f64`, which is
/// only `PartialEq`. Two [`Value::Lambda`]s compare by structural equality of
/// their parameter names, body AST, and captured environment.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A comptime integer. Arbitrary-precision in spirit; v1 uses `i128`.
    Int(i128),
    /// A floating-point value.
    Float(f64),
    /// A string value.
    Str(String),
    /// A boolean value.
    Bool(bool),
    /// An array value: `[a, b, c]`.
    Array(Vec<Value>),
    /// A half-open range `lo..hi`, a first-class comptime value iterated by
    /// `for` / `.map` in later tasks.
    Range {
        /// Inclusive lower bound.
        lo: i128,
        /// Exclusive upper bound.
        hi: i128,
    },
    /// A struct value with ordered fields. No byte layout (that is Plan 3).
    Struct {
        /// The struct type's name.
        ty_name: String,
        /// Ordered `(field name, value)` pairs.
        fields: Vec<(String, Value)>,
    },
    /// A tagged enum variant, comptime only.
    Enum {
        /// The enum type's name.
        ty_name: String,
        /// The active variant's name.
        variant: String,
        /// The variant's payload values, if any.
        payload: Vec<Value>,
    },
    /// A tuple value: `(a, b)` — tuple literals and multi-return.
    Tuple(Vec<Value>),
    /// The unit value: statements with no value, `while`, empty `else`.
    Unit,
    /// A lambda `|x| e`.
    ///
    /// Lambdas are not parsed until Task 6; this variant exists now so the
    /// value domain is complete and forward-compatible. The body is the AST
    /// expression to evaluate and `captured` is the defining environment
    /// (captured by value — see [`Env`]'s clone semantics). Kept in
    /// `value.rs` (not `eval.rs`): `Env` is cheaply/independently clonable and
    /// embeds without ordering issues, so all `Value` variants live together.
    Lambda {
        /// The lambda's parameter names, in order.
        params: Vec<String>,
        /// The lambda's body expression.
        body: Box<Expr>,
        /// The environment captured at the lambda's definition site.
        captured: Env,
    },
    /// A first-class reference to a named `comptime fn` (D2.12). A bare
    /// function name evaluates to this so it can be passed as a value —
    /// `bands.map(band_entry)` feeds `band_entry` to `map`. Carries only the
    /// fn's name; the [`Evaluator`](crate::eval::Evaluator) resolves it against
    /// the file's fn index when the value is applied.
    FnRef(String),
    /// A value carrying a sized nominal type (T5, D-P3.3): the FIRST place
    /// comptime arithmetic wraps. Produced by newtype construction (`Name(x)`),
    /// `fixed<>` multiplication, and `rescale`. `val` is normally a
    /// [`Value::Int`] — the stored integer, which for a `fixed<I,F>` is the
    /// SCALED value (`x·2^F`). A `Typed` value is transparent to everything
    /// EXCEPT type-aware arithmetic and diagnostics: it erases to its stored int
    /// (§8.3) via [`as_stored_int`](Value::as_stored_int). Bare comptime `int`
    /// arithmetic is untouched — only these values wrap at their width/scale.
    Typed {
        /// The value's nominal type (a [`Ty::Newtype`] or a bare [`Ty::Fixed`]).
        ty: Box<Ty>,
        /// The stored integer (normally a [`Value::Int`]).
        val: Box<Value>,
    },
    /// A CHECKED, CPU-NEUTRAL structured data buffer (T7, D-P3.5): the Plan 3 /
    /// Plan 4 seam. Produced by `byte`/`bytes`/`Data.empty`/`++` and by lowering
    /// a typed comptime value against its `Ty` (`lower_to_data`). It commits NO
    /// endianness and resolves NO pointer address — those are Plan 4; here the
    /// cells stay structured so Plan 4 can pick byte order and resolve fixups.
    Data(DataBuf),
    /// A RESOLVED instruction list (T1, D-P4.3): the `Code` monoid's carrier and
    /// the Plan 4 lowering seam for machine code, parallel to [`Value::Data`].
    /// Produced by `asm { }` / `Code.empty` / `++` (T3); NOT a lazy template —
    /// each `{expr}` splice is already resolved to a [`CodeOperand`] here.
    Code(CodeBuf),
    /// A comptime operand-size class (`b`/`w`/`l`/`s`) — the value a `{w}`
    /// mnemonic-size splice resolves to (§6.2). Emp-side; carries no ISA.
    Width(Width),
    /// A comptime condition-code class (`ne`/`eq`/…) — the value a `{cc}`
    /// mnemonic splice resolves to (`b{cc}`, §6.2). Emp-side; carries no ISA.
    Cc(Cc),
    /// A comptime register class (`d0`..`a7`) — the value a `{reg}` operand
    /// splice resolves to (§6.2). Emp-side; carries no ISA.
    Reg(Reg),
    /// A first-class LABEL VALUE (D-PP.3): a reference to a named link symbol — a
    /// `proc` or `data` item, module-local / imported / prelude. Produced when a
    /// bareword (or dotted path) in comptime VALUE position resolves to nothing
    /// the evaluator knows (no local/const/comptime-fn), so it is DEFERRED to
    /// link exactly as the string form `"init"` is. Carries only the (possibly
    /// dotted) symbol NAME; the address is NOT resolved here — a label value
    /// lowers to the same [`Cell::SymRef`] the string form does (`lower_ptr`) and
    /// splices to the same [`CodeOperand::Sym`] (`classify_operand_splice`), so a
    /// link-time fixup resolves it. Distinct from [`Value::Str`] on purpose: a
    /// label is a symbol reference, so it type-errors in a non-pointer field
    /// (naming `label`), rejects comptime address arithmetic (`init + 2`), and
    /// binds ONLY a `Label`-typed comptime fn param — none of which a raw string
    /// should do. It NEVER folds to a comptime integer (emission stays link-time).
    Label(String),
    /// A LINK-TIME integer value (D-H.2): an integer known only AFTER
    /// `resolve_layout` picks every relaxable fragment's final width. Produced by
    /// a PROVISIONAL `here()` — one whose position sits after a size-relaxable
    /// instruction (`jbra`/`jbsr`, an unsized branch, a bare `jmp`/`jsr`) in the
    /// currently-open section — which yields `LinkExpr(Sym(<anchor label>))`. The
    /// wrapped [`Expr`](sigil_ir::expr::Expr) is a RESIDUAL expression tree: the
    /// comptime operators IR `Expr` can represent lift onto it (an `Int` lifts via
    /// `Expr::Int`, range-checked i128→i64), so `here() + 4` builds
    /// `Add(Sym, Int(4))` instead of folding. Everything a `LinkExpr` reaches that
    /// is NOT one of those operators is the loud `[here.provisional]` error (it
    /// cannot size or steer comptime evaluation). A provisional `here()` is
    /// EMITTABLE (a plain `Sym` tree lowers via the item label, D-H.3) and
    /// GUARDABLE (`ensure`/`ensure_fatal` defer it to a `LinkAssert`, D-H.4);
    /// every other use refuses. Distinct from [`Value::Label`]: a label is a bare
    /// symbol reference with no arithmetic; a `LinkExpr` is a foldable integer
    /// expression that simply cannot be folded until link.
    LinkExpr(sigil_ir::expr::Expr),
    /// An "error already reported here" sentinel (D-P2.9). Operations on
    /// `Poison` yield `Poison` silently so one bad subexpression does not fan
    /// out into a cascade of diagnostics.
    Poison,
}

/// A checked, CPU-neutral, structured data buffer (T7, D-P3.5). `size` is the
/// total byte length (the sum of every cell's byte size); `cells` preserves the
/// STRUCTURE (scalars keep their width/signedness, pointer references stay
/// symbolic) so Plan 4 can commit endianness and resolve fixups. Building it via
/// [`concat`](DataBuf::concat) / [`push`](DataBuf::push) keeps `size` in step
/// with `cells`.
#[derive(Clone, Debug, PartialEq)]
pub struct DataBuf {
    /// The buffer's cells, in emission order.
    pub cells: Vec<Cell>,
    /// The total byte size — the sum of every cell's byte size. CPU-neutral.
    pub size: usize,
}

/// One structured cell of a [`DataBuf`] (T7). Kept structured (not a flat byte
/// blob) so Plan 4 has the width/signedness it needs to pick a byte order, and
/// the symbol name it needs to emit a relocation.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    /// A range-checked sized integer. `width ∈ {1, 2, 4}` bytes; `signed`
    /// records whether the source type was signed. NO endianness is committed —
    /// Plan 4 serializes this to `width` bytes in the target's byte order.
    Scalar {
        /// The (already range-checked) integer value.
        value: i128,
        /// Byte width: 1, 2, or 4.
        width: u8,
        /// Whether the source type was signed.
        signed: bool,
        /// Explicit little-endian override (`u16le`, R-T0.1 / DSM.7): when
        /// `true`, Plan 4 ALWAYS serializes this cell little-endian regardless
        /// of the section's CPU. `false` for every ordinary `u8`/`i8`/`u16`/
        /// `i16`/`u32`/`i32` cell — the default, CPU-driven byte order.
        le: bool,
    },
    /// A run of width-1 bytes (from `byte`/`bytes`/`++`). Single bytes have no
    /// byte order, so this stays CPU-neutral as raw bytes.
    Bytes(Vec<u8>),
    /// A pointer-typed field: a reference to a named symbol, `width` bytes wide.
    /// Plan 4 resolves the name to an address and emits a fixup; Plan 3 does NOT.
    ///
    /// `windowed` records whether this is a Z80 *bank pointer* (`winptr(sym)`,
    /// §7.2 — a 2-byte windowed pointer, `BankPtr16Le`) versus a plain absolute
    /// pointer (a 68k `Abs32`/`Abs16`). Plan 4's fixup-kind selection (D-P4.5)
    /// reads (`width`, section CPU, `windowed`): a plain 68k pointer is
    /// width 4 (`Abs32Be`) — the default (D-P3.7); a windowed Z80 pointer is
    /// width 2 (`BankPtr16Le`). An un-windowed pointer in a Z80 section is the
    /// `[cross-cpu.unwindowed-pointer]` error.
    SymRef {
        /// The referenced symbol's name.
        name: String,
        /// Pointer byte width (4 for a plain absolute pointer, 2 for a `winptr`).
        width: u8,
        /// Whether this is a Z80 windowed bank pointer (`winptr(sym)`, §7.2).
        ///
        /// A `bool` suffices while the two pointer flavors are distinguishable by
        /// `(width, windowed)`. If a THIRD flavor appears that a bool cannot name
        /// (e.g. one not separable by width), migrate this to a
        /// `PtrKind { Absolute, Windowed, … }` field rather than adding a second
        /// bool.
        windowed: bool,
    },
    /// A self-relative signed **word** offset for an `offsets` table entry:
    /// emits `dc.w target - base` (2 bytes) via a `RelWord16Be` fixup. Distinct
    /// from `SymRef` (an absolute pointer) — this is a symbol *difference*.
    RelOffset {
        /// The table's base symbol (the offsets block's own label).
        base: String,
        /// The entry's target symbol.
        target: String,
    },
    /// A general link-time value expression, `width` bytes wide (S2-D13f /
    /// R7m.4). A [`Value::LinkExpr`] landing in a data cell of declared width
    /// `w ∈ {1, 2, 4}` lowers to this. Plan 4 selects a width/CPU `ValueN` fixup
    /// kind (`Value8`/`Value16Be`/`Value16Le`/`Value32Be`/`Value32Le`); the
    /// linker folds `expr` against the FINAL table and writes the integer
    /// verbatim after an UNSIGNED-window range check. DISTINCT from `SymRef`:
    /// that is an *address* pointer (masked/sign-checked); this is a plain
    /// integer value carrying an arbitrary residual tree (`here() + 2`,
    /// `bankid(L)`, …), not just a bare symbol.
    Expr {
        /// The residual link-time expression to fold at link.
        expr: sigil_ir::expr::Expr,
        /// Byte width: 1, 2, or 4.
        width: u8,
        /// Explicit little-endian override (`u16le`, R-T0.1 / DSM.7): when
        /// `true`, the linker's VALUE fixup-kind selection ALWAYS picks the
        /// little-endian kind (`Value16Le`) regardless of the section's CPU.
        /// `false` for every ordinary width/CPU-driven `Cell::Expr`.
        le: bool,
    },
}

impl Cell {
    /// The cell's byte size: a scalar/symref is its `width`, a `RelOffset` is a
    /// fixed 2-byte word, a byte run is its length.
    pub fn byte_size(&self) -> usize {
        match self {
            Cell::Scalar { width, .. }
            | Cell::SymRef { width, .. }
            | Cell::Expr { width, .. } => *width as usize,
            Cell::RelOffset { .. } => 2,
            Cell::Bytes(b) => b.len(),
        }
    }
}

impl DataBuf {
    /// The empty buffer — the `Data` monoid's identity (`Data.empty`).
    pub fn empty() -> Self {
        DataBuf { cells: Vec::new(), size: 0 }
    }

    /// The monoid `++`: append `b`'s cells after `a`'s and sum their sizes.
    pub fn concat(mut a: DataBuf, b: DataBuf) -> DataBuf {
        a.cells.extend(b.cells);
        a.size += b.size;
        a
    }

    /// Push one cell, keeping [`size`](DataBuf::size) in step with `cells`.
    pub fn push(&mut self, cell: Cell) {
        self.size += cell.byte_size();
        self.cells.push(cell);
    }

    /// The comptime-known raw byte at `offset` (D2.33 indexing), or `None`
    /// where the byte is NOT knowable before link/stream time. Knowable:
    /// a [`Cell::Bytes`] run, and a width-1 [`Cell::Scalar`] (a single byte
    /// has no byte order; the stored two's-complement low byte IS the
    /// emitted byte). Not knowable: a multi-byte scalar (its byte order is
    /// committed by the section's CPU at stream time), and every symbolic
    /// cell (`SymRef`/`RelOffset`/`Expr` — values folded at link).
    ///
    /// Out-of-range offsets also return `None`; the caller distinguishes
    /// (it bounds-checks against [`size`](DataBuf::size) first, so its
    /// diagnostic can tell "past the end" from "not comptime-known").
    pub fn byte_at(&self, offset: usize) -> Option<u8> {
        let mut pos = 0usize;
        for cell in &self.cells {
            let sz = cell.byte_size();
            if offset < pos + sz {
                return match cell {
                    Cell::Bytes(v) => Some(v[offset - pos]),
                    Cell::Scalar { value, width: 1, .. } => Some((value & 0xFF) as u8),
                    _ => None,
                };
            }
            pos += sz;
        }
        None
    }
}

/// A RESOLVED instruction list (T1, D-P4.3): the `Code` monoid's carrier,
/// parallel to [`DataBuf`]. Unlike a lazy template, every `{expr}` splice is
/// already resolved to a [`CodeOperand`]; endianness and label/symbol addresses
/// stay UNRESOLVED — those are Plan 4 lowering. Build it via
/// [`concat`](CodeBuf::concat) / [`push`](CodeBuf::push).
#[derive(Clone, Debug, PartialEq)]
pub struct CodeBuf {
    /// The code fragment's ordered pieces, in emission order.
    pub items: Vec<CodeItem>,
}

/// One ordered piece of a [`CodeBuf`] (T1): a label, a single instruction, or a
/// [`DataBuf`] spliced into the code stream (§6.2).
#[derive(Clone, Debug, PartialEq)]
pub enum CodeItem {
    /// A label definition: `.draw:` or `export .done:`.
    Label {
        /// The label's name.
        name: String,
        /// Whether the label is exported.
        export: bool,
        /// The defining site's span.
        span: Span,
    },
    /// A single machine instruction: `mnemonic[.size] ops…`. `mnemonic` and
    /// `size` are already resolved (a `{cc}` mnemonic splice or a `{w}` size
    /// splice has been substituted into the strings/`Option<Width>`).
    Instr {
        /// The resolved mnemonic (e.g. `"move"`, `"bne"`).
        mnemonic: String,
        /// The resolved operand size, if any.
        size: Option<Width>,
        /// The resolved operands, in order.
        ops: Vec<CodeOperand>,
        /// The instruction's span.
        span: Span,
        /// The trailing `as Type` annotation (G5 §7 tier 5), if any — carried
        /// verbatim from `InstrLine.dispatch_bound`. On a producing instruction
        /// (`move.w d3, d2 as GridX`) the type-slice pass reads it to bless the
        /// destination register with a domain newtype; on an indirect call it is
        /// the dispatch bound (consumed separately via the AST). Emits NOTHING.
        as_type: Option<String>,
    },
    /// A [`DataBuf`] spliced into a code stream (§6.2) — a `Data` value inlined
    /// between instructions. Produced today by `dc.b`/`dc.w`/`dc.l` statements
    /// (tranche 8 — code-embedded constant data, e.g. an error handler's
    /// format-string bytes between a `jsr` and its resume label). Carries the
    /// producing statement's span so emission diagnostics anchor at the source.
    Inline(DataBuf, Span),
}

/// A resolved splice operand value (T1): the CPU-neutral surface forms an
/// `asm { }` operand can take once its `{expr}` splices are evaluated. This is
/// the INTERNAL operand type inside [`CodeItem::Instr`], NOT a [`Value`] variant.
#[derive(Clone, Debug, PartialEq)]
pub enum CodeOperand {
    /// An immediate: `#42`.
    Imm(i128),
    /// A LINK-TIME immediate: `#extern("SongTable")` / an equ-aliased
    /// extern sum (tranche 5 — the emp mirror of the AS front-end's
    /// `try_defer_long_imm`): the value cannot fold until link, so the
    /// instruction encodes with a 4-byte hole and ONE `Value32Be` fixup.
    /// `.l`-sized instructions only (a `.b`/`.w` symbolic immediate has no
    /// deferral yet — the ledgered width-extension gap); policed at lowering
    /// where the resolved size is known.
    ImmLink {
        /// The residual link-time expression (extern Syms + arithmetic).
        target: sigil_ir::expr::Expr,
    },
    /// A register: `d0`.
    Reg(Reg),
    /// The 68k status register operand: `sr` (`move.w sr, -(sp)` /
    /// `move.w #$2700, sr` — the interrupt-mask idiom). A register-class
    /// word in operand position (registers win over ordinary names there),
    /// like `d0`..`a7`.
    Sr,
    /// The 68k condition-code register operand: `ccr`.
    Ccr,
    /// A condition code used as an operand.
    Cc(Cc),
    /// A named symbol / label reference.
    Sym(String),
    /// A symbol + constant byte offset — a `Item.field` field-address memory
    /// operand (D-PP.5). `sym` is the data item's link symbol, `off` the field's
    /// `offsetof` within its struct; lowering rides the same `RelaxAbsSym` seam
    /// as [`Sym`](CodeOperand::Sym) but with fixup target `sym + off` (a foldable
    /// `Add`), so the linker widths the SUM by `asl_width_rule`. Distinct from a
    /// `DispInd` — there is NO base register; this is an ABSOLUTE address.
    SymOff {
        /// The data item's link symbol.
        sym: String,
        /// The field's byte offset within the item's struct.
        off: i128,
    },
    /// Register indirect: `(a0)`.
    Ind(Reg),
    /// Pre-decrement indirect: `-(a7)`.
    PreDec(Reg),
    /// Post-increment indirect: `(a0)+`.
    PostInc(Reg),
    /// Displacement indirect: `4(a0)`.
    DispInd {
        /// The displacement.
        disp: i128,
        /// The base register.
        reg: Reg,
    },
    /// Explicit-width absolute with a comptime address: `($FFFF8022).w` /
    /// `($C00004).l` — the AS-parity FORCED-width spelling (the bare-symbol
    /// idiom, which relaxes via the width rule, stays the new-style
    /// default). The `.w` window is validated at eval (asl's sign-extension
    /// rule) and re-checked at lowering.
    AbsInt {
        /// The comptime address.
        addr: i128,
        /// `true` for `.l`.
        long: bool,
    },
    /// Explicit-width absolute with a symbolic address: `(Sym).w` /
    /// `(Sym).l` — the WIDTH is pinned by the author; the address defers as
    /// ONE fixed-width fixup (no RelaxAbsSym candidate pair).
    AbsSym {
        /// The hygiene-resolved link symbol.
        target: String,
        /// `true` for `.l`.
        long: bool,
    },
    /// An-indexed indirect: `(a0,d2.w)` / `d8(a0,d2.w)` — 68k `(d8,An,Xn)`,
    /// brief extension word. Unlike the pc-indexed sibling there is no
    /// link-time fact here: the displacement is comptime-resolved and
    /// range-checked to the brief extension's signed-8-bit field at eval
    /// time (defense-in-depth re-check at lowering).
    IndIdx {
        /// The base address register.
        reg: Reg,
        /// The comptime displacement (i8 range).
        disp: i128,
        /// The index register.
        xn: Reg,
        /// `true` for `.l` (long index), `false` for `.w` (sign-extended,
        /// the AS-matching default when unsuffixed).
        xlong: bool,
    },
    /// A `movem` register-list operand (`d0-d1/a0`), already folded to the
    /// CANONICAL 16-bit mask (bit0=D0..bit7=D7, bit8=A0..bit15=A7) — the same
    /// convention as `sigil_isa::m68k::Operand::RegList`. Reglist parsing is
    /// MNEMONIC-DIRECTED (D-P1H.2, port #1 hblank recon): this variant is only
    /// ever produced while lowering a `movem` instruction's operands, never a
    /// general operand-grammar form, so it cannot leak into other mnemonics.
    RegList(u16),
    /// Plain PC-relative: `Sym(pc)` — 68k `(d16,PC)`. `target` is the
    /// hygiene-resolved link symbol (mirrors [`Sym`](CodeOperand::Sym)); the
    /// displacement is a link-time fixup (`FixupKind::PcRelDisp16`), not
    /// resolved here — same "target stays symbolic" contract as a branch.
    PcRel {
        /// The target's resolved link symbol.
        target: String,
        /// Comptime addend on the target (`Sym+n(pc)` / `Sym-n(pc)`) — the
        /// dispatch-table anchor idiom (`jmp .cc_table-4(pc,d0.w)`, tranche
        /// 9) where the adjusted address lands mid-instruction so no
        /// relocated label can express it. 0 for the plain `Sym(pc)` form.
        addend: i64,
    },
    /// PC-indexed: `Sym(pc,Xn.size)` — 68k `(d8,PC,Xn)`, brief extension word.
    /// `target` is the hygiene-resolved link symbol; `xn`/`xlong` are the
    /// index register and its width (`.w` sign-extended / `.l`), resolved
    /// eagerly (a register, not a link-time fact). The displacement is a
    /// link-time `FixupKind::PcRelDisp8` fixup.
    PcRelIdx {
        /// The target's resolved link symbol.
        target: String,
        /// Comptime addend on the target (see [`PcRel::addend`]).
        addend: i64,
        /// The index register.
        xn: Reg,
        /// `true` for `.l` (long index), `false` for `.w` (sign-extended,
        /// the AS-matching default when unsuffixed).
        xlong: bool,
    },
}

/// A comptime operand-size class (§6.2), emp-side (no ISA import). Modeled on
/// the 68k `b`/`w`/`l` sizes plus the branch `s` (short) suffix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    /// Byte (`.b`).
    B,
    /// Word (`.w`).
    W,
    /// Long (`.l`).
    L,
    /// Short branch (`.s`).
    S,
}

/// A comptime condition-code class (§6.2), emp-side. Membership mirrors the
/// shape of the 68k condition set (`sigil_isa::m68k::Cond`) without importing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cc {
    /// True (always).
    T,
    /// False (never).
    F,
    /// High (unsigned `>`).
    Hi,
    /// Low or same (unsigned `<=`).
    Ls,
    /// Carry clear (unsigned `>=`).
    Cc,
    /// Carry set (unsigned `<`).
    Cs,
    /// Not equal.
    Ne,
    /// Equal.
    Eq,
    /// Overflow clear.
    Vc,
    /// Overflow set.
    Vs,
    /// Plus (non-negative).
    Pl,
    /// Minus (negative).
    Mi,
    /// Greater or equal (signed).
    Ge,
    /// Less than (signed).
    Lt,
    /// Greater than (signed).
    Gt,
    /// Less or equal (signed).
    Le,
}

/// A comptime register class (§6.2), emp-side. The 68k data (`D0`..`D7`) and
/// address (`A0`..`A7`) register files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reg {
    /// Data register `d0`.
    D0,
    /// Data register `d1`.
    D1,
    /// Data register `d2`.
    D2,
    /// Data register `d3`.
    D3,
    /// Data register `d4`.
    D4,
    /// Data register `d5`.
    D5,
    /// Data register `d6`.
    D6,
    /// Data register `d7`.
    D7,
    /// Address register `a0`.
    A0,
    /// Address register `a1`.
    A1,
    /// Address register `a2`.
    A2,
    /// Address register `a3`.
    A3,
    /// Address register `a4`.
    A4,
    /// Address register `a5`.
    A5,
    /// Address register `a6`.
    A6,
    /// Address register `a7` (stack pointer).
    A7,
}

impl Reg {
    /// Parse a register name (`d0`..`d7`, `a0`..`a7`, plus the `sp` alias for
    /// `a7`) to its [`Reg`], else `None`. The canonical spelling→register map,
    /// shared by the operand mapper and the proc-param binding (D6.A3); it is
    /// the inverse of [`Display`].
    ///
    /// `sp` is a general operand-layer alias for `a7` (port #1 hblank recon,
    /// D-P1H.1), not a distinct register kind: it is byte-identical to `a7`
    /// everywhere a register name is recognized (plain operand, `(sp)`,
    /// `(sp)+`, `-(sp)`, `d16(sp)` displacement, and inside `movem` register
    /// lists). The whole aeon codebase spells the stack pointer `sp`, so every
    /// code port needs this. [`Display`](fmt::Display) still renders `Reg::A7`
    /// as `"a7"` — `sp` is parse-only sugar, not a new stored spelling.
    pub fn from_name(name: &str) -> Option<Reg> {
        Some(match name {
            "d0" => Reg::D0,
            "d1" => Reg::D1,
            "d2" => Reg::D2,
            "d3" => Reg::D3,
            "d4" => Reg::D4,
            "d5" => Reg::D5,
            "d6" => Reg::D6,
            "d7" => Reg::D7,
            "a0" => Reg::A0,
            "a1" => Reg::A1,
            "a2" => Reg::A2,
            "a3" => Reg::A3,
            "a4" => Reg::A4,
            "a5" => Reg::A5,
            "a6" => Reg::A6,
            "a7" | "sp" => Reg::A7,
            _ => return None,
        })
    }
}

impl CodeBuf {
    /// The empty code fragment — the `Code` monoid's identity (`Code.empty`).
    pub fn empty() -> Self {
        CodeBuf { items: Vec::new() }
    }

    /// The monoid `++`: append `b`'s items after `a`'s.
    pub fn concat(mut a: CodeBuf, b: CodeBuf) -> CodeBuf {
        a.items.extend(b.items);
        a
    }

    /// Push one item onto the fragment.
    pub fn push(&mut self, item: CodeItem) {
        self.items.push(item);
    }
}

impl fmt::Display for Width {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Width::B => "b",
            Width::W => "w",
            Width::L => "l",
            Width::S => "s",
        })
    }
}

impl fmt::Display for Cc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Cc::T => "t",
            Cc::F => "f",
            Cc::Hi => "hi",
            Cc::Ls => "ls",
            Cc::Cc => "cc",
            Cc::Cs => "cs",
            Cc::Ne => "ne",
            Cc::Eq => "eq",
            Cc::Vc => "vc",
            Cc::Vs => "vs",
            Cc::Pl => "pl",
            Cc::Mi => "mi",
            Cc::Ge => "ge",
            Cc::Lt => "lt",
            Cc::Gt => "gt",
            Cc::Le => "le",
        })
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Reg::D0 => "d0",
            Reg::D1 => "d1",
            Reg::D2 => "d2",
            Reg::D3 => "d3",
            Reg::D4 => "d4",
            Reg::D5 => "d5",
            Reg::D6 => "d6",
            Reg::D7 => "d7",
            Reg::A0 => "a0",
            Reg::A1 => "a1",
            Reg::A2 => "a2",
            Reg::A3 => "a3",
            Reg::A4 => "a4",
            Reg::A5 => "a5",
            Reg::A6 => "a6",
            Reg::A7 => "a7",
        })
    }
}

impl Value {
    /// A short, stable type name for use in type-mismatch diagnostics.
    ///
    /// A [`Value::Typed`] reports the generic `"typed"` here (this method's
    /// `&'static str` return cannot carry the newtype's owned, dynamic name);
    /// the type-aware arithmetic diagnostics that actually need the nominal
    /// name (cross-type mix, scale mismatch) format it via
    /// [`Ty::describe`](crate::layout::Ty::describe) directly.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Array(_) => "array",
            Value::Range { .. } => "range",
            Value::Struct { .. } => "struct",
            Value::Enum { .. } => "enum",
            Value::Tuple(_) => "tuple",
            Value::Unit => "unit",
            Value::Lambda { .. } => "lambda",
            Value::FnRef(_) => "fn",
            Value::Typed { .. } => "typed",
            Value::Data(_) => "data",
            Value::Code(_) => "code",
            Value::Width(_) => "width",
            Value::Cc(_) => "cc",
            Value::Reg(_) => "reg",
            Value::Label(_) => "label",
            Value::LinkExpr(_) => "link-expr",
            Value::Poison => "poison",
        }
    }

    /// The stored `i128` for a value that erases to a bare integer — either a
    /// [`Value::Int`] or a [`Value::Typed`] wrapping one. Used at every site
    /// that needs a raw integer from a value that may be nominally typed (array
    /// lengths, bitfield field values, string interpolation of a number, the
    /// argument to `Name(x)`), honoring the "`Typed` erases to its stored int"
    /// principle (§8.3). Returns `None` for any non-integer value.
    pub fn as_stored_int(&self) -> Option<i128> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Typed { val, .. } => val.as_stored_int(),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(x) => {
                // Whole finite floats print with a trailing `.0` so they read
                // as floats (`2.0`) and are visually distinct from ints.
                if x.is_finite() && x.fract() == 0.0 {
                    write!(f, "{x:.1}")
                } else {
                    write!(f, "{x}")
                }
            }
            // Strings render quoted so they delimit cleanly in diagnostics and
            // inside array/struct renderings.
            Value::Str(s) => write!(f, "{s:?}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Array(elems) => {
                f.write_str("[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{e}")?;
                }
                f.write_str("]")
            }
            Value::Range { lo, hi } => write!(f, "{lo}..{hi}"),
            Value::Struct { ty_name, fields } => {
                write!(f, "{ty_name}{{")?;
                for (i, (name, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}: {v}")?;
                }
                f.write_str("}")
            }
            Value::Enum { ty_name, variant, payload } => {
                write!(f, "{ty_name}.{variant}")?;
                if !payload.is_empty() {
                    f.write_str("(")?;
                    for (i, v) in payload.iter().enumerate() {
                        if i > 0 {
                            f.write_str(", ")?;
                        }
                        write!(f, "{v}")?;
                    }
                    f.write_str(")")?;
                }
                Ok(())
            }
            Value::Tuple(elems) => {
                f.write_str("(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{e}")?;
                }
                f.write_str(")")
            }
            Value::Unit => f.write_str("()"),
            Value::Lambda { .. } => f.write_str("<lambda>"),
            Value::FnRef(name) => write!(f, "<fn {name}>"),
            // A typed value renders as its inner (stored) value — the nominal
            // type shows in diagnostics, not in the interpolated/printed value.
            Value::Typed { val, .. } => write!(f, "{val}"),
            Value::Data(buf) => write!(f, "data[{} bytes]", buf.size),
            Value::Code(buf) => write!(f, "code[{} items]", buf.items.len()),
            Value::Width(w) => write!(f, "{w}"),
            Value::Cc(c) => write!(f, "{c}"),
            Value::Reg(r) => write!(f, "{r}"),
            // A label renders as its bare symbol name (no quotes — it is not a
            // string): `<label init>` distinguishes it in diagnostics.
            Value::Label(n) => write!(f, "<label {n}>"),
            // A link-time value has no comptime integer to render — a deferred
            // guard message folds it at link (D-H.5); this `Display` is only a
            // diagnostic fallback for a `LinkExpr` that reaches an unexpected site.
            Value::LinkExpr(_) => f.write_str("<link-time value>"),
            Value::Poison => f.write_str("<poison>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i(n: i128) -> Value {
        Value::Int(n)
    }

    #[test]
    fn display_int() {
        assert_eq!(i(42).to_string(), "42");
        assert_eq!(Value::Int(-7).to_string(), "-7");
    }

    #[test]
    fn display_float_fractional_and_whole() {
        assert_eq!(Value::Float(1.5).to_string(), "1.5");
        // A whole float prints with a trailing `.0` (chosen contract).
        assert_eq!(Value::Float(2.0).to_string(), "2.0");
    }

    #[test]
    fn display_str_is_quoted() {
        assert_eq!(Value::Str("hi".to_string()).to_string(), "\"hi\"");
    }

    #[test]
    fn display_bool() {
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Bool(false).to_string(), "false");
    }

    #[test]
    fn display_array() {
        let v = Value::Array(vec![i(1), i(2), i(3)]);
        assert_eq!(v.to_string(), "[1, 2, 3]");
        assert_eq!(Value::Array(vec![]).to_string(), "[]");
    }

    #[test]
    fn display_range() {
        assert_eq!(Value::Range { lo: 0, hi: 256 }.to_string(), "0..256");
    }

    #[test]
    fn display_tuple() {
        let v = Value::Tuple(vec![i(1), Value::Bool(true)]);
        assert_eq!(v.to_string(), "(1, true)");
    }

    #[test]
    fn display_struct() {
        let v = Value::Struct {
            ty_name: "Point".to_string(),
            fields: vec![("x".to_string(), i(1)), ("y".to_string(), i(2))],
        };
        assert_eq!(v.to_string(), "Point{x: 1, y: 2}");
    }

    #[test]
    fn display_enum_nullary_and_payload() {
        let bare = Value::Enum {
            ty_name: "Dir".to_string(),
            variant: "Up".to_string(),
            payload: vec![],
        };
        assert_eq!(bare.to_string(), "Dir.Up");
        let with = Value::Enum {
            ty_name: "Opt".to_string(),
            variant: "Some".to_string(),
            payload: vec![i(5)],
        };
        assert_eq!(with.to_string(), "Opt.Some(5)");
    }

    #[test]
    fn display_unit_poison() {
        assert_eq!(Value::Unit.to_string(), "()");
        assert_eq!(Value::Poison.to_string(), "<poison>");
    }

    #[test]
    fn display_lambda() {
        let lam = Value::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(Expr::Path(crate::ast::Path {
                segments: vec!["x".to_string()],
                span: dummy_span(),
            })),
            captured: Env::new(),
        };
        assert_eq!(lam.to_string(), "<lambda>");
    }

    #[test]
    fn display_fn_ref() {
        assert_eq!(Value::FnRef("dbl".to_string()).to_string(), "<fn dbl>");
    }

    #[test]
    fn display_label() {
        // A label renders as its bare symbol name — no quotes (it is not a
        // string), distinct from a `Str` in diagnostics.
        assert_eq!(Value::Label("init".to_string()).to_string(), "<label init>");
        assert_eq!(
            Value::Label("pitcher_plant.init".to_string()).to_string(),
            "<label pitcher_plant.init>"
        );
    }

    #[test]
    fn type_names() {
        assert_eq!(i(1).type_name(), "int");
        assert_eq!(Value::Float(1.0).type_name(), "float");
        assert_eq!(Value::Str(String::new()).type_name(), "string");
        assert_eq!(Value::Bool(true).type_name(), "bool");
        assert_eq!(Value::Array(vec![]).type_name(), "array");
        assert_eq!(Value::Range { lo: 0, hi: 1 }.type_name(), "range");
        assert_eq!(
            Value::Struct { ty_name: "T".into(), fields: vec![] }.type_name(),
            "struct"
        );
        assert_eq!(
            Value::Enum { ty_name: "T".into(), variant: "V".into(), payload: vec![] }
                .type_name(),
            "enum"
        );
        assert_eq!(Value::Tuple(vec![]).type_name(), "tuple");
        assert_eq!(Value::Unit.type_name(), "unit");
        assert_eq!(
            Value::Lambda {
                params: vec![],
                body: Box::new(Expr::Int(0, dummy_span())),
                captured: Env::new(),
            }
            .type_name(),
            "lambda"
        );
        assert_eq!(Value::FnRef("f".into()).type_name(), "fn");
        assert_eq!(Value::Label("init".into()).type_name(), "label");
        assert_eq!(Value::Data(DataBuf::empty()).type_name(), "data");
        assert_eq!(Value::Poison.type_name(), "poison");
    }

    #[test]
    fn databuf_monoid_and_display() {
        let mut a = DataBuf::empty();
        a.push(Cell::Scalar { value: 5, width: 1, signed: false, le: false });
        assert_eq!(a.size, 1);
        let mut b = DataBuf::empty();
        b.push(Cell::Bytes(vec![1, 2, 3]));
        let c = DataBuf::concat(a, b);
        assert_eq!(c.size, 4);
        assert_eq!(c.cells.len(), 2);
        assert_eq!(Value::Data(c).to_string(), "data[4 bytes]");
    }

    fn dummy_span() -> sigil_span::Span {
        sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }
}
