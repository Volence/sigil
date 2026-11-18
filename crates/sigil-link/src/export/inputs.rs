//! `inputsSha256`: one hash over everything the build's output is a function of that
//! the export can name (spec §4.2: "the tool identity, shape, defines and every input
//! file's SHA-256"). It is named for what it is: it changes when the tool changes even
//! if no source does, so G0b's old-versus-new comparison leaves it and the `tool` block
//! out.
//!
//! It covers MORE than the `files` table. The table holds source rows only (`lang` is a
//! closed enum), but the hash covers every read the build made that lies under a root:
//! sources, art and sound blobs, `map.toml`. A changed blob therefore moves the hash
//! even though no file row changes. Reads that lie under no root but are still build
//! inputs (the aeon `art/` directory) are covered too, under the hash-only root name
//! `aeon`, which never appears in the schema; a data file handed to the build from outside both trees (the anchor overlay) is covered as `external`, by file name only. Helper executables the build runs
//! (`convsym`) are tool identity: their file name and SHA-256, never a path.
//!
//! The hashed text is canonical and line-oriented, one line per item, so a reader can
//! reproduce it from the export alone:
//!
//! ```text
//! empyrean.build.inputs 2
//! tool name=<n> version=<v> revision=<r> tree=<t>
//! toolexe <file name> <sha256>      (one per executable, sorted by name)
//! shape game=<g> shape=<s>
//! define <NAME>=<value>             (one per define, sorted by name)
//! input <root> <path> <sha256>      (one per read, sorted by root then path)
//! ```
//!
//! Nothing about WHERE the build ran or WHAT the output was called is in it: no output
//! path, no listing path, no ROM hash. The ROM hash is the binding key and is the
//! result of these inputs, not one of them.

use super::{sha256_hex, sha256_of, ShapeRecord, ToolRecord};

/// One file the build read, named by root and root-relative path. `root` is one of the
/// four schema roots, or `aeon` for a hash-only read under no root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputRead {
    pub root: String,
    pub path: String,
    pub sha256: [u8; 32],
}

/// A helper executable the build ran: its file name (no directory) and content hash.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExe {
    pub name: String,
    pub sha256: [u8; 32],
}

/// The canonical text [`inputs_sha256`] hashes.
pub fn inputs_text(
    tool: &ToolRecord,
    tool_exes: &[ToolExe],
    shape: &ShapeRecord,
    defines: &[(String, i128)],
    inputs: &[InputRead],
) -> String {
    let mut out = String::from("empyrean.build.inputs 2\n");
    out.push_str(&format!(
        "tool name={} version={} revision={} tree={}\n",
        tool.name, tool.version, tool.revision, tool.tree
    ));
    let mut exes: Vec<&ToolExe> = tool_exes.iter().collect();
    exes.sort_by(|a, b| (&a.name, a.sha256).cmp(&(&b.name, b.sha256)));
    for e in exes {
        out.push_str(&format!("toolexe {} {}\n", e.name, sha256_hex(&e.sha256)));
    }
    out.push_str(&format!("shape game={} shape={}\n", shape.game, shape.shape));
    let mut defs: Vec<&(String, i128)> = defines.iter().collect();
    defs.sort();
    for (name, value) in defs {
        out.push_str(&format!("define {name}={value}\n"));
    }
    let mut reads: Vec<&InputRead> = inputs.iter().collect();
    reads.sort_by(|a, b| (&a.root, &a.path).cmp(&(&b.root, &b.path)));
    for r in reads {
        out.push_str(&format!("input {} {} {}\n", r.root, r.path, sha256_hex(&r.sha256)));
    }
    out
}

/// SHA-256 over [`inputs_text`]. `defines` are `i128` because that is what the shape
/// resolver yields; the hash needs no narrowing, so none happens here.
pub fn inputs_sha256(
    tool: &ToolRecord,
    tool_exes: &[ToolExe],
    shape: &ShapeRecord,
    defines: &[(String, i128)],
    inputs: &[InputRead],
) -> [u8; 32] {
    sha256_of(inputs_text(tool, tool_exes, shape, defines, inputs).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    type Fx = (ToolRecord, Vec<ToolExe>, ShapeRecord, Vec<(String, i128)>, Vec<InputRead>);

    fn fixture() -> Fx {
        let tool = ToolRecord { name: "sigil".into(), version: "0.1.0".into(), revision: "abc".into(), tree: "clean-sources".into() };
        let exes = vec![ToolExe { name: "convsym".into(), sha256: sha256_of(b"exe") }];
        let shape = ShapeRecord { game: "demo".into(), shape: "demo plain".into(), defines: vec![] };
        let defines = vec![("B".to_string(), 2i128), ("A".to_string(), 1)];
        let r = |root: &str, path: &str, b: &[u8]| InputRead { root: root.into(), path: path.into(), sha256: sha256_of(b) };
        let inputs = vec![r("game", "demo/map.toml", b"a"), r("engine", "data/sine.bin", b"z"), r("aeon", "art/x.bin", b"q")];
        (tool, exes, shape, defines, inputs)
    }

    #[test]
    fn order_of_inputs_does_not_matter_but_content_does() {
        let (t, e, s, d, i) = fixture();
        let base = inputs_sha256(&t, &e, &s, &d, &i);
        let (mut d2, mut i2) = (d.clone(), i.clone());
        d2.reverse();
        i2.reverse();
        assert_eq!(base, inputs_sha256(&t, &e, &s, &d2, &i2), "sorted, so input order is not an input");
        for k in 0..i.len() {
            let mut i3 = i.clone();
            i3[k].sha256 = sha256_of(b"changed");
            assert_ne!(base, inputs_sha256(&t, &e, &s, &d, &i3), "read {k} (blob/toml/unrooted) must move it");
        }
        let mut d3 = d.clone();
        d3[0].1 = 3;
        assert_ne!(base, inputs_sha256(&t, &e, &s, &d3, &i), "a define must move it");
        let mut t2 = t.clone();
        t2.revision = "abd".into();
        assert_ne!(base, inputs_sha256(&t2, &e, &s, &d, &i), "the tool must move it");
        let e2 = vec![ToolExe { name: "convsym".into(), sha256: sha256_of(b"exe2") }];
        assert_ne!(base, inputs_sha256(&t, &e2, &s, &d, &i), "the helper executable must move it");
        let mut s2 = s.clone();
        s2.shape = "demo debug".into();
        assert_ne!(base, inputs_sha256(&t, &e, &s2, &d, &i), "the shape must move it");
    }

    #[test]
    fn the_hashed_text_is_the_documented_one() {
        let (t, e, s, d, i) = fixture();
        let text = inputs_text(&t, &e, &s, &d, &i);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "empyrean.build.inputs 2");
        assert_eq!(lines[1], "tool name=sigil version=0.1.0 revision=abc tree=clean-sources");
        assert!(lines[2].starts_with("toolexe convsym "), "{}", lines[2]);
        assert_eq!(lines[3], "shape game=demo shape=demo plain");
        assert_eq!(&lines[4..6], ["define A=1", "define B=2"]);
        assert!(lines[6].starts_with("input aeon art/x.bin "), "{}", lines[6]);
        assert!(lines[7].starts_with("input engine data/sine.bin "), "{}", lines[7]);
        assert!(lines[8].starts_with("input game demo/map.toml "), "{}", lines[8]);
        assert_eq!(lines.len(), 9);
        assert!(!text.contains("/home") && !text.lines().any(|l| l.contains(" /")), "no absolute path");
    }

    #[test]
    fn the_span_recorders_sha256_agrees_with_sha2() {
        for n in [0usize, 1, 55, 56, 63, 64, 65, 1000] {
            let b: Vec<u8> = (0..n).map(|i| (i * 31 + 7) as u8).collect();
            assert_eq!(sigil_span::read_set::sha256(&b), sha256_of(&b), "len {n}");
        }
    }
}
