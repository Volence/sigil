//! The deterministic JSON writer for [`BuildExport`]. Every object is built as a
//! `serde_json::Map` in an explicit key order (the crate's `preserve_order`
//! feature keeps insertion order), so nothing depends on derive order or hash
//! iteration. Output is compact, ends in one `\n`, and holds only decimal integers
//! (no hex strings).

use super::*;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

/// SHA-256 of a byte slice.
pub fn sha256_of(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

/// 64 lowercase hex digits.
pub fn sha256_hex(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    Value::Object(m)
}

fn field(f: &FieldRecord) -> Value {
    obj(vec![
        ("name", json!(f.name)),
        ("offset", json!(f.offset)),
        ("size", json!(f.size)),
        ("type", json!(f.type_)),
    ])
}

fn symbol(s: &SymbolRecord) -> Value {
    let mut p = vec![("name", json!(s.name))];
    if let Some(d) = &s.demangled {
        p.push(("demangled", json!(d)));
    }
    if let Some(sp) = s.space {
        p.push(("space", json!(sp.as_str())));
    }
    p.push(("vma", json!(s.vma)));
    if let Some(l) = s.lma {
        p.push(("lma", json!(l)));
    }
    p.push(("kind", json!(s.kind.as_str())));
    if let Some(z) = s.size {
        p.push(("size", json!(z)));
    }
    if let Some(m) = &s.module {
        p.push(("module", json!(m)));
    }
    p.push(("public", json!(s.public)));
    obj(p)
}

fn layout(l: &LayoutRecord) -> Value {
    obj(vec![
        ("name", json!(l.name)),
        ("module", json!(l.module)),
        ("kind", json!(l.kind.as_str())),
        ("size", json!(l.size)),
        ("fields", Value::Array(l.fields.iter().map(field).collect())),
        (
            "overlays",
            Value::Array(
                l.overlays
                    .iter()
                    .map(|o| {
                        obj(vec![
                            ("name", json!(o.name)),
                            ("base", json!(o.base)),
                            ("offset", json!(o.offset)),
                            ("size", json!(o.size)),
                            ("fields", Value::Array(o.fields.iter().map(field).collect())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// Why an export cannot be rendered. A keyed JSON object cannot hold two entries under
/// one key, and silently keeping one would drop rows the completeness gate counts on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportError {
    DuplicateKey { what: &'static str, key: String },
    /// Image bytes no row covers: a fragment whose span names no file, or a gap in the
    /// image that no section and no declared no-source range accounts for.
    Uncovered { section: String, lma: u32, len: u32 },
    /// Two rows (or a no-source range and a section) claim the image byte at `lma`.
    Overlap { lma: u32 },
    /// A section still holds a width-deferred fragment at the walk point.
    Unlowered { section: String },
    /// A byte-emitting section whose load address is not an image address.
    ForeignSpace { section: String },
    /// A located source file with no row in `files`, named as its front end names it.
    FileNotInTable { path: String, why: String },
    /// Two `files` rows with one `(root, path)`.
    DuplicateFile { root: &'static str, path: String },
    /// A front end's instruction mark that names no single placed `Data` run.
    CodeMark { section: String, why: String },
    /// One `(module, name)` resolved to two different layouts (A4's hazard): the export
    /// would state two answers to one question, so it states none.
    TwoLayouts { module: String, name: String },
    /// One `(module, name)` resolved to two different constants.
    TwoConsts { module: String, name: String },
    /// An exported constant whose value does not fit `i64` (never dropped, never wrapped).
    ConstOutOfRange { module: String, name: String, value: String },
    /// A `pub const` that did not evaluate in its own module.
    Const { module: String, name: String, why: String },
    /// A layout the record cannot state (a size past `u32`, an overlay with no unique base).
    Layout { module: String, name: String, why: String },
    /// An exporter invariant that does not hold (a bug, not data).
    Internal(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::DuplicateKey { what, key } => write!(f, "duplicate {what} key `{key}` in the export"),
            ExportError::Uncovered { section, lma, len } => write!(
                f,
                "line table: {len} byte(s) at ${lma:X} (section `{section}`) have no source line and no declared no-source range"
            ),
            ExportError::Overlap { lma } => write!(f, "line table: two rows claim the image byte at ${lma:X}"),
            ExportError::Unlowered { section } => {
                write!(f, "line table: section `{section}` still holds a width-deferred fragment at the walk point")
            }
            ExportError::ForeignSpace { section } => write!(
                f,
                "line table: section `{section}` stores bytes at an address outside the image space, which no row can state"
            ),
            ExportError::FileNotInTable { path, why } => {
                write!(f, "line table: source file {path} wrote ROM bytes but has no row in `files`: {why}")
            }
            ExportError::DuplicateFile { root, path } => write!(f, "files: `{root}/{path}` is listed twice"),
            ExportError::CodeMark { section, why } => {
                write!(f, "line table: an instruction mark for section `{section}` cannot be placed: {why}")
            }
            ExportError::TwoLayouts { module, name } => write!(
                f,
                "layouts: `{module}::{name}` resolved to two different layouts, so the export cannot state one"
            ),
            ExportError::TwoConsts { module, name } => write!(
                f,
                "consts: `{module}::{name}` resolved to two different values, so the export cannot state one"
            ),
            ExportError::ConstOutOfRange { module, name, value } => {
                write!(f, "consts: `{module}::{name}` = {value} does not fit a signed 64-bit integer")
            }
            ExportError::Const { module, name, why } => write!(f, "consts: `{module}::{name}`: {why}"),
            ExportError::Layout { module, name, why } => write!(f, "layouts: `{module}::{name}`: {why}"),
            ExportError::Internal(m) => write!(f, "build export internal error: {m}"),
        }
    }
}

impl std::error::Error for ExportError {}

fn lines(t: &LineTable) -> Result<Value, ExportError> {
    let mut secs: Vec<&LineSection> = t.sections.iter().collect();
    secs.sort_by(|a, b| a.name.cmp(&b.name));
    let mut m = Map::new();
    for s in secs {
        if m.contains_key(&s.name) {
            return Err(ExportError::DuplicateKey { what: "line section", key: s.name.clone() });
        }
        let mut rows: Vec<&LineRow> = s.rows.iter().collect();
        rows.sort_by_key(|r| r.lma); // stable: equal lma keeps model order
        let rows: Vec<Value> = rows
            .iter()
            .map(|r| json!([r.vma, r.lma, r.len, r.file, r.line, r.kind.as_str()]))
            .collect();
        m.insert(s.name.clone(), obj(vec![("space", json!(s.space.as_str())), ("rows", Value::Array(rows))]));
    }
    Ok(obj(vec![
        ("columns", json!(["vma", "lma", "len", "file", "line", "kind"])),
        ("sections", Value::Object(m)),
    ]))
}

/// Render the export: keys in the order of spec §4.6, collections sorted
/// (symbols by `(space, vma, name)`, files by `(root, path)`, layouts and consts by
/// `(module, name)`, line rows by `lma`), compact, final newline. Refuses a duplicate
/// line-section name or define name rather than dropping one.
pub fn to_json(e: &BuildExport) -> Result<String, ExportError> {
    let mut symbols: Vec<&SymbolRecord> = e.symbols.iter().collect();
    symbols.sort_by(|a, b| (a.space, a.vma, &a.name).cmp(&(b.space, b.vma, &b.name)));
    let mut files: Vec<&FileRecord> = e.files.iter().collect();
    files.sort_by(|a, b| (a.root, &a.path).cmp(&(b.root, &b.path)));
    let mut layouts: Vec<&LayoutRecord> = e.layouts.iter().collect();
    layouts.sort_by(|a, b| (&a.module, &a.name).cmp(&(&b.module, &b.name)));
    let mut consts: Vec<&ConstRecord> = e.consts.iter().collect();
    consts.sort_by(|a, b| (&a.module, &a.name).cmp(&(&b.module, &b.name)));

    let mut defines = Map::new();
    let mut d: Vec<&(String, i64)> = e.shape.defines.iter().collect();
    d.sort();
    for (k, v) in d {
        if defines.contains_key(k) {
            return Err(ExportError::DuplicateKey { what: "define", key: k.clone() });
        }
        defines.insert(k.clone(), json!(v));
    }

    let root = obj(vec![
        ("format", json!("empyrean.build")),
        ("version", obj(vec![("major", json!(1)), ("minor", json!(0))])),
        (
            "rom",
            obj(vec![
                ("path", json!(e.rom.path)),
                ("size", json!(e.rom.size)),
                ("sha256", json!(sha256_hex(&e.rom.sha256))),
            ]),
        ),
        ("inputsSha256", json!(sha256_hex(&e.inputs_sha256))),
        (
            "tool",
            obj(vec![
                ("name", json!(e.tool.name)),
                ("version", json!(e.tool.version)),
                ("revision", json!(e.tool.revision)),
                ("tree", json!(e.tool.tree)),
            ]),
        ),
        (
            "shape",
            obj(vec![
                ("game", json!(e.shape.game)),
                ("shape", json!(e.shape.shape)),
                ("defines", Value::Object(defines)),
            ]),
        ),
        (
            "spaces",
            Value::Array(
                e.spaces
                    .iter()
                    .map(|s| obj(vec![("id", json!(s.id.as_str())), ("addrBits", json!(s.addr_bits))]))
                    .collect(),
            ),
        ),
        (
            "files",
            Value::Array(
                files
                    .iter()
                    .map(|f| {
                        obj(vec![
                            ("root", json!(f.root.as_str())),
                            ("path", json!(f.path)),
                            ("lang", json!(f.lang.as_str())),
                            ("sha256", json!(sha256_hex(&f.sha256))),
                            ("size", json!(f.size)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("symbols", Value::Array(symbols.iter().map(|s| symbol(s)).collect())),
        ("layouts", Value::Array(layouts.iter().map(|l| layout(l)).collect())),
        (
            "consts",
            Value::Array(
                consts
                    .iter()
                    .map(|c| {
                        obj(vec![
                            ("name", json!(c.name)),
                            ("module", json!(c.module)),
                            ("type", json!(c.type_)),
                            ("value", json!(c.value)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("lines", lines(&e.lines)?),
    ]);
    let mut s = serde_json::to_string(&root).expect("a Value always serialises");
    s.push('\n');
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BuildExport {
        let sym = |name: &str, vma: u32| SymbolRecord {
            name: name.to_string(),
            demangled: None,
            space: Some(Space::M68k),
            vma,
            lma: Some(vma),
            kind: SymbolKind::Label,
            size: None,
            module: None,
            public: false,
        };
        BuildExport {
            rom: RomRecord { path: "s4.bin".into(), size: 4, sha256: sha256_of(b"abcd") },
            inputs_sha256: [0; 32],
            tool: ToolRecord { name: "sigil".into(), version: "0.1.0".into(), revision: "r".into(), tree: "clean".into() },
            shape: ShapeRecord { game: "sonic4".into(), shape: "s4".into(), defines: vec![("DEBUG".into(), 0)] },
            spaces: vec![
                SpaceRecord { id: Space::M68k, addr_bits: 24 },
                SpaceRecord { id: Space::Z80, addr_bits: 16 },
            ],
            files: vec![],
            // Deliberately out of order: the writer must sort.
            symbols: vec![sym("Zed", 0x200), sym("Alpha", 0x100)],
            layouts: vec![],
            consts: vec![],
            lines: LineTable {
                sections: vec![LineSection {
                    name: "rom".into(),
                    space: Space::M68k,
                    rows: vec![
                        LineRow { vma: 0x200, lma: 0x200, len: 2, file: 0, line: 0, kind: RowKind::Data },
                        LineRow { vma: 0x100, lma: 0x100, len: 0x100, file: 0, line: 0, kind: RowKind::NoSource(NoSourceKind::Fill) },
                    ],
                }],
            },
        }
    }

    #[test]
    fn to_json_is_deterministic_and_ordered() {
        let e = sample();
        let a = to_json(&e).unwrap();
        let b = to_json(&e).unwrap();
        assert_eq!(a, b);
        assert!(
            a.starts_with(r#"{"format":"empyrean.build","version":{"major":1,"minor":0},"rom":"#),
            "{a}"
        );
        assert!(a.ends_with("}\n"));
        assert!(!a.contains("0x") && !a.contains('$'), "{a}");
        let v: serde_json::Value = serde_json::from_str(&a).expect("round-trips");
        let syms = v["symbols"].as_array().unwrap();
        assert_eq!(syms[0]["name"], "Alpha");
        assert_eq!(syms[1]["name"], "Zed");
        let rows = v["lines"]["sections"]["rom"]["rows"].as_array().unwrap();
        assert_eq!(rows[0][5], "fill");
        assert_eq!(rows[1][5], "data");
        assert_eq!(v["rom"]["sha256"].as_str().unwrap().len(), 64);
        let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            ["format", "version", "rom", "inputsSha256", "tool", "shape", "spaces", "files", "symbols", "layouts", "consts", "lines"]
        );
    }

    #[test]
    fn to_json_refuses_duplicate_section_and_define_keys() {
        let mut e = sample();
        let dup = e.lines.sections[0].clone();
        e.lines.sections.push(dup);
        assert_eq!(
            to_json(&e),
            Err(ExportError::DuplicateKey { what: "line section", key: "rom".into() })
        );
        let mut e = sample();
        e.shape.defines.push(("DEBUG".into(), 1));
        assert_eq!(to_json(&e), Err(ExportError::DuplicateKey { what: "define", key: "DEBUG".into() }));
    }
}
