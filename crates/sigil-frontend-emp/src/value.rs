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
    },
    /// A run of width-1 bytes (from `byte`/`bytes`/`++`). Single bytes have no
    /// byte order, so this stays CPU-neutral as raw bytes.
    Bytes(Vec<u8>),
    /// A pointer-typed field: a reference to a named symbol, `width` bytes wide
    /// (4, the Abs32 default — D-P3.7). Plan 4 resolves the name to an address
    /// and emits a fixup; Plan 3 does NOT.
    SymRef {
        /// The referenced symbol's name.
        name: String,
        /// Pointer byte width (4).
        width: u8,
    },
}

impl Cell {
    /// The cell's byte size: a scalar/symref is its `width`, a byte run is its
    /// length.
    pub fn byte_size(&self) -> usize {
        match self {
            Cell::Scalar { width, .. } | Cell::SymRef { width, .. } => *width as usize,
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
        assert_eq!(Value::Data(DataBuf::empty()).type_name(), "data");
        assert_eq!(Value::Poison.type_name(), "poison");
    }

    #[test]
    fn databuf_monoid_and_display() {
        let mut a = DataBuf::empty();
        a.push(Cell::Scalar { value: 5, width: 1, signed: false });
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
