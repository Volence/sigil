//! Spanned AST for .emp (Spec 2 §10 surface). Pure data — no semantics.
use sigil_span::Span;

/// A whole parsed `.emp` source file: its module header, module-level
/// attributes, and top-level items.
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// The mandatory `module x.y` (or `module x.y in section`) header.
    pub module: ModuleDecl,
    /// Module-level attributes: `@as_compat`, `@allow(group)`.
    pub attrs: Vec<Attr>,
    /// Top-level declarations following the header.
    pub items: Vec<Item>,
    /// `///` doc runs attached to items (S2-D11(d)), keyed by [`item_span`].
    pub docs: Vec<DocEntry>,
}

impl File {
    /// The doc text attached to the item whose [`item_span`] is `span`, if any.
    pub fn docs_for(&self, span: Span) -> Option<&str> {
        self.docs.iter().find(|d| d.item_span == span).map(|d| d.text.as_str())
    }
}

/// An `@name(args...)` attribute attached to a module, item, or field.
#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    /// The attribute name, e.g. `as_compat`, `allow`.
    pub name: String,
    /// Attribute arguments, e.g. the `naming.pascal` in `@allow(naming.pascal)`.
    pub args: Vec<Expr>,
    /// Full span of the attribute, `@` through the closing `)` (or the name).
    pub span: Span,
}

/// A dotted path: `engine.gfx.ArtTile`, `none`, a single name, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Dot-separated path segments, in order.
    pub segments: Vec<String>,
    /// Span covering the whole path.
    pub span: Span,
}

/// The `module x.y` or `module x.y in section` header that must open every file.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    /// The dotted module path.
    pub path: Path,
    /// `module x.y in obj_bank` — the section this module's code belongs to.
    pub in_section: Option<String>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A single top-level declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// `use ...` import declaration.
    Use(UseDecl),
    /// `const ...` declaration.
    Const(ConstDecl),
    /// `equ ...` declaration (R-T0.2): an assembler equate. Distinct from
    /// [`Item::Const`] — its whole purpose is to become a link-level symbol
    /// (that emission is a later task); deliberately not folded into
    /// `pub const` so existing `pub const` semantics never silently change.
    Equ(EquDecl),
    /// `enum ...` declaration.
    Enum(EnumDecl),
    /// `bitfield ...` declaration.
    Bitfield(BitfieldDecl),
    /// `struct ...` declaration.
    Struct(StructDecl),
    /// `offsets ...` declaration.
    Offsets(OffsetsDecl),
    /// `dispatch ...` declaration.
    Dispatch(DispatchDecl),
    /// `vars ...` declaration.
    Vars(VarsDecl),
    /// `data ...` declaration.
    Data(DataDecl),
    /// `proc ...` declaration.
    Proc(ProcDecl),
    /// `script ...` declaration (Plan 7 #9b).
    Script(ScriptDecl),
    /// `comptime fn ...` declaration.
    ComptimeFn(ComptimeFnDecl),
    /// `section ...` declaration.
    Section(SectionDecl),
    /// `newtype ...` declaration.
    Newtype(NewtypeDecl),
    /// An item-position `ensure(...)` / `ensure_fatal(...)` guard (§6.5, D5.1).
    Ensure(EnsureDecl),
    /// An `align N` item (D2.29, §4.8): pad to the next multiple of `N`.
    Align(AlignDecl),
    /// A `comptime test "name" { … }` block (S2-D11(a)): colocated comptime
    /// tests, stripped from emission, run by `sigil test`.
    ComptimeTest(ComptimeTestDecl),
}

/// A `comptime test "name" [(expect_error: "[diag.id]")] { … }` block
/// (S2-D11(a), Zig-style): the comptime-fn feedback loop — today's only
/// alternative is a full ROM build + byte-diff. Stripped from emission
/// ALWAYS (zero bytes, zero cost in normal builds); `sigil test` evaluates
/// the body as a comptime block. The `expect_error` variant asserts the body
/// DIAGNOSES (a "this must not compile" test, absorbing research T3-g
/// `EXPECT`): pass iff some body diagnostic contains the id substring, and
/// the captured diagnostics are then swallowed.
#[derive(Debug, Clone, PartialEq)]
pub struct ComptimeTestDecl {
    /// The test's display name (a string literal — tests aren't symbols).
    pub name: String,
    /// `(expect_error: "[diag.id]")` — the body must diagnose this id.
    pub expect_error: Option<String>,
    /// The comptime statement body (the comptime-fn body grammar).
    pub body: Vec<Stmt>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// An `align N` item (D2.29, §4.8): pads the current position to the next
/// multiple of `N` with `$00` fill — always the author's explicit,
/// byte-visible act (the compiler never inserts implicit alignment).
#[derive(Debug, Clone, PartialEq)]
pub struct AlignDecl {
    /// The alignment (must comptime-evaluate to a positive int).
    pub n: Expr,
    /// Span of the whole item.
    pub span: Span,
}

/// The span of an item's own declaration (the decl struct's `span` field) —
/// the key `File::docs_for` looks docs up by (S2-D11(d)).
pub fn item_span(item: &Item) -> Span {
    match item {
        Item::Use(d) => d.span,
        Item::Const(d) => d.span,
        Item::Equ(d) => d.span,
        Item::Enum(d) => d.span,
        Item::Bitfield(d) => d.span,
        Item::Struct(d) => d.span,
        Item::Offsets(d) => d.span,
        Item::Dispatch(d) => d.span,
        Item::Vars(d) => d.span,
        Item::Data(d) => d.span,
        Item::Proc(d) => d.span,
        Item::Script(d) => d.span,
        Item::ComptimeFn(d) => d.span,
        Item::Section(d) => d.span,
        Item::Newtype(d) => d.span,
        Item::Ensure(d) => d.span,
        Item::Align(d) => d.span,
        Item::ComptimeTest(d) => d.span,
    }
}

/// A `///` doc-comment run attached to one item (S2-D11(d)): parse-and-attach
/// only — surfacing (hover, rendered docs) is the Spec-3 seam.
#[derive(Debug, Clone, PartialEq)]
pub struct DocEntry {
    /// The documented item's own span ([`item_span`]) — the lookup key.
    pub item_span: Span,
    /// The joined doc text (one line per `///`, `\n`-separated, one optional
    /// leading space per line already stripped by the lexer).
    pub text: String,
}

/// An item-position guard: `ensure(cond, "msg")` / `ensure_fatal(cond, "msg")`
/// between items. `call` is the WHOLE call expression — evaluation reuses the
/// evaluator's guard special-case (arity, interpolation, `aborted`).
#[derive(Debug, Clone, PartialEq)]
pub struct EnsureDecl {
    /// True for `ensure_fatal`.
    pub fatal: bool,
    /// The full `ensure(...)` call expression.
    pub call: Expr,
    /// Span of the whole item.
    pub span: Span,
}

/// A `use base.{a, b}` / `use base.*` / `use base` import.
#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    /// The dotted base path being imported from (or wholly imported).
    pub base: Path,
    /// What is imported from `base`.
    pub names: UseNames,
    /// Span of the whole declaration.
    pub span: Span,
}

/// The imported-name portion of a [`UseDecl`].
#[derive(Debug, Clone, PartialEq)]
pub enum UseNames {
    /// `use base` — import the whole path as one name.
    Whole,
    /// `use base.*` — glob-import everything under `base`.
    Glob,
    /// `use base.{a, b, c}` — import exactly these names from `base`.
    List(Vec<String>),
}

/// A `const NAME: Ty = value` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    /// Whether this const is exported (`pub const`).
    pub public: bool,
    /// The constant's name.
    pub name: String,
    /// Optional explicit type annotation.
    pub ty: Option<Type>,
    /// The constant's value expression.
    pub value: Expr,
    /// Span of the whole declaration.
    pub span: Span,
}

/// An `equ NAME = expr` declaration (R-T0.2): an assembler equate — an item
/// whose ENTIRE purpose is to become a link-level symbol. Grammar mirrors
/// [`ConstDecl`] minus the type annotation (equ values are untyped comptime
/// ints or link-time expressions; Task 3 adds the `[equ.value]` restriction
/// diagnostic at lowering). `pub equ` makes it module-visible exactly like
/// other `pub` items.
#[derive(Debug, Clone, PartialEq)]
pub struct EquDecl {
    /// Whether this equ is exported (`pub equ`).
    pub is_pub: bool,
    /// The equate's name — the symbol it becomes at link (Task 3).
    pub name: String,
    /// The equate's value expression.
    pub value: Expr,
    /// Span of the whole declaration.
    pub span: Span,
}

/// An `enum Name: repr { variants... }` declaration (or `comptime enum Name
/// { variants... }`, whose variants may carry payload types instead of an
/// explicit value, and which needs no repr).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    /// Whether this enum is exported (`pub enum`).
    pub public: bool,
    /// Whether this is a `comptime enum` (payload-carrying, no explicit
    /// discriminant type required) rather than a plain repr-backed enum.
    pub comptime: bool,
    /// The enum's name.
    pub name: String,
    /// The underlying representation type, e.g. `u8` in `enum Anim: u8`.
    /// Always `Some` for a plain enum (required); optional for `comptime enum`.
    pub repr: Option<Type>,
    /// The enum's variants, in declaration order.
    pub variants: Vec<EnumVariant>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A single variant within an [`EnumDecl`]: `Idle = 0` or `Literal(string)`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    /// The variant's name.
    pub name: String,
    /// An explicit discriminant value, e.g. the `0` in `Idle = 0`.
    pub value: Option<Expr>,
    /// Payload types, e.g. `[Named(string)]` in `Literal(string)` (empty for
    /// a plain, non-payload-carrying variant).
    pub payload: Vec<Type>,
    /// Span of the whole variant.
    pub span: Span,
}

/// A `bitfield Name: repr { fields... }` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct BitfieldDecl {
    /// Whether this bitfield is exported (`pub bitfield`).
    pub public: bool,
    /// The bitfield's name.
    pub name: String,
    /// The underlying representation type.
    pub repr: Type,
    /// The bitfield's fields, in declaration order.
    pub fields: Vec<BitfieldField>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A single field within a [`BitfieldDecl`]: `tile: 11 @ 0`.
#[derive(Debug, Clone, PartialEq)]
pub struct BitfieldField {
    /// The field's name.
    pub name: String,
    /// The field's width in bits.
    pub bits: u32,
    /// Explicit bit-position anchor, e.g. the `0` in `tile: 11 @ 0`.
    pub anchor: Option<u32>,
    /// Span of the whole field.
    pub span: Span,
}

/// A `struct Name (size: expr) { fields... }` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    /// Whether this struct is exported (`pub struct`).
    pub public: bool,
    /// The struct's name.
    pub name: String,
    /// Explicit total size, e.g. `(size: 0x50)`.
    pub size: Option<Expr>,
    /// The struct's fields, in declaration order.
    pub fields: Vec<StructField>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A single field within a [`StructDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// The field's name.
    pub name: String,
    /// The field's type.
    pub ty: Type,
    /// Explicit byte offset, e.g. the `0x2E` in `@ 0x2E`.
    pub offset: Option<Expr>,
    /// Default value, e.g. the `0` in `= 0`.
    pub default: Option<Expr>,
    /// Span of the whole field.
    pub span: Span,
}

/// An `offsets Name { Variant: target, ... }` block: a bidirectional offset
/// table. Forward: emits `dc.w target - Name` per member. Reverse: introduces
/// the comptime ordinal constants `Name.Variant` (0-based) and `Name.count`.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetsDecl {
    /// Whether this offsets block is exported (`pub offsets`).
    pub public: bool,
    /// The offset table's name.
    pub name: String,
    /// The table's members, in declaration order.
    pub members: Vec<OffsetsMember>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// One `Variant: target` entry of an [`OffsetsDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetsMember {
    /// The ordinal's name (`Name.Variant`).
    pub name: String,
    /// Where this entry's word points (§4.7): a by-reference label, or an
    /// inline body co-located in the block (the [`DispatchTarget`] precedent).
    pub target: OffsetsTarget,
    /// Span of the whole member.
    pub span: Span,
}

/// An `offsets` member's target (§4.7 mixed form).
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetsTarget {
    /// `Name: label` — a reference to a label defined elsewhere (the shipped
    /// form; keeps shared/cross-module targets).
    Ref(Expr),
    /// `Name: Type = value` — an INLINE body, the exact `data`-item shape
    /// (the declared length stays the terminator guard). Emitted after the
    /// table in declaration order under a hidden hygienic label; the table
    /// word targets it.
    Inline(Type, Expr),
}

/// A `dispatch Name (encoding: E) { Member: target, ... }` block: an
/// encoding-agnostic typed state-dispatch table (D6.B1). Forward: emits a
/// code-pointer table per `encoding` (later task). Reverse: introduces the
/// pre-scaled comptime ordinal constants `Name.Member` and `Name.count`
/// (D6.B3, later task). The member grammar deliberately mirrors
/// [`OffsetsDecl`]'s `Name: target` shape; `Member: { ... }` (inline body /
/// scripted state) is the 9a inline-body form: sugar for an anonymous
/// per-member proc — a hygienic label sharing the same encoding row as a
/// named target, with NO state/yield semantics (D9.1).
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchDecl {
    /// Whether this dispatch table is exported (`pub dispatch`).
    pub public: bool,
    /// The dispatch table's name.
    pub name: String,
    /// The table's emission/ordinal-scaling encoding (required — no default).
    pub encoding: DispatchEncoding,
    /// The table's members, in declaration order.
    pub members: Vec<DispatchMember>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// The `(encoding: E)` knob of a [`DispatchDecl`] (D6.B2). Exactly two
/// encodings in v1; the construct enables encodings and imposes none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchEncoding {
    /// `dc.w member_target - Name` per member (signed-word range-checked,
    /// reuses the `offsets` RelOffset machinery). Ordinals pre-scaled ×2.
    WordOffsets,
    /// `dc.l target` per member (Abs32 fixups). Ordinals pre-scaled ×4.
    LongPtrs,
}

impl DispatchEncoding {
    /// The ordinal pre-scale factor (D6.B3): `Name.Member` = ordinal × this
    /// factor. Consumed by a later task (reverse-constant lowering).
    pub fn scale(&self) -> i128 {
        match self {
            DispatchEncoding::WordOffsets => 2,
            DispatchEncoding::LongPtrs => 4,
        }
    }
}

/// One `Member: target` / `Member: { … }` entry of a [`DispatchDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchMember {
    /// The member's name (`Name.Member`).
    pub name: String,
    /// The member's right-hand side: a label reference or an inline body.
    pub target: DispatchTarget,
    /// Span of the whole member.
    pub span: Span,
}

/// A dispatch member's right-hand side (Plan 7 #9a — D9.1).
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchTarget {
    /// `Member: target` — a label reference (path / string / comptime expr).
    Label(Expr),
    /// `Member: { … }` — an inline body: sugar for an anonymous per-member
    /// proc (hygienic label, same encoding row as a named target). NO
    /// state/yield semantics — that is 9b's `script` construct (D9.2).
    Body(Vec<AsmStmt>),
}

/// A `vars [name:] region { fields... }` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct VarsDecl {
    /// Whether this vars block is exported (`pub vars`).
    pub public: bool,
    /// `vars upper_ram { .. }` → name None, region ["upper_ram"].
    /// `vars PitcherPlantV: sst_custom { .. }` → name Some("PitcherPlantV"), region ["sst_custom"].
    /// `vars X: Sst.sst_custom { .. }` → name Some("X"), region ["Sst", "sst_custom"] (dotted
    /// window path, disambiguating which struct's byte-array field the overlay targets).
    pub name: Option<String>,
    /// The memory region (or dotted window path) this block is allocated into.
    pub region: Vec<String>,
    /// The block's fields, in declaration order.
    pub fields: Vec<VarsField>,
    /// The window binding resolved at the overlay's DEFINITION site, present ONLY
    /// on the clone injected into a CONSUMER module by `use`/prelude (Plan 7 #8).
    /// A bare `region: [w]` window is otherwise re-scanned in whatever namespace
    /// the overlay is queried in — so a consumer could rebind it to an unrelated
    /// same-named window field, or a colliding consumer struct could poison the
    /// binding with a spurious ambiguity. Stamping the resolved window here makes
    /// the overlay self-contained: the consumer uses this binding verbatim and
    /// never re-runs window resolution. `None` on every author-written decl (the
    /// defining module resolves against its own namespace, as before).
    pub resolved_window: Option<ResolvedWindow>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A window binding resolved at an overlay's definition site (Plan 7 #8), stamped
/// onto the overlay clone injected into a consumer module so the window offset /
/// size travel with the overlay instead of being re-derived from the consumer's
/// (possibly different, or absent) structs.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWindow {
    /// The base struct the window belongs to (in the DEFINING module).
    pub base_struct: String,
    /// The window field's byte offset within the base struct.
    pub window_offset: i128,
    /// The window field's byte size (its `N` in `[u8; N]`).
    pub window_size: i128,
}

/// A single field within a [`VarsDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct VarsField {
    /// The field's name.
    pub name: String,
    /// The field's type.
    pub ty: Type,
    /// Explicit alignment, e.g. the `256` in `@align(256)`.
    pub align: Option<Expr>,
    /// Span of the whole field.
    pub span: Span,
}

/// A `data NAME: Ty = value` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct DataDecl {
    /// Whether this data item is exported (`pub data`).
    pub public: bool,
    /// The data item's name.
    pub name: String,
    /// Optional explicit type annotation; inferable when the literal names its type.
    pub ty: Option<Type>,
    /// Optional `(max_size: expr)` capacity bound (D5.4): the checked buffer's
    /// byte length must not exceed it. Always-on; overflow is an error.
    pub max_size: Option<Expr>,
    /// The data item's value expression.
    pub value: Expr,
    /// Span of the whole declaration.
    pub span: Span,
    /// A cross-module TYPE-ONLY injection (D-PP.5), NOT a source construct:
    /// always `false` from the parser. The resolver sets it on the clone of a
    /// `pub data` item of struct type it prepends to a consumer module, so the
    /// consumer's evaluator learns the item's struct type (for `Item.field`
    /// field-address operands) WITHOUT emitting the item's bytes a second time.
    /// Lowering skips a `type_only` item entirely (no label, no bytes); the
    /// evaluator indexes only its `(name → struct)` binding, never its `value`.
    pub type_only: bool,
}

/// A `proc name(params...) { body... }` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcDecl {
    /// Whether this proc is exported (`pub proc`).
    pub public: bool,
    /// The proc's name.
    pub name: String,
    /// Parameters as `(name, type, span)`, e.g. `(a0, *Sst)`.
    pub params: Vec<(String, Type, Span)>,
    /// Registers this proc clobbers. `None` = no contract declared (legal —
    /// half-ported files); `Some(vec![])` = the explicit `clobbers()` form,
    /// "verified: touches nothing" (Volence ruling, tranche 3) — the lint
    /// then flags ANY register write.
    pub clobbers: Option<Vec<String>>,
    /// Registers this proc preserves (S2-D6b syntactic slice), as declared
    /// reglist segments: `preserves(d0-d1/a0)` → `[("d0", Some("d1")),
    /// ("a0", None)]`. Register validity is a lowering-time check
    /// (`[proc.preserves-invalid]`), not a parse-time one, mirroring
    /// `clobbers`.
    pub preserves: Vec<(String, Option<String>)>,
    /// Registers this proc RETURNS — the third partition member (S2-D6e). `None`
    /// = no `out(...)` declared (legal); `Some(vec![])` = the explicit `out()`
    /// form (declares "returns nothing"). Mirrors `clobbers`'s plain register
    /// list (NOT `preserves`' movem-reglist form — outputs are named single
    /// registers, never movem ranges). Output registers join `check_clobbers`'
    /// `allowed` set (a result-register write is not `[proc.clobber-undeclared]`);
    /// register validity is a lowering-time check (`[proc.out-invalid]`), not a
    /// parse-time one, mirroring `clobbers`/`preserves`.
    pub out: Option<Vec<String>>,
    /// The proc this one falls into, if any.
    pub falls_into: Option<String>,
    /// The proc's assembly body.
    pub body: Vec<AsmStmt>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A `script name(params) (encoding: E) [shows label] { body }` declaration
/// (Plan 7 #9b — D9.2/D9.6). A script is a coroutine: `yield` saves a typed
/// resume point (the object's next-frame state) and exits through the
/// per-frame epilogue; the compiler emits a HIDDEN dispatch-encoded resume
/// table at the script's name, followed by the body's resume segments.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptDecl {
    /// Whether the script's table label is exported (`pub script`).
    pub public: bool,
    /// The script's name — the hidden table's base label (the engine handle).
    pub name: String,
    /// Parameters, exactly as [`ProcDecl::params`] (typed register bindings).
    pub params: Vec<(String, Type, Span)>,
    /// The hidden table's emission/ordinal-scaling encoding (required — the
    /// engine dispatcher indexes the table, so this is engine contract).
    pub encoding: DispatchEncoding,
    /// The declared per-frame epilogue (`shows <label>`), overridable per
    /// yield site. A bare `yield` with no epilogue in scope is an error.
    pub epilogue: Option<ScriptLabel>,
    /// The script's statements.
    pub body: Vec<ScriptStmt>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A statement within a `script` body (R9b.1).
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptStmt {
    /// Any ordinary proc-body statement (label / instruction / comptime call).
    Asm(AsmStmt),
    /// `loop { … }` — unconditional loop (hidden label + `jbra` back).
    Loop {
        /// The loop's statements.
        body: Vec<ScriptStmt>,
        /// Span of the whole loop.
        span: Span,
    },
    /// `yield` / `yield shows <label>` / `yield .label` — save a resume
    /// point, exit via the per-frame epilogue (D9.6 + the D2.30 batch).
    Yield {
        /// `yield shows <label>` — per-site epilogue override (D2.30(a));
        /// `None` uses the script's `shows` declaration.
        epilogue: Option<ScriptLabel>,
        /// `yield .label` — the NAMED RESUME (D2.30(b)): "frame over; next
        /// frame, continue at `.label`". Stores the target segment's ordinal
        /// instead of minting a resume point at this site.
        resume: Option<ScriptLabel>,
        /// Span of the statement.
        span: Span,
    },
    /// `wait_frames #N, <slot>` (D2.30(c)) — the declarative PURE park:
    /// store N into the named timer slot, then a hidden per-frame decrement
    /// plus self-resuming yield. Pure compiler expansion of the documented
    /// tick idiom — no dispatcher protocol (value-carrying yields stay
    /// 9c-gated).
    WaitFrames {
        /// The park length (an immediate; a comptime-visible 0 is refused).
        n: Expr,
        /// The timer slot operand — named explicitly at the site (tenet 5:
        /// no hidden state; different objects park on different fields).
        slot: Operand,
        /// Span of the statement.
        span: Span,
    },
}

/// An epilogue label reference: `Draw_Sprite` (global) or `.rearm` (local).
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptLabel {
    /// The label name (without the leading dot for locals).
    pub name: String,
    /// True for the `.name` (proc-local) form.
    pub local: bool,
    /// Span of the reference.
    pub span: Span,
}

/// A `comptime fn name(params...) -> ret { body... }` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ComptimeFnDecl {
    /// Whether this comptime fn is exported (`pub comptime fn`).
    pub public: bool,
    /// The function's name.
    pub name: String,
    /// Parameters as `(name, type, span)`.
    pub params: Vec<(String, Type, Span)>,
    /// Optional explicit return type.
    pub ret: Option<Type>,
    /// The function's comptime statement body.
    pub body: Vec<Stmt>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A `section name (attrs...) { items... }` declaration, or its bare form.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionDecl {
    /// The section's name.
    pub name: String,
    /// Section attributes as `(name, value)`, e.g. `(cpu: z80, vma: $0000)`.
    pub attrs: Vec<(String, Expr)>,
    /// The section's nested items; empty for the bare (non-block) declaration.
    pub items: Vec<Item>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A type expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// A named type: `u8`, `ObjDef`, `engine.gfx.ArtTile`.
    Named(Path),
    /// A pointer type: `*Sst`.
    Ptr(Box<Type>),
    /// An array type with element type and length expression: `[i8; 256]`.
    Array(Box<Type>, Expr),
    /// A tuple type: `(Data, Code)`.
    Tuple(Vec<Type>),
    /// A fixed-point type `fixed<I, F>`: `I` integer bits, `F` fraction bits.
    Fixed {
        /// Integer-part bit width.
        i: u32,
        /// Fraction-part bit width.
        f: u32,
    },
    /// A refined type `T where LO..HI`: `T` narrowed to the range given by the
    /// two expressions. The bounds are INCLUSIVE on BOTH ends (D-P3.8) — e.g.
    /// `VramTile where 0..2047` covers all 2048 tiles, and `set_pal(64)` on a
    /// `where 0..63` param fails with "64 not in 0..63". This deliberately
    /// diverges from [`Expr::Range`]'s half-open (inclusive-lo, exclusive-hi)
    /// iteration semantics; a later `check_in_range` must use `<=` on the hi
    /// bound, not `<`.
    Refined(Box<Type>, Expr, Expr),
}

/// A `newtype Name = Underlying [where LO..HI]` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct NewtypeDecl {
    /// Whether this newtype is exported (`pub newtype`).
    pub public: bool,
    /// The newtype's name.
    pub name: String,
    /// The underlying type it wraps.
    pub underlying: Type,
    /// The optional `where LO..HI` range refinement, as `(lo, hi)`.
    pub refine: Option<(Expr, Expr)>,
    /// Span of the whole declaration.
    pub span: Span,
}

// ---- expressions -------------------------------------------------------

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// An integer literal.
    Int(i64, Span),
    /// A floating-point literal.
    Float(f64, Span),
    /// A string literal.
    Str(String, Span),
    /// A path expression: names, enum paths, `none`.
    Path(Path),
    /// A proc-LOCAL label reference `.name` in expression position (F2, tranche
    /// 7). ONLY meaningful in a label-value context (a call argument): it
    /// resolves through the ENCLOSING proc body's hygienic local-label naming to
    /// a [`Value::Label`](crate::value::Value::Label) carrying the SAME mangled
    /// link symbol a `.name:` written directly in that proc gets. In any pure
    /// comptime expression position (`const x = .foo`) it is a loud error — the
    /// form never leaks a silent Label into ordinary expressions. The `String` is
    /// the bare label name WITHOUT the leading dot.
    LocalLabel(String, Span),
    /// A unary operation.
    Unary {
        /// The unary operator.
        op: UnOp,
        /// The operand.
        expr: Box<Expr>,
        /// Span of the whole expression.
        span: Span,
    },
    /// A binary operation.
    Binary {
        /// The binary operator.
        op: BinOp,
        /// The left-hand operand.
        lhs: Box<Expr>,
        /// The right-hand operand.
        rhs: Box<Expr>,
        /// Span of the whole expression.
        span: Span,
    },
    /// A function/comptime-fn call.
    Call {
        /// The called path.
        callee: Path,
        /// Call arguments.
        args: Vec<Arg>,
        /// Span of the whole expression.
        span: Span,
    },
    /// A struct literal: `Ty { field: value, ... }`. Every declared field
    /// must be NAMED (S2-D13(h), checkpoint ruling 2026-07-09): a field whose
    /// declared default should apply is written `field: default` — elision
    /// is per-field and self-documenting (`Expr::Default`); there is no bulk
    /// marker (the `..` form was built and retired at the checkpoint — the
    /// page couldn't say WHICH fields it covered; re-ledgered for a struct
    /// with enough defaults that per-field `default` reads as noise).
    StructLit {
        /// The struct's type path.
        ty: Path,
        /// Field initializers as `(name, value)`; a value may be
        /// [`Expr::Default`].
        fields: Vec<(String, Expr)>,
        /// Span of the whole expression.
        span: Span,
    },
    /// The contextual `default` marker in struct-literal field-value position
    /// (`vel: default`): "this field takes its DECLARED default". An error
    /// anywhere else, and an error on a field with no declared default.
    Default(Span),
    /// An array literal: `[e1, e2, ...]`.
    ArrayLit {
        /// The array's elements.
        elems: Vec<Expr>,
        /// Span of the whole expression.
        span: Span,
    },
    /// A tuple literal: `(e1, e2, ...)`.
    TupleLit {
        /// The tuple's elements.
        elems: Vec<Expr>,
        /// Span of the whole expression.
        span: Span,
    },
    /// A range expression: `0..256`.
    Range {
        /// The inclusive lower bound.
        lo: Box<Expr>,
        /// The exclusive upper bound.
        hi: Box<Expr>,
        /// Span of the whole expression.
        span: Span,
    },
    /// An `if cond { then... } [else { els... }]` expression.
    If {
        /// The condition.
        cond: Box<Expr>,
        /// The then-branch statements.
        then: Vec<Stmt>,
        /// The optional else-branch statements.
        els: Option<Vec<Stmt>>,
        /// Span of the whole expression.
        span: Span,
    },
    /// A `for var in iter { body... }` expression.
    For {
        /// The loop variable's name.
        var: String,
        /// The iterated expression.
        iter: Box<Expr>,
        /// The loop body.
        body: Vec<Stmt>,
        /// Span of the whole expression.
        span: Span,
    },
    /// An `asm { ... }` block — a `Code` value.
    Asm {
        /// The assembly statements.
        body: Vec<AsmStmt>,
        /// Span of the whole expression.
        span: Span,
    },
    /// A comptime lambda `|p1, p2| body` (≥1 param). Erases at lowering; used to
    /// feed inline transforms to map/filter/fold (§6.8, D2.12).
    Lambda {
        /// The parameter names, in order (at least one).
        params: Vec<String>,
        /// The single body expression.
        body: Box<Expr>,
        /// Span of the whole lambda.
        span: Span,
    },
    /// A `match scrutinee { pat => body, ... }` expression.
    Match {
        /// The scrutinee being matched.
        scrutinee: Box<Expr>,
        /// The match arms, in order.
        arms: Vec<MatchArm>,
        /// Span of the whole expression.
        span: Span,
    },
    /// `sizeof(T)` — the byte size of a type (resolved at layout time).
    SizeOf(Box<Type>, Span),
    /// `offsetof(T, field)` — the byte offset of `field` within `T`.
    OffsetOf(Box<Type>, String, Span),
    /// `rescale<I, F>(x)` — reinterpret a fixed-point value under a new
    /// `fixed<I, F>` scale.
    Rescale {
        /// Target integer-part bit width.
        i: u32,
        /// Target fraction-part bit width.
        f: u32,
        /// The value being rescaled.
        arg: Box<Expr>,
        /// Span of the whole expression.
        span: Span,
    },
    /// Postfix indexing `base[i]` (D2.33): comptime element access into an
    /// array, or raw-byte access into a `Data` value (`embed(...)[i]`).
    Index {
        /// The indexed expression.
        base: Box<Expr>,
        /// The index expression (a comptime integer).
        index: Box<Expr>,
        /// Span of the whole expression.
        span: Span,
    },
    /// Postfix field access off a NON-path base (D2.33): `embed(...).len`.
    /// Path-shaped access (`a.b`) stays inside [`Expr::Path`] segments — this
    /// node only ever wraps calls/literals/parenthesized/indexed bases.
    Field {
        /// The receiver expression.
        base: Box<Expr>,
        /// The accessed field name.
        name: String,
        /// Span of the whole expression.
        span: Span,
    },
}

/// A single arm of a [`Expr::Match`]: `Pat => body`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// The arm's pattern.
    pub pat: Pattern,
    /// The arm's body expression.
    pub body: Expr,
    /// Span of the whole arm.
    pub span: Span,
}

/// A match-arm pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `_` — matches anything, binds nothing.
    Wildcard(Span),
    /// A bare lowercase identifier — matches anything, binds it to a name.
    Binding(String, Span),
    /// A path (optionally qualified, e.g. `Anim.Idle`), optionally followed
    /// by parenthesized subpatterns for a payload-carrying variant, e.g.
    /// `Token.Literal(s)`.
    Variant {
        /// The variant's path.
        path: Path,
        /// Subpatterns for the variant's payload (empty for a nullary variant).
        subpats: Vec<Pattern>,
        /// Span of the whole pattern.
        span: Span,
    },
}

/// A call argument, optionally named: `spawn(SeedDef, offset: 4)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    /// The argument's name, if given by keyword.
    pub name: Option<String>,
    /// The argument's value.
    pub value: Expr,
    /// Span of the whole argument.
    pub span: Span,
}

/// A unary operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    /// Arithmetic negation: `-x`.
    Neg,
    /// Logical negation: `!x`.
    Not,
    /// Bitwise complement: `~x`.
    BitNot,
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `++` (concatenation)
    Concat,
}

// ---- comptime-fn statements -------------------------------------------

/// A statement, valid inside `comptime fn` bodies (and comptime blocks in procs).
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let name = value`
    Let {
        /// The bound name.
        name: String,
        /// The bound value.
        value: Expr,
        /// Span of the whole statement.
        span: Span,
    },
    /// `let (a, b, c) = e`
    LetTuple {
        /// The bound names, in tuple order.
        names: Vec<String>,
        /// The bound value.
        value: Expr,
        /// Span of the whole statement.
        span: Span,
    },
    /// `comptime var name: ty = value`
    Var {
        /// The bound name.
        name: String,
        /// Optional explicit type annotation.
        ty: Option<Type>,
        /// The initial value.
        value: Expr,
        /// Span of the whole statement.
        span: Span,
    },
    /// `target = value`
    Assign {
        /// The assignment target.
        target: Path,
        /// The assigned value.
        value: Expr,
        /// Span of the whole statement.
        span: Span,
    },
    /// `return [value]`
    Return {
        /// The returned value, if any.
        value: Option<Expr>,
        /// Span of the whole statement.
        span: Span,
    },
    /// A bare expression statement.
    Expr(Expr),
    /// `while cond { body... }`
    While {
        /// The loop condition.
        cond: Expr,
        /// The loop body.
        body: Vec<Stmt>,
        /// Span of the whole statement.
        span: Span,
    },
    /// A nested `comptime { ... }` block.
    ComptimeBlock {
        /// The block's statements.
        body: Vec<Stmt>,
        /// Span of the whole statement.
        span: Span,
    },
    /// A `patch name: ty` declaration.
    Patch {
        /// The patched name.
        name: String,
        /// The patch's type.
        ty: Type,
        /// Span of the whole statement.
        span: Span,
    },
    /// A `bind name = value` declaration.
    Bind {
        /// The bound name.
        name: String,
        /// The bound value.
        value: Expr,
        /// Span of the whole statement.
        span: Span,
    },
    /// An [`Expr::If`] at statement position.
    If(Expr),
    /// An [`Expr::For`] at statement position.
    For(Expr),
}

// ---- proc/asm bodies ---------------------------------------------------

/// A statement within a `proc` (or `asm { }`) body.
#[derive(Debug, Clone, PartialEq)]
pub enum AsmStmt {
    /// A label definition: `.draw:` or `export .done:`.
    Label {
        /// The label's name.
        name: String,
        /// Whether the label is exported.
        export: bool,
        /// Span of the whole statement.
        span: Span,
    },
    /// A single machine instruction line.
    Instr(InstrLine),
    /// A comptime-fn call at statement position, e.g. `spawn(SeedDef, offset: ...)`.
    Call(Expr),
    /// A `todo!`/`unreachable!` statement trap (S2-D11(e)): assembles to the
    /// 68k ILLEGAL word so a WIP file builds and RUNS to the hole; `todo!`
    /// additionally names itself at build time (`[todo.present]`).
    Trap {
        /// Which spelling this is (`todo!` reports, `unreachable!` is silent).
        kind: TrapKind,
        /// The optional site message: `todo!("wire the seed spawn")`.
        message: Option<String>,
        /// Span of the whole statement.
        span: Span,
    },
    /// A comptime `if` at proc/asm statement position (tranche 5, H1 —
    /// mt_bank's define-conditional pattern for CODE): the condition must
    /// evaluate to a comptime bool/int; the chosen branch's statements lower
    /// inline, the unchosen branch is never lowered. `els` holds either the
    /// `else { }` body or a single nested `If` for an `else if` chain.
    /// Branches hold `AsmStmt` only, so a script `yield` (a `ScriptStmt`)
    /// can never nest inside one by construction.
    If {
        /// The comptime condition.
        cond: Expr,
        /// Statements lowered when the condition is true.
        then: Vec<AsmStmt>,
        /// `else` statements (or a single `If` for `else if`), if any.
        els: Option<Vec<AsmStmt>>,
        /// Span of the whole statement.
        span: Span,
    },
}

/// The two statement-trap spellings (S2-D11(e)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapKind {
    /// `todo!` — a hole to fill; every site is reported via `[todo.present]`.
    Todo,
    /// `unreachable!` — a permanent, intentional trap; no diagnostic.
    Unreachable,
}

/// A single machine-instruction line: mnemonic, optional size, operands.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrLine {
    /// The mnemonic, possibly spliced: `b{cc}` → `[Text("b"), Splice(cc)]`.
    pub mnemonic: Vec<TextOrSplice>,
    /// The optional size suffix: `.b` / `.{w}`.
    pub size: Option<TextOrSplice>,
    /// The instruction's operands.
    pub operands: Vec<Operand>,
    /// Span of the whole instruction line.
    pub span: Span,
}

/// A piece of mnemonic/size text that may be literal or a `{splice}`.
#[derive(Debug, Clone, PartialEq)]
pub enum TextOrSplice {
    /// Literal text.
    Text(String),
    /// A spliced comptime expression.
    Splice(Expr),
}

/// An instruction operand.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// An immediate: `#expr`.
    Imm(Expr),
    /// Pre-decrement addressing: `-(a7)`.
    PreDec(Box<Operand>),
    /// Post-increment addressing: `(a0)+`.
    PostInc(Box<Operand>),
    /// Indirect addressing: `(e1, e2.w, ...)` with optional trailing size,
    /// e.g. `(VDP_Ctrl).l`.
    Ind {
        /// The parenthesized parts, each with an optional per-part size.
        parts: Vec<(Expr, Option<TextOrSplice>)>,
        /// The optional trailing size suffix.
        size: Option<TextOrSplice>,
        /// Span of the whole operand.
        span: Span,
    },
    /// Displacement + indirect addressing: `timer(a0)`, `4(a0,d0.w)` — a
    /// displacement expression applied to an inner [`Operand::Ind`].
    DispInd {
        /// The displacement expression.
        disp: Expr,
        /// The inner indirect operand.
        inner: Box<Operand>,
        /// The displacement arrived as a `{splice}` (`{off}(aN)`, F1/tranche 7),
        /// not a literal/field expression. Only the DIAGNOSTIC class differs: a
        /// non-int spliced displacement reports `[asm.splice-kind]` (the operand-
        /// splice diagnostic) rather than the generic "displacement must be an
        /// integer" — the evaluation and range-check are otherwise identical.
        disp_spliced: bool,
        /// Span of the whole operand.
        span: Span,
    },
    /// A bare expression operand: register, label, `.local`, path.
    Plain {
        /// The operand expression.
        expr: Expr,
        /// The optional size suffix.
        size: Option<TextOrSplice>,
        /// Span of the whole operand.
        span: Span,
    },
    /// `{splice}` as a whole operand.
    Splice(Expr),
}
