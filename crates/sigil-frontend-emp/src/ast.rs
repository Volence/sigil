//! Spanned AST for .emp (Spec 2 §10 surface). Pure data, plus the derived
//! canonical register views of a proc signature ([`ProcDecl::unconditional_outs`]
//! and friends) — a projection of the declared reglists through the register
//! file, not semantics of its own.
use sigil_span::Span;

/// The `assert.<w>` / FSTRING-argument display width, re-exported from the
/// diagnostics encoder ([`crate::eval::diag`], Task 1) so the AST and the
/// byte-encoder name ONE type — the desugar/lowering stage (Task 3) hands an
/// [`AsmStmt::Assert`]'s `width` straight to `diag::assert_message` with no
/// conversion seam. (The encoder module owns the definition because its
/// `width_bits` mapping is the byte-level ground truth; `ast` merely surfaces
/// the spelling in the grammar.)
pub use crate::eval::diag::Width;

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
    /// Attribute arguments, positional or keyword: the `"naming.pascal"` in
    /// `@allow("naming.pascal")`, the `cycles: 195` in `@budget(cycles: 195)`.
    pub args: Vec<Arg>,
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
    /// `module x.y (cpu: z80)` — the module attribute list (T1, §5). Only the
    /// `cpu:` key is read today (it seeds the module's default-section CPU);
    /// this is the ONE forward-compatible slot a later rung's module-scope
    /// `invariant(de)` etc. attaches to. `(key, value)` pairs, section-attr
    /// shape.
    pub attrs: Vec<(String, Expr)>,
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
    /// `table ...` declaration (Plan 7 T2-d): a counted / sentinel / sparse
    /// collection. Sibling of [`Item::Offsets`] (disjoint cell byte contract),
    /// sharing lowering machinery. Boxed — the attribute-rich [`TableDecl`] is
    /// much larger than the other variants.
    Table(Box<TableDecl>),
    /// `dispatch ...` declaration.
    Dispatch(DispatchDecl),
    /// `vars ...` declaration.
    Vars(VarsDecl),
    /// `region ...` declaration (item #7): a RAM address window a region-form
    /// `vars` block allocates variables into.
    Region(RegionDecl),
    /// `data ...` declaration.
    Data(DataDecl),
    /// `proc ...` declaration.
    Proc(ProcDecl),
    /// `extern proc ...` boundary declaration (contract-grammar v2 §3): the
    /// contract of an `.asm`-defined callee; emits nothing, closure leaf.
    ExternProc(ExternProcDecl),
    /// `extern NAME: Type` boundary declaration (L8): a typed link reference to
    /// a value symbol defined outside this module (a harvested game constant, an
    /// AS/emp equ). Emits nothing; referencing the name yields a link-deferred
    /// value carrying the declared newtype, so an engine module can name a
    /// game-side id WITH its type without mirroring the value.
    ExternConst(ExternConstDecl),
    /// `type X = proc (...) ...` contract-type declaration (contract-grammar v2
    /// §4): the bound every installable dispatch target must satisfy.
    ContractType(ContractTypeDecl),
    /// `interface Name { members }` (L1): an engine-declared game contract — the
    /// typed surface a game IMPLEMENTS. Emits nothing; the bind pass
    /// ([`crate::resolve::contract`]) resolves each declared member against the
    /// one `implement` block for the interface, and consuming engine modules
    /// name members qualified (`Name.MEMBER`) or `invoke Name.hook`.
    Interface(InterfaceDecl),
    /// `implement Name { bindings }` (L1): a game's manifest — the one binding of
    /// each interface member (a const value, a proc symbol, or a hook symbol),
    /// optionally under a comptime `if` group. Emits nothing.
    Implement(ImplementDecl),
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
    /// `context Name { … }` (contract unification §3.1): a declared machine-state
    /// context — the DECLARED tier above the inferred `[bus.*]` net.
    Context(ContextDecl),
}

/// A `context Name { … }` declaration (contract unification §3.1). Two flavors:
///
/// ```text
/// context z80_stopped {           // ACQUIRED: compiler-owned bracket
///     acquire = asm {
///         move.w  #$0100, Z80_BUS_REQUEST
///     .wait_z80:
///         btst    #0, Z80_BUS_REQUEST
///         bne     .wait_z80
///     }
///     release = asm { move.w #$0000, Z80_BUS_REQUEST }
/// }
///
/// context vblank { granted }      // GRANTED: entered by hardware / a dispatcher
/// ```
///
/// Contexts are module-scoped, `pub`-able, `use`-importable comptime-only items:
/// they emit ZERO bytes of their own — an ACQUIRED context's bytes appear only
/// where a `with` bracket splices its `acquire`/`release`.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextDecl {
    /// Whether the context is exported (`pub context`).
    pub public: bool,
    /// The context's name (the spelling `with`/`requires`/`grants` use).
    pub name: String,
    /// Acquired (with acquire/release Code exprs) or granted (a trust root).
    pub kind: ContextKind,
    /// Span of the whole declaration.
    pub span: Span,
}

/// Which flavor a [`ContextDecl`] is (§3.1).
#[derive(Debug, Clone, PartialEq)]
pub enum ContextKind {
    /// An ACQUIRED context: `with <ctx> { }` splices `acquire` before the body
    /// and `release` after it, and proves the pairing. Both exprs must evaluate
    /// to `Code`, and they are evaluated AT THE USE SITE in the consumer's scope
    /// — so a `pub` context spells its bracket inline rather than calling a
    /// module-private template (see `resolve::pub_comptime_name`).
    Acquired {
        /// The `acquire = <expr>` Code expression.
        acquire: Expr,
        /// The `release = <expr>` Code expression.
        release: Expr,
    },
    /// A GRANTED context: no acquire/release. Asserted at a root proc via
    /// `grants(...)`; the assembler cannot verify hardware dispatch, so the
    /// grant is a TRUST ROOT — greppable and auditable, never inferred.
    Granted,
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
        Item::Table(d) => d.span,
        Item::Dispatch(d) => d.span,
        Item::Vars(d) => d.span,
        Item::Region(d) => d.span,
        Item::Data(d) => d.span,
        Item::Proc(d) => d.span,
        Item::ExternProc(d) => d.span,
        Item::ExternConst(d) => d.span,
        Item::ContractType(d) => d.span,
        Item::Interface(d) => d.span,
        Item::Implement(d) => d.span,
        Item::Script(d) => d.span,
        Item::ComptimeFn(d) => d.span,
        Item::Section(d) => d.span,
        Item::Newtype(d) => d.span,
        Item::Ensure(d) => d.span,
        Item::Align(d) => d.span,
        Item::ComptimeTest(d) => d.span,
        Item::Context(d) => d.span,
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

/// A `table Name [: [RowType]] [(attrs)] { rows }` block (Plan 7 T2-d): a
/// counted / sentinel / sparse ROM-data collection. The single abstract shape
/// `[count header] rows [terminator]`, optionally fronted by a keyed index
/// table. Two emission shapes, chosen by the presence of the `cell:` attribute:
/// record-list (contiguous `[header?] rows [sentinel?]`) or index (a payload
/// stream plus a key-addressed cell table). See the design doc
/// `2026-07-11-counted-sparse-collection-design.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct TableDecl {
    /// Whether this table is exported (`pub table`).
    pub public: bool,
    /// The table's name — the base label (record-list: the header/first byte;
    /// index: the first CELL, so `Table[key - min_key]` indexing is correct).
    pub name: String,
    /// The optional `: [RowType]` element-type annotation (typed rows). `None`
    /// for blob/parts rows.
    pub row_type: Option<Type>,
    /// The attribute knobs (`cell:`/`key:`/`hole:`/`header:`/…).
    pub attrs: TableAttrs,
    /// The rows, in declaration order (keyed rows must be ascending).
    pub rows: Vec<TableRow>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// The `(attr, ...)` knobs of a [`TableDecl`]. Every field is optional; their
/// presence selects the emission shape and semantics (design §3).
#[derive(Debug, Clone, PartialEq)]
pub struct TableAttrs {
    /// `cell: PtrType` — index mode: emit a cell per key over the key domain.
    pub cell: Option<Type>,
    /// `key: KeyDomain` — the key domain (a `lo..=hi` range in v1).
    pub key: Option<KeyDomain>,
    /// `hole: IntLiteral` — sparse fill for absent keys (else exhaustive).
    pub hole: Option<Expr>,
    /// `header: Type(Expr)` — a count header word; `Expr` is over the reserved
    /// `count` (the derived row count).
    pub header: Option<(Type, Expr)>,
    /// `sentinel: Value` — a trailing terminator row/value.
    pub sentinel: Option<Expr>,
    /// `item_align: N` — a self-adjusting pad after every emitted part.
    pub item_align: Option<Expr>,
    /// `body: before | after` — payload-stream placement vs the cell table
    /// (index mode only; default `after`).
    pub body: Option<BodyPlacement>,
    /// Span of the whole attribute list (for whole-list diagnostics).
    pub span: Span,
}

/// Placement of the index-mode payload stream relative to the cell table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyPlacement {
    /// Payload emits BEFORE the cell table (`body: before`).
    Before,
    /// Payload emits AFTER the cell table (`body: after`, the default).
    After,
}

/// A `table`'s key domain (design §3). v1 supports an inclusive integer range;
/// enum / `offsets`-name domains are a later increment.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyDomain {
    /// `lo..=hi` — an inclusive integer range.
    Range(Expr, Expr),
}

/// One row of a [`TableDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    /// The optional `Key:` prefix (required in keyed tables, absent otherwise).
    pub key: Option<Expr>,
    /// The row's body — labeled data parts, or a typed record literal.
    pub body: TableRowBody,
    /// Span of the whole row.
    pub span: Span,
}

/// A [`TableRow`]'s body (design §3 `row_body`).
#[derive(Debug, Clone, PartialEq)]
pub enum TableRowBody {
    /// Blob mode: one or more `Label = DataExpr` parts (the `offsets` D2.31
    /// inline-body precedent). The cell (index mode) targets the FIRST part's
    /// label.
    Parts(Vec<TablePart>),
    /// Typed mode: a `[RowType]` record literal (§4.5 struct-literal rules).
    Record(Expr),
}

/// One `Label = DataExpr` part of a [`TableRowBody::Parts`] row — the exact
/// `data`-item shape. The label is an ordinary module-scoped data label (real
/// link symbol, `pub`-able, cross-seam-visible).
#[derive(Debug, Clone, PartialEq)]
pub struct TablePart {
    /// The part's label (a real link symbol).
    pub label: String,
    /// The part's data expression.
    pub value: Expr,
    /// Span of the whole part.
    pub span: Span,
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
    /// The OVERLAY-form block's fields (`vars Name: window { .. }`), in
    /// declaration order. Empty for the region form, which uses
    /// [`region_body`](Self::region_body) instead.
    pub fields: Vec<VarsField>,
    /// The REGION-form block's ordered items (`vars region { .. }`, item #7):
    /// typed fields interleaved with `pad`/`mark`/`alias`/conditional groups, in
    /// declaration order (order is load-bearing — it is the RAM allocation
    /// order). Empty for the overlay form.
    pub region_body: Vec<RegionField>,
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

/// One item inside a region-form `vars` block (item #7 §2.2), in declaration
/// (allocation) order. RAM emits no image bytes, so `@align` and `pad` here are
/// pure RESERVE advances of the location counter — no fill is ever emitted.
#[derive(Debug, Clone, PartialEq)]
pub enum RegionField {
    /// `name: T [@align(N)]` — a typed variable defining a link-visible label at
    /// its address and reserving `sizeof(T)` bytes. Reuses [`VarsField`].
    Typed(VarsField),
    /// `pad(N)` — an anonymous reserve of N bytes (the `ds.b 1` even-pad idiom),
    /// intent-named, defining no label.
    Pad { count: Expr, span: Span },
    /// `mark Name` — a zero-size label at the current counter (the `ram.asm`
    /// marker-label idiom, e.g. `Object_RAM:`, `Engine_RAM_End:`).
    Mark { name: String, span: Span },
    /// `name: alias(Other)` — a label equal to another field's address (the
    /// buffer-reuse `Name = Other` idiom). Allocates nothing.
    Alias { name: String, target: String, span: Span },
    /// `if <cond> [@shape_divergent] { .. } [else { .. }]` — a conditional field
    /// group driven by the comptime define environment. A size-varying group must
    /// carry `shape_divergent`; a size-equal group is proven invariant.
    Group {
        /// The comptime condition (nonzero = the `then` arm).
        cond: Expr,
        /// `@shape_divergent` present — the author declaring the group's arms may
        /// differ in size (everything after the group moves between shapes).
        shape_divergent: bool,
        /// Fields placed when `cond` is nonzero.
        then_body: Vec<RegionField>,
        /// Fields placed when `cond` is zero (empty when there is no `else`).
        else_body: Vec<RegionField>,
        /// Span of the whole group (the `if` keyword through the closing brace).
        span: Span,
    },
}

/// A `region name @ base .. limit [, w_addressable]` declaration (item #7 §2.1):
/// a named RAM address window that region-form `vars` blocks allocate into.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionDecl {
    /// Whether this region is exported (`pub region`).
    pub public: bool,
    /// The region's name (`upper_ram`, `game_ram`, …).
    pub name: String,
    /// The region's base VMA — a literal address or `after(<region>)`.
    pub base: RegionBase,
    /// The region's exclusive limit VMA (a comptime expression).
    pub limit: Expr,
    /// `w_addressable` — assert every byte in `[base, limit)` is reachable by
    /// sign-extended `.w` addressing (bit 15 set across the window).
    pub w_addressable: bool,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A [`RegionDecl`]'s base: an explicit address or the running end of another
/// region (`after(<region>)`, the `phase Engine_RAM_End` chaining idiom).
#[derive(Debug, Clone, PartialEq)]
pub enum RegionBase {
    /// `@ <expr> ..` — an explicit base VMA (comptime expression).
    Addr(Expr),
    /// `@ after(<region>) ..` — the base is the chained parent region's end.
    After { region: String, span: Span },
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
    /// Optional `(align: N)` per-item alignment (L3): pin THIS item's own base to
    /// the next `N`-byte boundary, independent of what precedes it — the pad is a
    /// self-adjusting `$00` fill before the item's label, plus a link-time
    /// congruence assert on the final address. Unlike a top-level `align N` item
    /// this survives size-relaxable code earlier in the section for a
    /// relaxation-INVARIANT `N` (word alignment on the even-length m68k ISA): the
    /// pad's parity cannot shift when every relaxation delta is a multiple of 2.
    pub align: Option<Expr>,
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

/// A flag-encoded result declared via `out(carry: name)` (contract-grammar v2
/// §6 / D2): the callee returns a status flag the caller MUST consume. `carry`
/// is the sole corpus demand today (`QueueDMA_*`'s `dropped`, `RingBuffer_Add`'s
/// `full`); the parser accepts the general flag-name form (`zero`/`negative`/…)
/// for forward use, with flag-name VALIDITY a lowering-time check
/// (`[proc.out-flag-invalid]`), mirroring `clobbers`/`out` reg validity. Unlike
/// an `out` register, a flag is NOT part of the register-file partition — it
/// lands here, never in the `out` reglist, so the transitive closure (§1) is
/// unaffected (flag results are pure caller-side must-use metadata, §6).
#[derive(Debug, Clone, PartialEq)]
pub struct FlagResult {
    /// The status flag — `carry` today; any flag name the parser accepts.
    pub flag: String,
    /// The result's name (`dropped`, `full`, …), used by `@discards(name)`.
    pub name: String,
    /// Span of the `flag: name` clause.
    pub span: Span,
}

/// A conditional register result declared via `out(rN if cc)` (contract-grammar
/// v2 §6, D2.35's deferred sibling): register `rN` is a live-out result, but
/// VALID only on the path where condition `cc` holds. `rN` ALSO joins the `out`
/// reglist (so the closure charges it as written); this guard rides alongside so
/// the caller-side check can flag reading `rN` on the invalid path
/// (`[call.result-invalid-path]`). Register + cc validity are lowering-time
/// checks (`[proc.out-cond-invalid]`).
#[derive(Debug, Clone, PartialEq)]
pub struct CondResult {
    /// The result register (`a1`, …) — also present in the `out` reglist.
    pub reg: String,
    /// The condition code under which `reg` is valid (`cc`, `eq`, `cs`, …).
    pub cc: String,
    /// Span of the `rN if cc` clause.
    pub span: Span,
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
    /// Registers this proc clobbers, as declared reglist segments (C1 item 2 —
    /// the movem-reglist grammar `preserves` uses): `clobbers(d0-d3/a1)` →
    /// `[("d0", Some("d3")), ("a1", None)]`. `None` = no contract declared
    /// (legal — half-ported files); `Some(vec![])` = the explicit `clobbers()`
    /// form, "verified: touches nothing" (Volence ruling, tranche 3) — the lint
    /// then flags ANY register write. Register validity + range expansion is a
    /// lowering-time check (`[proc.clobber-invalid]`), not a parse-time one.
    pub clobbers: Option<Vec<(String, Option<String>)>>,
    /// Registers this proc preserves (S2-D6b syntactic slice), as declared
    /// reglist segments: `preserves(d0-d1/a0)` → `[("d0", Some("d1")),
    /// ("a0", None)]`. Register validity is a lowering-time check
    /// (`[proc.preserves-invalid]`), not a parse-time one, mirroring
    /// `clobbers`.
    pub preserves: Vec<(String, Option<String>)>,
    /// Registers this proc RETURNS — the third partition member (S2-D6e). `None`
    /// = no `out(...)` declared (legal); `Some(vec![])` = the explicit `out()`
    /// form (declares "returns nothing"). As of C1 item 2 it takes the SAME
    /// movem-reglist grammar as `clobbers`/`preserves` (`out(d0-d1/a0)`).
    /// Output registers join `check_clobbers`' `allowed` set (a result-register
    /// write is not `[proc.clobber-undeclared]`); register validity + range
    /// expansion is a lowering-time check (`[proc.out-invalid]`), not a
    /// parse-time one, mirroring `clobbers`/`preserves`.
    pub out: Option<Vec<(String, Option<String>)>>,
    /// Flag results declared via `out(carry: name)` (contract-grammar v2 §6):
    /// status-flag-encoded results the caller MUST consume. Empty for a proc with
    /// no flag result. Separate from `out` because a flag is not a register-file
    /// member (so the closure ignores it); see [`FlagResult`].
    pub out_flags: Vec<FlagResult>,
    /// Conditional register results declared via `out(rN if cc)` (§6): each
    /// names a register ALSO present in `out`, guarded by a validity condition.
    /// Empty for a proc with no conditional result; see [`CondResult`].
    pub out_cond: Vec<CondResult>,
    /// Typed data-register results declared via `out(dN: Type)` (G5, §7 tier 5):
    /// a domain newtype (`out(d0: SectionId)`) on an output register — the
    /// data-register analogue of the `out(carry: name)` flag result. The
    /// register ALSO joins `out` (so out-verify still checks it is written); the
    /// type is metadata the caller-side slot-type slice checks. `(reg, ty, span)`.
    /// Empty for a proc with no typed output.
    pub out_types: Vec<(String, Type, Span)>,
    /// Contexts every call site of this proc must have active (§3.3):
    /// `requires(z80_stopped, vblank)`. `(name, span)` in source order; empty
    /// for a proc with no context requirement. CHECKED at every call site by
    /// `[context.unsatisfied]` — which is what makes the entry state of a
    /// requiring proc a DECLARED fact the machine-state nets may seed from.
    pub requires: Vec<(String, Span)>,
    /// Contexts this proc asserts are active for its whole body (§3.2):
    /// `grants(vblank)`. A TRUST ROOT — the assembler cannot verify hardware
    /// dispatch, so a grant is never inferred and never checked, only recorded
    /// and audited. `(name, span)` in source order; empty for an ordinary proc.
    pub grants: Vec<(String, Span)>,
    /// The proc this one falls into, if any.
    pub falls_into: Option<String>,
    /// Item-level `@`-attributes preceding the decl — currently only
    /// `@scaffolding("reason")` (contract-grammar v2 §8): inert metadata that
    /// marks a ratified zero-caller keep so D7's dead-symbol analysis won't nag
    /// it. Empty for a plain proc.
    pub attrs: Vec<Attr>,
    /// The proc's assembly body.
    pub body: Vec<AsmStmt>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A proc SIGNATURE — the params + register-effect contract clauses shared by a
/// `proc` body decl, an `extern proc` boundary decl (§3), and a `type X = proc`
/// contract type (§4). Reglist segments take the movem-reglist grammar
/// (`clobbers(d0-d3/a1)`), validated at lowering like [`ProcDecl`]'s.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcSig {
    /// Parameters as `(name, optional type, span)` — the declared inputs (§2:
    /// the param list IS `in()`). The type is OPTIONAL here (unlike a `proc`
    /// body's params): a bare-register param `(d4)` is a legal untyped input
    /// (§2's no-ceremony rule), `None`; `(a4: *DictBase)` carries `Some(ty)` for
    /// the future §7 slot check. These params are never lowered (the decl emits
    /// nothing), so an untyped one needs no synthesized `Type`.
    pub params: Vec<(String, Option<Type>, Span)>,
    /// `clobbers(...)`: `None` = undeclared, `Some(vec![])` = explicit empty.
    pub clobbers: Option<Vec<(String, Option<String>)>>,
    /// `preserves(...)` reglist segments (empty = none).
    pub preserves: Vec<(String, Option<String>)>,
    /// `out(...)`: `None` = undeclared, `Some(vec![])` = explicit empty.
    pub out: Option<Vec<(String, Option<String>)>>,
    /// Flag results (`out(carry: name)`, §6) — see [`FlagResult`].
    pub out_flags: Vec<FlagResult>,
    /// Conditional register results (`out(rN if cc)`, §6) — see [`CondResult`].
    pub out_cond: Vec<CondResult>,
    /// Typed data-register results (`out(dN: Type)`, G5 §7 tier 5) — see the
    /// same field on [`ProcDecl`]. `(reg, ty, span)`; empty = none.
    pub out_types: Vec<(String, Type, Span)>,
    /// `requires(ctx, …)` (§3.3) — see the same field on [`ProcDecl`]. An
    /// `extern proc` carries it so an `.asm`-defined callee can demand a context
    /// exactly as a `.emp` one does. There is deliberately no `grants` here: a
    /// grant is a trust root asserted where the BODY lives, and a signature has
    /// no body.
    pub requires: Vec<(String, Span)>,
}

/// The canonical register SET a contract reglist denotes under register file
/// `rf`. 68k routes through the frozen production expander (`sp`→`a7`, `sr`
/// dropped, movem ranges expanded); Z80 through the register-file seam (pair
/// sugar splits to halves, no range form). Errors are discarded — the
/// `[proc.*-invalid]` / `[contract.unknown-register]` diagnostics are owned by
/// the primary validation sites, and a nonsense name simply contributes nothing.
fn expand_contract_reglist(
    segs: &[(String, Option<String>)],
    rf: crate::regfile::RegFile,
) -> std::collections::BTreeSet<String> {
    match rf {
        crate::regfile::RegFile::M68k => crate::lower::expand_reglist_regs(segs),
        crate::regfile::RegFile::Z80 => crate::regfile::expand_reglist(segs, rf, |_| {}),
    }
}

/// The registers carrying an `out(rN if cc)` guard, CANONICALIZED under `rf`.
fn cond_out_regs_of(
    out_cond: &[CondResult],
    rf: crate::regfile::RegFile,
) -> std::collections::BTreeSet<String> {
    let segs: Vec<(String, Option<String>)> =
        out_cond.iter().map(|c| (c.reg.clone(), None)).collect();
    expand_contract_reglist(&segs, rf)
}

/// The registers whose `out()` mention is EXCLUSIVELY conditional — every
/// appearance in the reglist carries an `if cc` guard.
///
/// [`cond_out_regs_of`] answers "does this register carry SOME guard", which is
/// too coarse for the `[proc.out-clobbers-overlap]` exemption: `out(a1, a1 if
/// eq) clobbers(a1)` states unconditionally that a1 is a result AND that it is
/// scratch, the exact contradiction that diagnostic names, and one guarded
/// mention must not license it. The parser pushes each `rN if cc` clause into the
/// reglist as its own single segment, so a register is exclusively conditional
/// iff the number of reglist segments COVERING it does not exceed the number of
/// `if cc` clauses NAMING it. Counting (rather than set subtraction) is what
/// catches a range mention too: `out(a0-a2, a1 if eq)` covers a1 twice against
/// one guard, so a1 is not exclusively conditional.
fn cond_only_out_regs_of(
    out: Option<&[(String, Option<String>)]>,
    out_cond: &[CondResult],
    rf: crate::regfile::RegFile,
) -> std::collections::BTreeSet<String> {
    let mut mentions: std::collections::BTreeMap<String, usize> = Default::default();
    for seg in out.unwrap_or(&[]) {
        for r in expand_contract_reglist(std::slice::from_ref(seg), rf) {
            *mentions.entry(r).or_default() += 1;
        }
    }
    let mut guards: std::collections::BTreeMap<String, usize> = Default::default();
    for c in out_cond {
        for r in expand_contract_reglist(&[(c.reg.clone(), None)], rf) {
            *guards.entry(r).or_default() += 1;
        }
    }
    guards
        .into_iter()
        .filter(|(r, n)| mentions.get(r).copied().unwrap_or(0) <= *n)
        .map(|(r, _)| r)
        .collect()
}

/// The `(register, cc)` pairs of every EXCLUSIVELY-conditional out, canonical
/// under `rf` and in declaration order.
///
/// The set-returning accessors answer "which registers are guarded"; a consumer
/// that must know WHICH guard — the survives check, the edge-sensitive callee
/// credit, the corpus `cond_callees` map — needs the pair, and each one otherwise
/// rebuilds `out_cond.iter().filter_map(Reg::from_name …)` with its own
/// canonicalisation. Registers with an unconditional mention are dropped: their
/// out is unconditional, whatever a second guarded mention says.
fn cond_out_pairs_of(
    out: Option<&[(String, Option<String>)]>,
    out_cond: &[CondResult],
    rf: crate::regfile::RegFile,
) -> Vec<(String, String)> {
    let only = cond_only_out_regs_of(out, out_cond, rf);
    out_cond
        .iter()
        .filter_map(|c| {
            let reg = canonical_contract_reg(&c.reg, rf)?;
            only.contains(&reg).then(|| (reg, c.cc.clone()))
        })
        .collect()
}

/// The UNCONDITIONAL out registers: the expanded `out` reglist MINUS every
/// register that also carries an `if cc` guard. See [`ProcDecl::unconditional_outs`].
fn unconditional_outs_of(
    out: Option<&[(String, Option<String>)]>,
    out_cond: &[CondResult],
    rf: crate::regfile::RegFile,
) -> std::collections::BTreeSet<String> {
    let mut set = expand_contract_reglist(out.unwrap_or(&[]), rf);
    for r in cond_out_regs_of(out_cond, rf) {
        set.remove(&r);
    }
    set
}

/// The canonical views of a proc's declared `out()` clause.
///
/// **Why an accessor exists.** [`crate::parser`]'s `out_list` pushes an
/// `out(rN if cc)` register into BOTH [`ProcDecl::out_cond`] AND the plain
/// [`ProcDecl::out`] reglist — out-verify must see the register in `out` to check
/// it is written at all. Every consumer that needs the UNCONDITIONAL set must
/// therefore subtract the guarded registers itself, and every such subtraction
/// must expand both sides through the SAME register file (a raw-text subtraction
/// misses `sp` vs `a7` on 68k and every Z80 pair spelling). These two methods are
/// the single place that fact lives.
///
/// **Which view a consumer wants.** A gate that treats an out as a DEFINITION on
/// every return edge (D1b must-def credit, §6 taint-kill, D1c's held-value
/// excuse, an unconditional-`out` bound) takes [`Self::unconditional_outs`] —
/// crediting a conditional out there is a false negative. A gate asking "does the
/// callee WRITE this register" (the closure's effective set, a clobber license,
/// a derived preserves complement) takes the FULL `out` reglist: a conditional
/// result is written on the cc edge, so it is destroyed from the caller's view on
/// every edge. A gate RELAXING itself because the out is conditional takes
/// [`Self::cond_only_out_regs`] — a register mentioned unconditionally as well
/// keeps the unconditional reading, and the relax must not reach it.
impl ProcDecl {
    /// The registers carrying an `out(rN if cc)` guard, canonical under `rf`.
    pub fn cond_out_regs(
        &self,
        rf: crate::regfile::RegFile,
    ) -> std::collections::BTreeSet<String> {
        cond_out_regs_of(&self.out_cond, rf)
    }

    /// The registers whose every `out()` mention carries an `if cc` guard — the
    /// set the `[proc.out-clobbers-overlap]` exemption is keyed on. See
    /// [`cond_only_out_regs_of`].
    pub fn cond_only_out_regs(
        &self,
        rf: crate::regfile::RegFile,
    ) -> std::collections::BTreeSet<String> {
        cond_only_out_regs_of(self.out.as_deref(), &self.out_cond, rf)
    }

    /// The `(register, cc)` pairs of every exclusively-conditional out, canonical
    /// under `rf`. See [`cond_out_pairs_of`].
    pub fn cond_out_pairs(&self, rf: crate::regfile::RegFile) -> Vec<(String, String)> {
        cond_out_pairs_of(self.out.as_deref(), &self.out_cond, rf)
    }

    /// The declared outs that are produced on EVERY return path — the expanded
    /// `out` reglist minus [`Self::cond_out_regs`].
    pub fn unconditional_outs(
        &self,
        rf: crate::regfile::RegFile,
    ) -> std::collections::BTreeSet<String> {
        unconditional_outs_of(self.out.as_deref(), &self.out_cond, rf)
    }
}

impl ProcSig {
    /// The registers carrying an `out(rN if cc)` guard, canonical under `rf`.
    pub fn cond_out_regs(
        &self,
        rf: crate::regfile::RegFile,
    ) -> std::collections::BTreeSet<String> {
        cond_out_regs_of(&self.out_cond, rf)
    }

    /// The declared outs that are produced on EVERY return path — the expanded
    /// `out` reglist minus [`Self::cond_out_regs`].
    pub fn unconditional_outs(
        &self,
        rf: crate::regfile::RegFile,
    ) -> std::collections::BTreeSet<String> {
        unconditional_outs_of(self.out.as_deref(), &self.out_cond, rf)
    }
}

/// A canonical single register spelling under `rf`, or `None` when the name is
/// outside that CPU's contract vocabulary. `sp` canonicalizes to `a7` on 68k; a
/// Z80 pair name has no single canonical unit and returns `None`.
pub fn canonical_contract_reg(name: &str, rf: crate::regfile::RegFile) -> Option<String> {
    let set = expand_contract_reglist(&[(name.to_string(), None)], rf);
    (set.len() == 1).then(|| set.into_iter().next().unwrap())
}

/// An `extern proc Name (params) [clobbers(...)] [preserves(...)] [out(...)]`
/// declaration (contract-grammar v2 §3): the caller-side statement of a routine
/// defined in NO `.emp` — a still-`.asm` callee's contract, made checkable where
/// today there is prose or nothing. Emits NOTHING and defines NO label (the
/// `.asm` header stays the source of truth, drift-guarded by a comment); it only
/// registers the contract so the transitive closure (§1) can treat the extern as
/// a LEAF (`effective == declared clobbers`) instead of a hole. Participates in
/// module resolution as a real symbol decl (§11 Q4): a name declared both
/// `extern proc` and `proc` collides loudly.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternProcDecl {
    /// Whether this extern is re-exported (`pub extern proc`) — the §3
    /// second-consumer hoist to a shared home.
    pub public: bool,
    /// The external routine's link symbol.
    pub name: String,
    /// The declared signature (params + contract clauses).
    pub sig: ProcSig,
    /// Span of the whole declaration.
    pub span: Span,
}

/// An `extern NAME: Type` declaration (L8): a typed reference to a value symbol
/// defined outside this module and resolved at link (a harvested game constant's
/// EquSym, an AS/emp equ). Referencing `NAME` yields a link-deferred value tagged
/// with `ty`'s newtype, so an engine module names a game-side id WITH its type —
/// no local mirror const, no drift guard. Emits nothing; it is name-resolution
/// only, exactly like `const`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternConstDecl {
    /// Whether this extern is re-exported (`pub extern NAME: Type`).
    pub public: bool,
    /// The external value's link symbol.
    pub name: String,
    /// The declared type — the newtype the reference carries at use sites.
    pub ty: Type,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A `type Name = proc (params) [clobbers(...)] [preserves(...)] [out(...)]`
/// contract-type declaration (contract-grammar v2 §4): names the contract every
/// installable target of an indirect dispatch must satisfy. Used to BOUND a
/// `jsr (a1) as Name` site (so the closure uses the bound's clobbers instead of
/// ⊤) and to check table elements / pointer installs against the subcontract
/// relation. Emits nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractTypeDecl {
    /// Whether this contract type is exported (`pub type`).
    pub public: bool,
    /// The contract type's name (e.g. `ObjRoutine`, `HBlankHandler`).
    pub name: String,
    /// The proc-shaped contract every conforming target must satisfy.
    pub sig: ProcSig,
    /// Span of the whole declaration.
    pub span: Span,
}

/// An `interface Name { members }` declaration (L1): the engine-declared game
/// contract. Members come in three kinds ([`InterfaceMember`]): a typed `const`,
/// a `proc` reference bound by a declared proc contract type, and a `hook` the
/// engine INVOKES (a full proc signature, with an optional `= empty` default
/// whose unbound call site emits nothing). Emits nothing itself.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    /// Whether this interface is exported (`pub interface`).
    pub public: bool,
    /// The interface's name (the qualifier at consumer sites: `Name.MEMBER`).
    pub name: String,
    /// The declared members, in declaration order.
    pub members: Vec<InterfaceMember>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// One member of an [`InterfaceDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMember {
    /// The member's name (`Name.member`).
    pub name: String,
    /// The member's kind + typing.
    pub kind: InterfaceMemberKind,
    /// Span of the whole member.
    pub span: Span,
}

/// The kind of an [`InterfaceMember`] (v1, deliberately minimal — §2).
#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceMemberKind {
    /// `const NAME: Type` — a typed comptime value the engine's lowering
    /// consumes (a bound `implement` supplies it; the consumer reads
    /// `Name.NAME` as a comptime constant).
    Const(Type),
    /// `proc name: ProcType` — a reference to a game proc the engine takes the
    /// ADDRESS of (`#Name.name`), typed by a declared `type ProcType = proc`.
    Proc(Type),
    /// `hook name (params) clobbers(...) [preserves(...)] [= empty]` — a proc
    /// signature the ENGINE calls (`invoke Name.name`). `default_empty` marks
    /// the `= empty` form: an unbound hook's call site emits nothing; a hook
    /// WITHOUT it is required (a missing binding is `[contract.missing-member]`).
    Hook {
        /// The declared signature the impl must satisfy.
        sig: ProcSig,
        /// `= empty` present — the hook defaults to unbound (zero-byte call).
        default_empty: bool,
    },
}

/// An `implement Name { bindings }` declaration (L1): the one binding of each
/// interface member, optionally under a comptime `if` group. Emits nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplementDecl {
    /// Whether this implement is exported (`pub implement`).
    pub public: bool,
    /// The interface this block implements (`implement Name`).
    pub name: String,
    /// The member bindings, in declaration order (comptime `if` groups nest).
    pub bindings: Vec<ImplBinding>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// One binding inside an [`ImplementDecl`].
#[derive(Debug, Clone, PartialEq)]
pub enum ImplBinding {
    /// `const NAME = expr` — bind a const member to a comptime value.
    Const {
        /// The bound member's name.
        name: String,
        /// The value expression (evaluated in the impl module's scope).
        value: Expr,
        /// Span of the whole binding.
        span: Span,
    },
    /// `proc name = Symbol` — bind a proc member to a game proc symbol.
    Proc {
        /// The bound member's name.
        name: String,
        /// The bound symbol reference (a bare or dotted link name).
        symbol: Path,
        /// Span of the whole binding.
        span: Span,
    },
    /// `hook name = Symbol` — bind a hook member to a game proc that satisfies
    /// the declared hook signature.
    Hook {
        /// The bound member's name.
        name: String,
        /// The bound proc symbol.
        symbol: Path,
        /// Span of the whole binding.
        span: Span,
    },
    /// `if cond { bindings } [else { bindings }]` — a comptime conditional
    /// binding group (the item-7a `if DEBUG == 1 { }` precedent), driven by the
    /// build-shape define environment.
    Group {
        /// The comptime condition (nonzero = the `then` arm).
        cond: Expr,
        /// Bindings applied when `cond` is nonzero.
        then_body: Vec<ImplBinding>,
        /// Bindings applied when `cond` is zero (empty when there is no `else`).
        else_body: Vec<ImplBinding>,
        /// Span of the whole group.
        span: Span,
    },
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
    /// Parameters as `(name, type, span, default)`. The `default` expr, when
    /// present (`name: T = expr`), supplies the argument if the caller omits
    /// it; a param with no default is required (omission is a `missing
    /// argument` error). Defaults evaluate in declaration scope — globals
    /// only, never the caller's locals or sibling params.
    pub params: Vec<(String, Type, Span, Option<Expr>)>,
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

impl Arg {
    /// The argument's value as a string literal, or `None` for any other shape.
    pub fn str_value(&self) -> Option<&str> {
        match &self.value {
            Expr::Str(s, _) => Some(s.as_str()),
            _ => None,
        }
    }
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
    /// A local typed-register binding (Spec 2, C2): `let a2: *Sst`. Emits ZERO
    /// bytes — it is the author's typing ASSERTION about a register that already
    /// holds its value (no initializer). From the `let` to the end of the
    /// enclosing block (or until a subsequent `let` rebinds the same register), a
    /// bare field displacement `field(a2)` resolves in the type's field space,
    /// identically to a typed proc PARAM. `reg` is the register spelling (aN/dN);
    /// a non-register name is diagnosed at lowering.
    Let {
        /// The register being typed (spelling, e.g. `"a2"`).
        reg: String,
        /// The asserted type (anything a typed param accepts: `*Struct` pointer
        /// views, value newtypes).
        ty: Type,
        /// Span of the whole `let` statement.
        span: Span,
    },
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
    /// A `{expr}` Code-splice at statement position (2026-07-11 mini-spec): a
    /// hole whose `expr` evaluates to `Code`, inlined in place. Distinct from
    /// [`Call`](AsmStmt::Call) (a bare `foo(...)`) in that the braces admit ANY
    /// expr — a variable, an `if`-expr, `Code.empty()` — and mark the hole
    /// visibly. Hygiene is unchanged: the spliced items are already resolved
    /// within their producing block; a fragment neither defines nor references
    /// a skeleton label. `Code.empty()` splices to nothing; a `Data`/other
    /// value is a steering/type error.
    Splice(Expr),
    /// `assert.<w> src, cond [, dest]` (diagnostics construct, spec §3). A
    /// self-gated debug check: when `DEBUG != 1` it lowers to ZERO bytes (Task
    /// 3), else it expands to the CCR-safe compare/tst + `RaiseError` blob whose
    /// auto-message is built from the source SPELLINGS (spec §4.4). The parser
    /// records both the structured [`Operand`] (for eval's register/immediate
    /// validation, spec §5) AND the verbatim source spelling (for the message
    /// bytes — `#Object_RAM` must survive as `#Object_RAM` or the twin bytes
    /// diverge, spec §4.4 retrofit rule).
    Assert {
        /// The operation width (`.b`/`.w`/`.l`) — required at parse time.
        width: Width,
        /// The compared/tested source operand (register in v1, spec §5). Boxed
        /// because an [`Operand`] carries a full [`Expr`] — inlining two of them
        /// by value would make this the outsized `AsmStmt` variant
        /// (`clippy::large_enum_variant`).
        src: Box<Operand>,
        /// The verbatim source spelling of `src`, for the auto-message.
        src_spelling: String,
        /// The condition code: one of the 16 Bcc codes, lowercased. Validated
        /// against the code set at parse time (spec §5).
        cond: String,
        /// The compare destination (`cmp` form). `None` is the `tst` form —
        /// a flag test on `src` alone. The `String` is the verbatim spelling.
        dest: Option<(Box<Operand>, String)>,
        /// Span of the whole statement.
        span: Span,
    },
    /// `raise_error "<fstring>" [, <flag>]...` (diagnostics construct, spec §4.1)
    /// — an UNCONDITIONAL fatal: it lowers to the DELIBERATE-raise blob (frame
    /// simulation + user fstring) via [`crate::eval::diag::encode_fstring`].
    /// Unlike `assert` it has no DEBUG gate (matches AS: path_swap's is a
    /// release-path fatal). The optional trailing flags fold into the exit-flag
    /// byte's `opts` (the closed error-handler flag set,
    /// [`crate::eval::diag::ERROR_FLAGS`]); a non-flag second argument (the
    /// `consoleprogram` form) is a steering error at parse time (spec §5, out of
    /// scope).
    RaiseError {
        /// The user's format string (decoded string-literal contents).
        fstring: String,
        /// The error-handler flag bits from the options form (0 if none given).
        opts: u8,
        /// Span of the whole statement.
        span: Span,
    },
    /// `raise_exception "<fstring>" [, <flag>]...` (t25) — the EXCEPTION-VECTOR
    /// counterpart of `raise_error`: the CPU's `__ErrorMessage` handler shape.
    /// Lowers to the raise tail WITHOUT the `pea self(pc)` + `move.w sr,-(sp)`
    /// frame simulation (the CPU pushed SR+PC by hardware when it vectored to the
    /// handler), so it is exactly 6 bytes shorter at the front than `raise_error`.
    /// Used by the 12 CPU exception-vector stubs (error_handler.emp); the 2
    /// bus/address vectors carry `address_error` in `opts`.
    RaiseException {
        /// The user's format string (decoded string-literal contents).
        fstring: String,
        /// The error-handler flag bits from the options form (0 if none given).
        opts: u8,
        /// Span of the whole statement.
        span: Span,
    },
    /// `with <ctx> [if <comptime cond>] { … }` (contract unification §3.2): an
    /// ACQUIRED context bracket. The context's `acquire` Code splices before the
    /// body and its `release` after it — the SAME bytes the manual pair emits —
    /// and the region is proven: every path through the body reaches the release
    /// (`[context.escape]`), no branch enters it mid-region
    /// (`[context.entry-skip]`), and it is not already active
    /// (`[context.reacquire]`).
    ///
    /// `cond` is the comptime gate the ported corpus needs (the OFF-build-only
    /// bus fence in `vblank.emp` / `section.emp` brackets code the ON build runs
    /// unbracketed): when it evaluates FALSE the body lowers verbatim with no
    /// acquire, no release, and no region — so one body serves both shapes
    /// without duplication. `None` is the unconditional bracket.
    With {
        /// The context's name.
        ctx: String,
        /// The comptime gate, if the bracket is conditional.
        cond: Option<Expr>,
        /// The bracketed statements.
        body: Vec<AsmStmt>,
        /// Span of the `with <ctx>` header (the diagnostics' anchor).
        span: Span,
    },
    /// `invoke Iface.hook` (L1) — an engine-side call of a game-implemented hook.
    /// Lowers to an absolute `jsr <bound-proc>` (abs.l, placement-independent by
    /// rule) when the hook is bound, and to ZERO bytes when it is `empty`/unbound.
    /// The binding is resolved at lowering from the interface environment the
    /// bind pass produced ([`crate::resolve::contract`]).
    Invoke {
        /// The interface name (`Iface`).
        iface: String,
        /// The hook member name (`hook`).
        member: String,
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
    /// An `as ContractType` dispatch bound on this instruction (contract-grammar
    /// v2 §4): `jsr (a1) as ObjRoutine` names the contract every installable
    /// target of this indirect call must satisfy, so the closure uses that
    /// bound's clobbers instead of ⊤. `None` for an unannotated instruction.
    /// Emits nothing (metadata for the contract closure + subcontract checks).
    pub dispatch_bound: Option<String>,
    /// A trailing `@discards(name)` attribute on a call (contract-grammar v2 §6 /
    /// §11 Q3): the explicit, greppable opt-out of the flag-result must-use check
    /// (`[call.flag-result-unused]`) for a callee declaring `out(carry: name)`.
    /// `Some(name)` names the discarded flag-result; `None` on an unannotated
    /// instruction. Emits nothing (metadata for the caller-side check).
    pub discards: Option<String>,
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
        /// A `:b`/`:w`/`:l` sized override on a typed field displacement
        /// (`Sst.prev_anim:l(a1)`) — DECLARES the intended access width for a
        /// deliberate multi-field overlay write, replacing the field's own size
        /// in the `[operand.field-overrun]` check (bounded by the struct end).
        /// `None` for ordinary displacements. Distinct from the trailing
        /// instruction size; it binds to the field via `:`, not `.`.
        field_size_override: Option<TextOrSplice>,
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
