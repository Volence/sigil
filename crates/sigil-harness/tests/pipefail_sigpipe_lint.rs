//! A MATCH READ AS A NON-MATCH: `pipefail` OVER AN EARLY-EXITING READER.
//!
//! `grep -q` exits the moment it MATCHES. Its writer is then killed by SIGPIPE
//! and exits 141, and `set -o pipefail` hands 141 back as the pipeline's status —
//! so an `if` on that pipeline takes the ELSE branch ON A MATCH. Whether the
//! writer's write(2) lands before the reader exits is a scheduling race, which
//! makes the fault load-dependent, silent, and asymmetric: it can only ever fire
//! when the answer was YES.
//!
//! It cost this lane a dark nightly lane and a red landing run on a tree that had
//! not touched the script. It was then fixed AT ONE SITE, which is what this file
//! exists to stop being the whole answer: the construct is not rare, the reader is
//! not always `grep -q`, and the next one arrives the same way — as somebody
//! else's test going red.
//!
//! THE THREE CONDITIONS, all of which must hold for a site to be a defect:
//!
//!   a. an EARLY-EXITING READER as the pipeline's last stage — `grep -q/-l/-m`,
//!      `head`, `sed` with a `q` command, `awk` with `exit`, `find -quit`;
//!   b. a WRITER that can still be writing when it goes;
//!   c. `pipefail` in effect AND the pipeline's STATUS consumed as a decision.
//!
//! (b) is a property of the data and cannot be read off the source, so this lint
//! judges (a) and (c). That is the right direction: a site with (a) and (c) is one
//! input size away from the fault, and the fix — asking the question without a
//! pipe, or ending the pipeline in a reader that runs to EOF — costs nothing.
//!
//! WHAT IT DOES NOT FLAG, and why each is genuinely safe rather than tolerated:
//!
//!   * `v=$(… | head -1)` under `set -uo pipefail` with no `set -e`. The status
//!     is discarded and the decision reads the VALUE, which is complete: `head`
//!     writes its lines before it exits, so the signal reaches only its upstream.
//!   * a pipeline ending in `sort`, `tee`, `tr`, `wc`, `uniq` — a reader that
//!     cannot produce output before EOF cannot exit before EOF, so there is no
//!     signal to take. This is what makes a site safe BY CONSTRUCTION rather than
//!     by argument.
//!   * an early-exiting command with no pipe under it — `grep -m1 FILE`,
//!     `find … -quit` — where there is no writer to signal.
//!
//! WHAT IT CANNOT SEE, stated because an assertion of completeness and the check
//! that would establish it are separable, and only the assertion is cheap:
//!
//!   * a pipeline inside a heredoc body. Those are skipped wholesale, because the
//!     evidence files for this very class quote the defective construct inside
//!     one, and a lint whose first casualty is its own proof is not usable.
//!   * a pipeline assembled at runtime from a variable.
//!   * `set -e` arriving from a SOURCER rather than the file itself. Every
//!     `scripts/lib/*.sh` in this tree sets its own options in the branch that
//!     runs, so the shortfall is currently empty; it would not stay empty for a
//!     lib that relied on its caller's.

use std::path::{Path, PathBuf};

// ── the corpus ──────────────────────────────────────────────────────────────────

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the harness crate sits two levels under the repo root")
        .to_path_buf()
}

/// Directories that hold no shell this repo runs: git's own storage, agent
/// worktrees (other branches, not this tree), and build output.
const SKIP_DIRS: &[&str] = &[".git", ".claude", ".worktrees", "target", "node_modules"];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let base = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if SKIP_DIRS.contains(&base.as_str()) {
            continue;
        }
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// One body of shell, with the label a finding is reported under.
struct Unit {
    label: String,
    text: String,
    /// Physical line of the file that this unit's line 1 sits on, minus one — so a
    /// finding inside a workflow `run:` block names a line the reader can open.
    line_base: usize,
}

/// Every body of shell this repo runs: `.sh` files whole, and each `run:` block of
/// a GitHub workflow — which is shell too, executes under its own `set` line, and
/// is where one of this class's two live instances was found.
fn shell_units() -> Vec<Unit> {
    let root = repo_root();
    let mut files = Vec::new();
    walk(&root, &mut files);

    let mut units = Vec::new();
    for f in &files {
        let rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        match ext {
            "sh" | "bash" => units.push(Unit { label: rel, text, line_base: 0 }),
            "yml" | "yaml" => units.extend(run_blocks(&rel, &text)),
            _ => {}
        }
    }
    units
}

/// The `run: |` blocks of a workflow, dedented to their own indentation so they
/// read as the shell they are.
fn run_blocks(rel: &str, text: &str) -> Vec<Unit> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim_end();
        if !(t.ends_with("run: |") || t.ends_with("run: |-")) {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut body: Vec<String> = Vec::new();
        let mut indent: Option<usize> = None;
        let mut j = start;
        while j < lines.len() {
            let l = lines[j];
            if l.trim().is_empty() {
                body.push(String::new());
                j += 1;
                continue;
            }
            let ind = l.len() - l.trim_start().len();
            let base = *indent.get_or_insert(ind);
            if ind < base {
                break;
            }
            body.push(l[base..].to_string());
            j += 1;
        }
        out.push(Unit {
            label: rel.to_string(),
            text: body.join("\n"),
            line_base: start,
        });
        i = j;
    }
    out
}

// ── reading the shell ───────────────────────────────────────────────────────────

/// Readers that can exit before their input is exhausted, so their writer can be
/// killed mid-write.
fn ends_in_early_exit_reader(segment: &str) -> Option<String> {
    let last = last_stage(segment)?;
    let mut words = last.split_whitespace();
    let cmd = words.next()?;
    let cmd = cmd.rsplit('/').next().unwrap_or(cmd);
    let rest: Vec<&str> = words.collect();
    let joined = rest.join(" ");
    let hit = match cmd {
        "head" => true,
        "grep" | "egrep" | "fgrep" | "rg" => rest.iter().any(|w| {
            w.starts_with('-')
                && !w.starts_with("--")
                && (w.contains('q') || w.contains('l') || w.contains('m'))
                || *w == "--quiet"
                || *w == "--silent"
                || *w == "--files-with-matches"
                || w.starts_with("--max-count")
        }),
        // `sed` quits early only with an explicit `q`; `sed -n '2,80p'` reads on to EOF.
        "sed" => joined.contains('q'),
        "awk" | "gawk" | "mawk" => joined.contains("exit"),
        "find" => joined.contains("-quit"),
        _ => false,
    };
    hit.then(|| last.to_string())
}

/// The text after the last top-level `|` — the stage that decides whether anything
/// upstream can be signalled at all.
fn last_stage(segment: &str) -> Option<&str> {
    let pipes = top_level_positions(segment, b'|');
    let at = *pipes.last()?;
    Some(segment[at + 1..].trim())
}

/// Byte offsets of `needle` in `s` that are outside quotes, brackets, command
/// substitutions and arithmetic — and, for `|`, are a single `|` rather than `||`.
fn top_level_positions(s: &str, needle: u8) -> Vec<usize> {
    let b = s.as_bytes();
    let (mut sq, mut dq) = (false, false);
    let (mut paren, mut brack, mut brace) = (0i32, 0i32, 0i32);
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' && !sq {
            i += 2;
            continue;
        }
        if !dq && c == b'\'' {
            sq = !sq;
            i += 1;
            continue;
        }
        if !sq && c == b'"' {
            dq = !dq;
            i += 1;
            continue;
        }
        if sq || dq {
            i += 1;
            continue;
        }
        match c {
            b'(' => paren += 1,
            b')' => paren -= 1,
            b'[' => brack += 1,
            b']' => brack -= 1,
            b'{' => brace += 1,
            b'}' => brace -= 1,
            _ => {}
        }
        if paren == 0 && brack == 0 && brace == 0 && c == needle {
            // `||` and `&&` are operators, not the single character.
            let doubled = (i + 1 < b.len() && b[i + 1] == needle)
                || (i > 0 && b[i - 1] == needle);
            if !doubled {
                out.push(i);
            }
        }
        i += 1;
    }
    out
}

/// Split a logical line into the pieces `&&` and `||` join, remembering the
/// operator that FOLLOWS each piece — that operator is what consumes its status.
fn split_on_logic(line: &str) -> Vec<(String, Option<&'static str>)> {
    let b = line.as_bytes();
    let mut cuts: Vec<(usize, &'static str)> = Vec::new();
    for op in [b'&', b'|'] {
        let (mut sq, mut dq) = (false, false);
        let (mut paren, mut brack) = (0i32, 0i32);
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            if c == b'\\' && !sq {
                i += 2;
                continue;
            }
            if !dq && c == b'\'' {
                sq = !sq;
            } else if !sq && c == b'"' {
                dq = !dq;
            } else if !sq && !dq {
                match c {
                    b'(' => paren += 1,
                    b')' => paren -= 1,
                    b'[' => brack += 1,
                    b']' => brack -= 1,
                    _ => {}
                }
                if paren == 0 && brack == 0 && c == op && i + 1 < b.len() && b[i + 1] == op {
                    cuts.push((i, if op == b'&' { "&&" } else { "||" }));
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }
    }
    cuts.sort();
    let mut out = Vec::new();
    let mut from = 0usize;
    for (at, op) in cuts {
        out.push((line[from..at].to_string(), Some(op)));
        from = at + 2;
    }
    out.push((line[from..].to_string(), None));
    out
}

/// Physical lines folded into logical ones: backslash continuations, and a line
/// whose last non-comment character is a `|` (the pipeline continues).
/// Heredoc bodies are dropped whole. Returns `(line number, text)`.
fn logical_lines(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();

        // A heredoc introducer takes everything up to its terminator out of scope.
        if let Some(tag) = heredoc_tag(raw) {
            i += 1;
            while i < lines.len() && lines[i].trim() != tag {
                i += 1;
            }
            i += 1;
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        let start = i + 1;
        let mut acc = raw.trim().to_string();
        loop {
            let cont_backslash = acc.ends_with('\\');
            if cont_backslash {
                acc.pop();
            }
            let cont_pipe = !cont_backslash && {
                let t = acc.trim_end();
                t.ends_with('|') && !t.ends_with("||")
            };
            if !(cont_backslash || cont_pipe) || i + 1 >= lines.len() {
                break;
            }
            i += 1;
            let mut nxt = lines[i].trim();
            // A comment line inside a continued pipeline continues it.
            while nxt.starts_with('#') && i + 1 < lines.len() {
                i += 1;
                nxt = lines[i].trim();
            }
            acc.push(' ');
            acc.push_str(nxt);
        }
        out.push((start, acc));
        i += 1;
    }
    out
}

/// The terminator of a heredoc opened on this line, if it opens one.
fn heredoc_tag(line: &str) -> Option<String> {
    let at = line.find("<<")?;
    let rest = &line[at + 2..];
    let rest = rest.strip_prefix('-').unwrap_or(rest);
    if rest.starts_with('<') {
        return None; // `<<<` is a here-string, not a heredoc
    }
    let rest = rest.trim_start();
    let tag: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '\'' || *c == '"')
        .collect();
    let tag = tag.trim_matches(|c| c == '\'' || c == '"').to_string();
    (!tag.is_empty()).then_some(tag)
}

fn sets_option(text: &str, want: impl Fn(&str) -> bool) -> bool {
    text.lines().any(|l| {
        let t = l.trim();
        !t.starts_with('#') && t.starts_with("set ") && want(t)
    })
}

// ── the finding ─────────────────────────────────────────────────────────────────

struct Finding {
    label: String,
    line: usize,
    text: String,
    why: &'static str,
}

/// Every pipeline in the corpus whose last stage can exit early — the CENSUS, not
/// the offence. Its size is what says the reader below is actually reading shell.
fn census_and_findings() -> (usize, Vec<Finding>, usize) {
    let units = shell_units();
    let mut census = 0usize;
    let mut findings = Vec::new();
    for u in &units {
        let pipefail = sets_option(&u.text, |t| t.contains("pipefail"));
        let errexit = sets_option(&u.text, |t| {
            t.contains("errexit")
                || t.split_whitespace()
                    .any(|w| w.starts_with('-') && !w.starts_with("--") && w.contains('e'))
        });
        for (n, line) in logical_lines(&u.text) {
            // A `case` arm's pattern carries `|` as alternation, not as a pipe.
            if line.contains(";;") {
                continue;
            }
            let segments = split_on_logic(&line);
            for (idx, (seg, follower)) in segments.iter().enumerate() {
                if ends_in_early_exit_reader(seg).is_none() {
                    continue;
                }
                census += 1;
                if !pipefail {
                    continue;
                }
                let first = idx == 0;
                let head = line.trim_start();
                let condition = first
                    && (head.starts_with("if ")
                        || head.starts_with("elif ")
                        || head.starts_with("while ")
                        || head.starts_with("until ")
                        || head.starts_with("! "));
                let gates_next = *follower == Some("&&");
                let bare_under_errexit = errexit
                    && segments.len() == 1
                    && !head.starts_with("if ")
                    && !head.starts_with("elif ")
                    && !head.starts_with("while ")
                    && !head.starts_with("until ");
                let why = if condition {
                    "the pipeline's status IS the condition, so a 141 from SIGPIPE takes \
                     the other branch"
                } else if gates_next {
                    "`&&` consumes the pipeline's status, so a 141 from SIGPIPE skips what \
                     follows"
                } else if bare_under_errexit {
                    "`set -e` consumes every command's status, so a 141 from SIGPIPE aborts \
                     the script"
                } else {
                    continue;
                };
                findings.push(Finding {
                    label: u.label.clone(),
                    line: n + u.line_base,
                    text: line.trim().to_string(),
                    why,
                });
            }
        }
    }
    (census, findings, units.len())
}

// ── the gate ────────────────────────────────────────────────────────────────────

#[test]
fn no_pipefail_decision_rests_on_an_early_exiting_reader() {
    let (census, findings, units) = census_and_findings();

    // LOUD ON UNMEASURABLE. A reader that parses nothing finds nothing and exits
    // green, which is indistinguishable from a clean corpus by result alone — and
    // that is the exact shape of failure this whole class is about.
    assert!(
        units > 20,
        "COULD NOT MEASURE: only {units} shell unit(s) found — the walk did not reach \
         this repo's scripts, so its silence says nothing"
    );
    assert!(
        census > 0,
        "COULD NOT MEASURE: {units} shell unit(s) scanned and NOT ONE pipeline ending in \
         an early-exiting reader was seen. This corpus is full of `… | head -N`; a zero \
         here means the pipeline reader broke, not that the tree is clean"
    );

    if !findings.is_empty() {
        let n_bad = findings.len();
        let mut msg = format!(
            "{n_bad} pipeline(s) let SIGPIPE decide, out of {census} that end in an \
             early-exiting reader across {units} shell unit(s).\n\n\
             The last stage exits before its input is exhausted, the writer is killed by \
             SIGPIPE and exits 141, and `pipefail` hands 141 back as the pipeline's \
             status — so a MATCH reads as a NON-MATCH. It fires only when the writer is \
             still writing, which makes it load-dependent and invisible to a serial run \
             of the same check.\n\n\
             Ask the question without a pipe (a shell loop, `[[ $s == *needle* ]]`), or \
             end the pipeline in a reader that runs to EOF (`sort`, `awk`, `wc`).\n\n",
        );
        for f in &findings {
            msg.push_str(&format!("  {}:{}: {}\n      {}\n", f.label, f.line, f.text, f.why));
        }
        panic!("{msg}");
    }
}
