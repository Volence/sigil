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
    /// `comptime fn ...` declaration.
    ComptimeFn(ComptimeFnDecl),
    /// `section ...` declaration.
    Section(SectionDecl),
    /// `newtype ...` declaration.
    Newtype(NewtypeDecl),
    /// An item-position `ensure(...)` / `ensure_fatal(...)` guard (§6.5, D5.1).
    Ensure(EnsureDecl),
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
    /// The target label reference (a path expression).
    pub target: Expr,
    /// Span of the whole member.
    pub span: Span,
}

/// A `dispatch Name (encoding: E) { Member: target, ... }` block: an
/// encoding-agnostic typed state-dispatch table (D6.B1). Forward: emits a
/// code-pointer table per `encoding` (later task). Reverse: introduces the
/// pre-scaled comptime ordinal constants `Name.Member` and `Name.count`
/// (D6.B3, later task). The member grammar deliberately mirrors
/// [`OffsetsDecl`]'s `Name: target` shape; `Member: { ... }` (inline body /
/// scripted state) is reserved for a future backlog item (#9) and is a
/// parse error here, not a silently-accepted alternate form.
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

/// One `Member: target` entry of a [`DispatchDecl`].
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchMember {
    /// The member's name (`Name.Member`).
    pub name: String,
    /// The target label reference (a path expression).
    pub target: Expr,
    /// Span of the whole member.
    pub span: Span,
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
    /// Registers this proc clobbers.
    pub clobbers: Vec<String>,
    /// The proc this one falls into, if any.
    pub falls_into: Option<String>,
    /// The proc's assembly body.
    pub body: Vec<AsmStmt>,
    /// Span of the whole declaration.
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
    /// A struct literal: `Ty { field: value, ... }`.
    StructLit {
        /// The struct's type path.
        ty: Path,
        /// Field initializers as `(name, value)`.
        fields: Vec<(String, Expr)>,
        /// Span of the whole expression.
        span: Span,
    },
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
