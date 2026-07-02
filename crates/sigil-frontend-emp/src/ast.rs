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

/// An `enum Name: repr { variants... }` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    /// Whether this enum is exported (`pub enum`).
    pub public: bool,
    /// The enum's name.
    pub name: String,
    /// The underlying representation type, e.g. `u8` in `enum Anim: u8`.
    pub repr: Type,
    /// Variants as `(name, optional explicit value, span)`.
    pub variants: Vec<(String, Option<Expr>, Span)>,
    /// Span of the whole declaration.
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

/// A `vars [name:] region { fields... }` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct VarsDecl {
    /// Whether this vars block is exported (`pub vars`).
    pub public: bool,
    /// `vars upper_ram { .. }` → name None, region "upper_ram".
    /// `vars PitcherPlantV: sst_custom { .. }` → name Some("PitcherPlantV"), region "sst_custom".
    pub name: Option<String>,
    /// The memory region this block is allocated into.
    pub region: String,
    /// The block's fields, in declaration order.
    pub fields: Vec<VarsField>,
    /// Span of the whole declaration.
    pub span: Span,
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
