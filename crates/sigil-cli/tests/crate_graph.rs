//! Integration test: enforce the one-way workspace crate graph.
//!
//! Rules verified:
//!   (a) `sigil-isa` has zero workspace (`sigil-*`) dependencies — extraction-ready
//!   (b) `sigil-ir` depends only on `sigil-span`
//!   (c) only `sigil-cli` may depend on `sigil-frontend-as` — contamination safeguard
//!
//! Parses `cargo metadata` JSON with a minimal hand parser; no external crates.

use std::collections::BTreeMap;
use std::process::Command;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        if let Json::Obj(pairs) = self {
            for (k, v) in pairs {
                if k == key {
                    return Some(v);
                }
            }
        }
        None
    }
    fn as_array(&self) -> Option<&Vec<Json>> {
        if let Json::Arr(a) = self {
            Some(a)
        } else {
            None
        }
    }
    fn as_str(&self) -> Option<&str> {
        if let Json::Str(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser { b: s.as_bytes(), i: 0 }
    }

    fn ws(&mut self) {
        while self.i < self.b.len() {
            match self.b[self.i] {
                b' ' | b'\t' | b'\n' | b'\r' => self.i += 1,
                _ => break,
            }
        }
    }

    fn value(&mut self) -> Json {
        self.ws();
        match self.b[self.i] {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Json::Str(self.string()),
            b't' => {
                self.i += 4;
                Json::Bool(true)
            }
            b'f' => {
                self.i += 5;
                Json::Bool(false)
            }
            b'n' => {
                self.i += 4;
                Json::Null
            }
            _ => self.number(),
        }
    }

    fn object(&mut self) -> Json {
        let mut out = Vec::new();
        self.i += 1; // consume '{'
        self.ws();
        if self.b[self.i] == b'}' {
            self.i += 1;
            return Json::Obj(out);
        }
        loop {
            self.ws();
            let key = self.string();
            self.ws();
            self.i += 1; // consume ':'
            let val = self.value();
            out.push((key, val));
            self.ws();
            match self.b[self.i] {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    break;
                }
                _ => break,
            }
        }
        Json::Obj(out)
    }

    fn array(&mut self) -> Json {
        let mut out = Vec::new();
        self.i += 1; // consume '['
        self.ws();
        if self.b[self.i] == b']' {
            self.i += 1;
            return Json::Arr(out);
        }
        loop {
            let val = self.value();
            out.push(val);
            self.ws();
            match self.b[self.i] {
                b',' => self.i += 1,
                b']' => {
                    self.i += 1;
                    break;
                }
                _ => break,
            }
        }
        Json::Arr(out)
    }

    fn string(&mut self) -> String {
        let mut out = String::new();
        self.i += 1; // consume opening '"'
        while self.i < self.b.len() {
            let c = self.b[self.i];
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.b[self.i];
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'u' => {
                            let end = (self.i + 4).min(self.b.len());
                            let hex = std::str::from_utf8(&self.b[self.i..end]).unwrap_or("");
                            if let Ok(code) = u32::from_str_radix(hex, 16) {
                                if let Some(ch) = char::from_u32(code) {
                                    out.push(ch);
                                }
                            }
                            self.i = end;
                        }
                        other => out.push(other as char),
                    }
                }
                _ => out.push(c as char),
            }
        }
        out
    }

    fn number(&mut self) -> Json {
        let start = self.i;
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E' => self.i += 1,
                _ => break,
            }
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("0");
        Json::Num(s.parse().unwrap_or(0.0))
    }
}

/// Map every sigil-* workspace package to the sorted list of its sigil-* dependencies.
fn sigil_dep_map() -> BTreeMap<String, Vec<String>> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("cargo metadata stdout not utf8");
    let json = Parser::new(&stdout).value();
    let packages = json
        .get("packages")
        .and_then(|p| p.as_array())
        .expect("metadata has no packages array");

    let mut deps_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for pkg in packages {
        let name = match pkg.get("name").and_then(|n| n.as_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !name.starts_with("sigil") {
            continue;
        }
        let mut sigil_deps = Vec::new();
        if let Some(deps) = pkg.get("dependencies").and_then(|d| d.as_array()) {
            for dep in deps {
                // Skip dev- and build-dependencies; only normal deps define the
                // shipping crate graph.
                let kind = dep.get("kind").and_then(|k| k.as_str());
                if matches!(kind, Some("dev") | Some("build")) {
                    continue;
                }
                if let Some(dn) = dep.get("name").and_then(|n| n.as_str()) {
                    if dn.starts_with("sigil-") {
                        sigil_deps.push(dn.to_string());
                    }
                }
            }
        }
        sigil_deps.sort();
        sigil_deps.dedup();
        deps_of.insert(name, sigil_deps);
    }
    deps_of
}

#[test]
fn crate_graph_is_one_way() {
    let deps_of = sigil_dep_map();
    let get = |name: &str| -> Vec<String> {
        deps_of
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("package {} not found in cargo metadata", name))
    };

    // (a) sigil-isa is extraction-ready: no workspace deps at all.
    assert_eq!(
        get("sigil-isa"),
        Vec::<String>::new(),
        "sigil-isa must have no sigil workspace deps (extraction-ready)"
    );

    // sigil-span is a pure leaf.
    assert_eq!(
        get("sigil-span"),
        Vec::<String>::new(),
        "sigil-span must have no deps"
    );

    // (b) sigil-ir depends only on sigil-span.
    assert_eq!(
        get("sigil-ir"),
        vec!["sigil-span".to_string()],
        "sigil-ir must depend only on sigil-span"
    );

    // sigil-backend-z80 wraps the ISA: depends on sigil-ir + sigil-isa (+ span).
    assert_eq!(
        get("sigil-backend-z80"),
        vec!["sigil-ir".to_string(), "sigil-isa".to_string(), "sigil-span".to_string()],
        "sigil-backend-z80 must depend on sigil-ir, sigil-isa, sigil-span only"
    );

    // sigil-link is backend-agnostic in its library deps: sigil-ir + sigil-span.
    // (sigil-backend-z80/sigil-isa are dev-dependencies for the integration test
    // and MUST NOT appear as normal dependencies — cargo metadata --no-deps lists
    // dev-deps too, so filter is by dependency kind below.)
    assert_eq!(
        get("sigil-link"),
        vec!["sigil-ir".to_string(), "sigil-span".to_string()],
        "sigil-link library must depend only on sigil-ir + sigil-span (backends are dev-deps)"
    );

    // (c) only sigil-cli may depend on sigil-frontend-as.
    for (pkg, deps) in &deps_of {
        if deps.iter().any(|d| d == "sigil-frontend-as") {
            assert_eq!(
                pkg, "sigil-cli",
                "only sigil-cli may depend on sigil-frontend-as, but {} does",
                pkg
            );
        }
    }

    // Positive wiring check so (c) is non-vacuous: sigil-cli DOES pull in the frontend.
    assert!(
        get("sigil-cli").contains(&"sigil-frontend-as".to_string()),
        "sigil-cli must depend on sigil-frontend-as"
    );
}
