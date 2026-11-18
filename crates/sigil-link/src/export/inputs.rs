//! `inputsSha256`: one hash over everything the build's output is a function of that
//! the export itself records (spec §4.2). It is named for what it is: it changes when
//! the tool changes even if no source does, so G0b's old-versus-new comparison leaves
//! it and the `tool` block out.
//!
//! The hashed text is canonical and line-oriented, one line per item, so a reader can
//! reproduce it from the export alone:
//!
//! ```text
//! empyrean.build.inputs 1
//! tool name=<n> version=<v> revision=<r> tree=<t>
//! shape game=<g> shape=<s>
//! define <NAME>=<value>            (one per define, sorted by name)
//! file <root> <path> <lang> <sha256> <size>   (one per file, sorted by root then path)
//! ```
//!
//! Nothing about WHERE the build ran or WHAT the output was called is in it: no output
//! path, no listing path, no ROM hash. The ROM hash is the binding key and is the
//! result of these inputs, not one of them.

use super::{sha256_hex, sha256_of, FileRecord, ShapeRecord, ToolRecord};

/// The canonical text [`inputs_sha256`] hashes.
pub fn inputs_text(
    tool: &ToolRecord,
    shape: &ShapeRecord,
    defines: &[(String, i128)],
    files: &[FileRecord],
) -> String {
    let mut out = String::from("empyrean.build.inputs 1\n");
    out.push_str(&format!(
        "tool name={} version={} revision={} tree={}\n",
        tool.name, tool.version, tool.revision, tool.tree
    ));
    out.push_str(&format!("shape game={} shape={}\n", shape.game, shape.shape));
    let mut defs: Vec<&(String, i128)> = defines.iter().collect();
    defs.sort();
    for (name, value) in defs {
        out.push_str(&format!("define {name}={value}\n"));
    }
    let mut fs: Vec<&FileRecord> = files.iter().collect();
    fs.sort_by(|a, b| (a.root, &a.path).cmp(&(b.root, &b.path)));
    for f in fs {
        out.push_str(&format!(
            "file {} {} {} {} {}\n",
            f.root.as_str(),
            f.path,
            f.lang.as_str(),
            sha256_hex(&f.sha256),
            f.size
        ));
    }
    out
}

/// SHA-256 over [`inputs_text`]. `defines` are `i128` because that is what the shape
/// resolver yields; the hash needs no narrowing, so none happens here.
pub fn inputs_sha256(
    tool: &ToolRecord,
    shape: &ShapeRecord,
    defines: &[(String, i128)],
    files: &[FileRecord],
) -> [u8; 32] {
    sha256_of(inputs_text(tool, shape, defines, files).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    fn fixture() -> (ToolRecord, ShapeRecord, Vec<(String, i128)>, Vec<FileRecord>) {
        let tool = ToolRecord { name: "sigil".into(), version: "0.1.0".into(), revision: "abc".into(), tree: "clean-sources".into() };
        let shape = ShapeRecord { game: "demo".into(), shape: "demo plain".into(), defines: vec![] };
        let defines = vec![("B".to_string(), 2i128), ("A".to_string(), 1)];
        let f = |root, path: &str, b: &[u8]| FileRecord {
            root,
            path: path.into(),
            lang: Lang::Emp,
            sha256: sha256_of(b),
            size: b.len() as u64,
        };
        (tool, shape, defines, vec![f(RootName::Game, "demo/a.emp", b"a"), f(RootName::Engine, "z.emp", b"z")])
    }

    #[test]
    fn order_of_inputs_does_not_matter_but_content_does() {
        let (t, s, d, f) = fixture();
        let base = inputs_sha256(&t, &s, &d, &f);
        let mut d2 = d.clone();
        d2.reverse();
        let mut f2 = f.clone();
        f2.reverse();
        assert_eq!(base, inputs_sha256(&t, &s, &d2, &f2), "sorted, so input order is not an input");
        let mut f3 = f.clone();
        f3[0].sha256 = sha256_of(b"b");
        assert_ne!(base, inputs_sha256(&t, &s, &d, &f3), "a file hash must move it");
        let mut d3 = d.clone();
        d3[0].1 = 3;
        assert_ne!(base, inputs_sha256(&t, &s, &d3, &f), "a define must move it");
        let mut t2 = t.clone();
        t2.revision = "abd".into();
        assert_ne!(base, inputs_sha256(&t2, &s, &d, &f), "the tool must move it");
        let mut s2 = s.clone();
        s2.shape = "demo debug".into();
        assert_ne!(base, inputs_sha256(&t, &s2, &d, &f), "the shape must move it");
    }

    #[test]
    fn the_hashed_text_is_the_documented_one() {
        let (t, s, d, f) = fixture();
        let text = inputs_text(&t, &s, &d, &f);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "empyrean.build.inputs 1");
        assert_eq!(lines[1], "tool name=sigil version=0.1.0 revision=abc tree=clean-sources");
        assert_eq!(lines[2], "shape game=demo shape=demo plain");
        assert_eq!(&lines[3..5], ["define A=1", "define B=2"]);
        assert!(lines[5].starts_with("file engine z.emp emp "), "{}", lines[5]);
        assert!(lines[6].starts_with("file game demo/a.emp emp "), "{}", lines[6]);
        assert_eq!(lines.len(), 7);
    }

    #[test]
    fn the_span_recorders_sha256_agrees_with_sha2() {
        for n in [0usize, 1, 55, 56, 63, 64, 65, 1000] {
            let b: Vec<u8> = (0..n).map(|i| (i * 31 + 7) as u8).collect();
            assert_eq!(sigil_span::read_set::sha256(&b), sha256_of(&b), "len {n}");
        }
    }
}
