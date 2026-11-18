//! The build-data export model: what one `sigil build` knows about the ROM it just
//! wrote, as plain data (spec `G0a-build-data-export-design` §4.6). The model is
//! driver-independent; `json` renders it, `locate` is the span-to-file/line seam the
//! later tasks fill. Closed enums here are closed on purpose: a consumer matches them
//! exhaustively, and a new variant is a version change, not a silent addition.

pub mod entry;
pub mod inputs;
pub mod json;
pub mod lines;
pub mod locate;
pub mod placed;

pub use entry::{build_export, ExportInputs, FileKey, SideTables, Z80Bridge};
pub use inputs::{inputs_sha256, inputs_text, InputRead, ToolExe};
pub use json::{sha256_hex, sha256_of, to_json, ExportError};
pub use lines::{walk_lines, NoSourceRange, IMAGE_SECTION};
pub use placed::{section_space, PlacedLabel, PlacedSymbols, Z80Window};

/// Everything the export file holds. Field order is the JSON key order of §4.6.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildExport {
    pub rom: RomRecord,
    pub inputs_sha256: [u8; 32],
    pub tool: ToolRecord,
    pub shape: ShapeRecord,
    pub spaces: Vec<SpaceRecord>,
    pub files: Vec<FileRecord>,
    pub symbols: Vec<SymbolRecord>,
    pub layouts: Vec<LayoutRecord>,
    pub consts: Vec<ConstRecord>,
    pub lines: LineTable,
}

/// The ROM file as written: its name, length and full SHA-256 (the binding key).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RomRecord {
    pub path: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// An address space. Closed: serialises `"m68k"` / `"z80"`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Space {
    M68k,
    Z80,
}

impl Space {
    pub fn as_str(self) -> &'static str {
        match self {
            Space::M68k => "m68k",
            Space::Z80 => "z80",
        }
    }
}

/// One address space and its width in bits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpaceRecord {
    pub id: Space,
    pub addr_bits: u32,
}

/// The tool that wrote the file. `tree` is the build's tree-state word.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolRecord {
    pub name: String,
    pub version: String,
    pub revision: String,
    pub tree: String,
}

/// The shape the ROM was built in. `game` and `shape` come from the caller, never
/// from Sigil's source. `defines` are the shape's integer defines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeRecord {
    pub game: String,
    pub shape: String,
    pub defines: Vec<(String, i64)>,
}

/// The named root a source file's path is relative to. Closed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootName {
    Engine,
    Game,
    Generated,
    Sigil,
}

impl RootName {
    pub fn as_str(self) -> &'static str {
        match self {
            RootName::Engine => "engine",
            RootName::Game => "game",
            RootName::Generated => "generated",
            RootName::Sigil => "sigil",
        }
    }
}

/// A source file's language. Closed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Lang {
    Emp,
    Asm,
    Z80,
}

impl Lang {
    pub fn as_str(self) -> &'static str {
        match self {
            Lang::Emp => "emp",
            Lang::Asm => "asm",
            Lang::Z80 => "z80",
        }
    }
}

/// One input file: root-relative path, language, content hash and length.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRecord {
    pub root: RootName,
    pub path: String,
    pub lang: Lang,
    pub sha256: [u8; 32],
    pub size: u64,
}

/// What a symbol is. Closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Proc,
    Label,
    Data,
    Equ,
    Ram,
}

impl SymbolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Proc => "proc",
            SymbolKind::Label => "label",
            SymbolKind::Data => "data",
            SymbolKind::Equ => "equ",
            SymbolKind::Ram => "ram",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolRecord {
    pub name: String,
    pub demangled: Option<String>,
    pub space: Option<Space>,
    pub vma: u32,
    pub lma: Option<u32>,
    pub kind: SymbolKind,
    pub size: Option<u32>,
    pub module: Option<String>,
    pub public: bool,
}

/// What a layout describes: `"struct"`, `"bitfield"` or `"enum"`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutKind {
    Struct,
    Bitfield,
    Enum,
}

impl LayoutKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LayoutKind::Struct => "struct",
            LayoutKind::Bitfield => "bitfield",
            LayoutKind::Enum => "enum",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRecord {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub type_: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverlayRecord {
    pub name: String,
    pub base: String,
    pub offset: u32,
    pub size: u32,
    pub fields: Vec<FieldRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutRecord {
    pub name: String,
    pub module: String,
    pub kind: LayoutKind,
    pub size: u32,
    pub fields: Vec<FieldRecord>,
    pub overlays: Vec<OverlayRecord>,
}

/// A constant (1.0: integer, boolean, enum only).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstRecord {
    pub name: String,
    pub module: String,
    pub type_: String,
    pub value: i64,
}

/// Why a row has no source line. Closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoSourceKind {
    Fill,
    OrgGap,
    Deb2Appendix,
    HeaderChecksum,
}

/// What a line row covers. Closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowKind {
    Code,
    Data,
    NoSource(NoSourceKind),
}

impl RowKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RowKind::Code => "code",
            RowKind::Data => "data",
            RowKind::NoSource(NoSourceKind::Fill) => "fill",
            RowKind::NoSource(NoSourceKind::OrgGap) => "org-gap",
            RowKind::NoSource(NoSourceKind::Deb2Appendix) => "deb2",
            RowKind::NoSource(NoSourceKind::HeaderChecksum) => "checksum",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineRow {
    pub vma: u32,
    pub lma: u32,
    pub len: u32,
    /// Index into `BuildExport::files`.
    pub file: u32,
    pub line: u32,
    pub kind: RowKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineSection {
    pub name: String,
    pub space: Space,
    pub rows: Vec<LineRow>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LineTable {
    pub sections: Vec<LineSection>,
}
