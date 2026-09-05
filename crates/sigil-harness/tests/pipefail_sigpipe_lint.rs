//! A MATCH READ AS A NON-MATCH: `pipefail` OVER AN EARLY-EXITING READER.
//!
//! `grep -q` exits the moment it MATCHES. If its writer still owes output past
//! that point, the writer's next write(2) lands on a closed pipe: SIGPIPE, exit
//! 141, and `set -o pipefail` hands 141 back as the pipeline's status — so an `if`
//! on that pipeline takes the ELSE branch ON A MATCH. It is silent and asymmetric:
//! it can only ever fire when the answer was YES.
//!
//! WHAT DECIDES WHETHER IT FIRES IS THE WRITER'S SIZE, NOT MACHINE LOAD, and that
//! is worth stating precisely because the intuitive rule is backwards. If the
//! writer has already handed over everything it will ever emit, no signal is
//! delivered and the fault is IMPOSSIBLE. If it must still issue one more write
//! after the reader has gone, the fault is NEAR-CERTAIN. Load only decides the
//! narrow band between. Measured serially, one worker, no concurrency, in
//! `docs/superpowers/notes/2026-09-05-pipefail-sigpipe-classes/boundary.sh`: a
//! bash `printf` writer gives 0/400 at 4,798 bytes and 394/400 at 14,398.
//!
//! And the turnover is a property of the WRITER, not of the pipe. Over one
//! 65,536-byte pipe, the same reader, the same machine: `printf` turns over at
//! ~5-14 KB (BELOW the pipe), `seq` at ~24-240 KB, `cat` at ~480-720 KB. So
//! "under a pipe buffer, therefore safe" is not a sound per-site rule — it clears
//! the shell builtin, which is the writer this repo's scripts actually use, at
//! sizes where it already fails four times in five.
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
//! judges (a) and (c) and DELIBERATELY DOES NOT PRICE THE SIZE. That is the right
//! direction, and it is why the mechanism correction above changed nothing here: a
//! site with (a) and (c) is one input size away from the fault, today's input size
//! is not a property anyone re-checks when a corpus grows, and the fix — asking the
//! question without a pipe, or ending the pipeline in a reader that runs to EOF —
//! costs nothing. A lint that excused small writers would have to be re-run against
//! every corpus that ever grows, which is not a lint.
//!
//! WHERE IT LOOKS. Two surfaces, and they are counted separately because they fail
//! separately:
//!
//!   * a pipeline written at the top level of a line — `cmd | grep -q x`;
//!   * a pipeline written INSIDE a `$( … )` — `v=$(cmd | head -1)`.
//!
//! The second was outside this lint's reach on the day it shipped, and the shape of
//! that miss is the reason it is closed here rather than left booked. Mutating
//! `set -e` into `scripts/nightly_source_gates.sh` made the lint name line 233 and
//! stay silent about line 620, `REGISTER=$(sed … | grep -v … | head -20)`, whose
//! writer streams the whole strict suite log and is squarely in the near-certain
//! band. A fixer who repaired the named line and watched the lint go green would
//! have concluded the file was clear — which is this class's founding failure, one
//! level up: WRONG OUTPUT FROM THE INSTRUMENT, not merely absent output.
//!
//! HOW A STATUS GETS OUT OF A `$( … )`. A substitution's status reaches a decision
//! only through the statement that expands it, and bash is specific about which
//! statements pass it on. The rule is measured against bash itself, in
//! `the_substitution_position_rule_agrees_with_bash` below, because both directions
//! of an error here are silent on a clean corpus:
//!
//!   * `x=$(…)`, `x="pre $(…)"`, `a=1 b=$(…)` — a plain assignment TAKES the
//!     substitution's status, and concatenation does not stop it;
//!   * `local x=$(…)`, `export`/`declare`/`readonly` — the declaration BUILTIN's own
//!     status masks it, so these are safe and are not flagged;
//!   * `echo "$(…)"`, `foo "$(…)"` — argument position; the command's status wins.
//!
//! WHAT IT DOES NOT FLAG, and why each is genuinely safe rather than tolerated:
//!
//!   * `v=$(… | head -1)` under `set -uo pipefail` with no `set -e`, whose value a
//!     later `[[ -n $v ]]` reads. The status is discarded and the VALUE is complete:
//!     `head` writes its lines before it exits, so the signal reaches only its
//!     upstream. This is the shape of every one of the ten such sites in this tree.
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
//!   * A STATUS THAT LEAVES THE FILE. Two routes, both real, neither decidable from
//!     the file alone, and both named here rather than described as a residual:
//!       - a statement that is the LAST one in a function body hands its status to
//!         whoever calls the function, and bash does pass an assignment's status out
//!         that way (`f() { x=$(exit 7); }; f` returns 7 — measured; `local` masks it
//!         and returns 0). Deciding it needs the CONSUMING end, which is what the
//!         2026-09-05 sweep had to enumerate by hand for `reference_env_var`. In this
//!         tree exactly one site takes that route, `nightly_source_gates.sh:233`, and
//!         it is a top-level pipeline rather than a `$( … )`; ZERO of the ten
//!         substitution sites are a function's last statement. It is not implemented
//!         because implementing it today buys one red that must immediately be
//!         excused by re-doing that caller enumeration, and a check whose first act
//!         is to be waved through teaches people to wave it through.
//!       - `set -e` arriving from a SOURCER rather than the file itself. Every
//!         `scripts/lib/*.sh` in this tree sets its own options in the branch that
//!         runs, so that shortfall is currently empty; it would not stay empty for a
//!         lib that relied on its caller's.
//!   * `||` as the consumer (`v=$(… | head) || handler` fires the handler on a
//!     spurious 141). Neither surface treats `||` as consuming, which was true
//!     before this change too; measured against the corpus it would add zero
//!     findings today, so it is recorded rather than silently adopted.
//!   * two assignment shapes bash propagates through and the position rule rejects,
//!     both enumerated as absent from this tree — see
//!     `substitution_status_propagates`.

use std::path::{Path, PathBuf};

// ── the corpus ──────────────────────────────────────────────────────────────────

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the harness crate sits two levels under the repo root")
        .to_path_buf()
}

/// THE POPULATION IS WHAT THE REPO CARRIES, asked of git rather than of the disk.
///
/// A directory walk was the obvious thing and it was wrong twice over. It descended
/// into `.cargo-target`, so the lint's answer depended on how much had been built —
/// and, worse, other harness tests plant SHELL SCRIPTS in bed directories under that
/// same target dir. A lint that reads generated beds can go red because of what
/// another test was doing at the time, which is a flaky gate, and a flaky gate is
/// how a real finding gets waved through as "that one again". Nothing on disk but
/// outside the index is shell this repo runs.
fn tracked_files() -> Vec<PathBuf> {
    let root = repo_root();
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "-z"])
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "COULD NOT MEASURE: `git ls-files` could not run in {}: {e}. This lint's \
                 population is what git tracks; without it, it has no population at all \
                 and its silence would say nothing.",
                root.display()
            )
        });
    assert!(
        out.status.success(),
        "COULD NOT MEASURE: `git ls-files` failed in {} ({}): {}",
        root.display(),
        out.status,
        String::from_utf8_lossy(&out.stderr).trim()
    );
    let mut files: Vec<PathBuf> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| root.join(String::from_utf8_lossy(s).to_string()))
        .collect();
    files.sort();
    files
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
    let files = tracked_files();

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

// ── command substitutions ───────────────────────────────────────────────────────

/// One `$( … )` or `` ` … ` `` found in a line.
///
/// `tok` is the whole substitution including its sigil and closer; `content` is the
/// shell inside it. `depth` is 0 for a substitution written directly in the line and
/// higher for one nested inside another — only depth 0 is analysed, because a nested
/// substitution's status is swallowed by the command that expands it.
struct Subst {
    tok_start: usize,
    tok_end: usize,
    content_start: usize,
    content_end: usize,
    depth: usize,
}

/// Every command substitution in `s`, at every depth.
///
/// `$(` opens one INSIDE double quotes too — `x="$(cmd | head -1)"` is the shape this
/// repo actually writes — so the quote state cannot simply skip quoted text the way
/// `top_level_positions` does. `$(( … ))` is arithmetic and is stepped over.
/// Backticks are scanned as well: this tree has none today (all 37 backtick lines are
/// prose inside quoted strings or a `sed` pattern, enumerated), and scanning them
/// costs nothing, so the lint does not rest on that remaining true.
fn command_substitutions(s: &str) -> Vec<Subst> {
    let mut out = Vec::new();
    scan_substitutions(s.as_bytes(), 0, s.len(), 0, &mut out);
    out.sort_by_key(|c| (c.tok_start, c.depth));
    out
}

/// Scan `b[i..limit]` for substitutions, returning the offset of the `)` that closes
/// the level being scanned (or `limit` when nothing closes it).
fn scan_substitutions(b: &[u8], mut i: usize, limit: usize, depth: usize, out: &mut Vec<Subst>) -> usize {
    let (mut sq, mut dq) = (false, false);
    let mut paren = 0i32;
    while i < limit {
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
        if sq {
            i += 1;
            continue;
        }
        if c == b'"' {
            dq = !dq;
            i += 1;
            continue;
        }
        if c == b'$' && i + 1 < limit && b[i + 1] == b'(' {
            if i + 2 < limit && b[i + 2] == b'(' {
                let mut j = i + 3;
                while j + 1 < limit && !(b[j] == b')' && b[j + 1] == b')') {
                    j += 1;
                }
                i = (j + 2).min(limit);
                continue;
            }
            let cs = i + 2;
            let ce = scan_substitutions(b, cs, limit, depth + 1, out);
            out.push(Subst {
                tok_start: i,
                tok_end: (ce + 1).min(limit),
                content_start: cs,
                content_end: ce,
                depth,
            });
            i = (ce + 1).min(limit);
            continue;
        }
        if c == b'`' {
            let cs = i + 1;
            let mut j = cs;
            while j < limit && b[j] != b'`' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let ce = j.min(limit);
            scan_substitutions(b, cs, ce, depth + 1, out);
            out.push(Subst {
                tok_start: i,
                tok_end: (ce + 1).min(limit),
                content_start: cs,
                content_end: ce,
                depth,
            });
            i = (ce + 1).min(limit);
            continue;
        }
        if !dq {
            if c == b'(' {
                paren += 1;
            } else if c == b')' {
                if paren == 0 && depth > 0 {
                    return i;
                }
                paren -= 1;
            }
        }
        i += 1;
    }
    limit
}

/// The pipeline inside a substitution whose status BECOMES the substitution's, if it
/// ends in an early-exiting reader.
///
/// A substitution exits with the status of the LAST command it runs, so only the last
/// `;`-separated statement can decide it — and within that statement, any piece an
/// `&&`/`||` chain can end on. Everything before a `;` is unreachable as a status.
fn substitution_early_exit_pipeline(content: &str) -> Option<String> {
    let mut from = 0usize;
    let mut last = content;
    for at in top_level_positions(content, b';') {
        if content[from..at].trim().is_empty() {
            from = at + 1;
            continue;
        }
        last = &content[from..at];
        from = at + 1;
    }
    if !content[from..].trim().is_empty() {
        last = &content[from..];
    }
    for (_, seg, _) in split_on_logic(last) {
        if ends_in_early_exit_reader(&seg).is_some() {
            return Some(seg.trim().to_string());
        }
    }
    None
}

/// Does this substitution's status reach the statement that contains it?
///
/// MEASURED, not recalled (`bash -c 'set -e; …; echo after'`, exit status shown):
///
/// ```text
///   x=$(exit 7)                  -> 7    an assignment takes the substitution's status
///   x="pre $(exit 7)"            -> 7    concatenation does not stop it
///   a=1 b=$(exit 7)              -> 7
///   f() { local x=$(exit 7); }   -> 0    `local` is a BUILTIN; its own status masks it
///   export/declare/readonly …    -> 0    same masking
///   echo "$(exit 7)"             -> 0    argument position: the command's status wins
///   true "$(exit 7)"             -> 0
/// ```
///
/// So the rule is not "the substitution is the whole right-hand side" — it is "the
/// statement is a plain assignment and the substitution is somewhere in its value".
/// A declaration builtin in front of it (`local`, `declare`, `typeset`, `export`,
/// `readonly`) is rejected because the parse below demands `NAME=` at the head of the
/// statement, which those never satisfy.
///
/// KNOWN FALSE NEGATIVES, stated rather than left to be discovered: `b=$(…) a=1`
/// (assignment after the substitution) and `arr=( $(…) )` both propagate in bash and
/// are rejected here, because anything separated from the substitution by a space
/// could equally be the command word of `VAR=$(…) prog args`, which does NOT
/// propagate. Neither shape appears in this tree with an early-exiting reader.
fn substitution_status_propagates(segment: &str, sub: &Subst) -> bool {
    // The `;`-statement of the segment that contains the substitution.
    let mut st = 0usize;
    let mut en = segment.len();
    for at in top_level_positions(segment, b';') {
        if at < sub.tok_start {
            st = at + 1;
        } else {
            en = at;
            break;
        }
    }
    if sub.tok_end > en {
        return false;
    }
    let stmt = &segment[st..en];
    let tok_start = sub.tok_start - st;
    let tok_end = sub.tok_end - st;

    // Leading keywords are not part of the statement's own shape.
    let mut off = 0usize;
    loop {
        let rest = &stmt[off..];
        let trimmed = rest.trim_start();
        let skipped = rest.len() - trimmed.len();
        let word = trimmed.split_whitespace().next().unwrap_or("");
        if matches!(word, "if" | "elif" | "while" | "until" | "!" | "then" | "do" | "else" | "{") {
            off += skipped + word.len();
        } else {
            off += skipped;
            break;
        }
    }
    if off > tok_start {
        return false;
    }
    let body = &stmt[off..];

    // `NAME=`, `NAME[idx]=`, `NAME+=` — and nothing else at the head.
    let bb = body.as_bytes();
    let mut k = 0usize;
    while k < bb.len() && (bb[k].is_ascii_alphanumeric() || bb[k] == b'_') {
        k += 1;
    }
    if k == 0 || bb[0].is_ascii_digit() {
        return false;
    }
    if k < bb.len() && bb[k] == b'[' {
        match body[k..].find(']') {
            Some(rel) => k += rel + 1,
            None => return false,
        }
    }
    if k < bb.len() && bb[k] == b'+' {
        k += 1;
    }
    if k >= bb.len() || bb[k] != b'=' {
        return false;
    }
    let rhs_at = off + k + 1;
    if tok_start < rhs_at {
        return false;
    }

    // Nothing after the substitution that could be a command word of its own.
    stmt[tok_end..]
        .chars()
        .all(|c| c.is_whitespace() || matches!(c, '"' | '\'' | ')' | '}'))
}

/// Split a logical line into the pieces `&&` and `||` join, remembering the byte
/// offset each piece starts at and the operator that FOLLOWS it — that operator is
/// what consumes its status.
fn split_on_logic(line: &str) -> Vec<(usize, String, Option<&'static str>)> {
    let b = line.as_bytes();
    let mut cuts: Vec<(usize, &'static str)> = Vec::new();
    for op in *b"&|" {
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
        out.push((from, line[from..at].to_string(), Some(op)));
        from = at + 2;
    }
    out.push((from, line[from..].to_string(), None));
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

/// Two censuses and the offences.
///
/// `census` is every pipeline in the corpus whose last stage can exit early, counted
/// at the top level of a line; `sub_census` is the same thing inside a `$( … )`. They
/// are kept apart because each one is the vacuity witness for a DIFFERENT half of the
/// reader, and a single total would let one half break while the other kept the number
/// above zero — which is precisely how this lint shipped blind to command
/// substitutions in the first place.
struct Scan {
    census: usize,
    sub_census: usize,
    findings: Vec<Finding>,
    units: usize,
}

fn census_and_findings() -> Scan {
    let units = shell_units();
    let mut census = 0usize;
    let mut sub_census = 0usize;
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
            for (idx, (_at, seg, follower)) in segments.iter().enumerate() {
                let head = line.trim_start();
                let first = idx == 0;
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

                // A pipeline written at the top level of the segment.
                if ends_in_early_exit_reader(seg).is_some() {
                    census += 1;
                    if pipefail {
                        let why = if condition {
                            Some(
                                "the pipeline's status IS the condition, so a 141 from SIGPIPE \
                                 takes the other branch",
                            )
                        } else if gates_next {
                            Some(
                                "`&&` consumes the pipeline's status, so a 141 from SIGPIPE skips \
                                 what follows",
                            )
                        } else if bare_under_errexit {
                            Some(
                                "`set -e` consumes every command's status, so a 141 from SIGPIPE \
                                 aborts the script",
                            )
                        } else {
                            None
                        };
                        if let Some(why) = why {
                            findings.push(Finding {
                                label: u.label.clone(),
                                line: n + u.line_base,
                                text: line.trim().to_string(),
                                why,
                            });
                        }
                    }
                }

                // A pipeline written inside a `$( … )`. Its status reaches a decision
                // only through a plain assignment; see `substitution_status_propagates`.
                for sub in command_substitutions(seg) {
                    if sub.depth != 0 {
                        continue;
                    }
                    let content = &seg[sub.content_start..sub.content_end];
                    if substitution_early_exit_pipeline(content).is_none() {
                        continue;
                    }
                    sub_census += 1;
                    if !pipefail || !substitution_status_propagates(seg, &sub) {
                        continue;
                    }
                    let why = if condition {
                        "the assignment's status IS the condition, and an assignment takes the \
                         status of the command substitution in its value, so a 141 from SIGPIPE \
                         INSIDE `$( … )` takes the other branch"
                    } else if gates_next {
                        "`&&` consumes the assignment's status, and an assignment takes the \
                         status of the command substitution in its value, so a 141 from SIGPIPE \
                         INSIDE `$( … )` skips what follows"
                    } else if bare_under_errexit {
                        "`set -e` consumes every command's status, and an assignment takes the \
                         status of the command substitution in its value, so a 141 from SIGPIPE \
                         INSIDE `$( … )` aborts the script"
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
    }
    Scan { census, sub_census, findings, units: units.len() }
}

// ── the gate ────────────────────────────────────────────────────────────────────

#[test]
fn no_pipefail_decision_rests_on_an_early_exiting_reader() {
    let Scan { census, sub_census, findings, units } = census_and_findings();

    // LOUD ON UNMEASURABLE. A reader that parses nothing finds nothing and exits
    // green, which is indistinguishable from a clean corpus by result alone — and
    // that is the exact shape of failure this whole class is about.
    assert!(
        units > 20,
        "COULD NOT MEASURE: only {units} shell unit(s) found, the walk did not reach \
         this repo's scripts, so its silence says nothing"
    );
    assert!(
        census > 0,
        "COULD NOT MEASURE: {units} shell unit(s) scanned and NOT ONE pipeline ending in \
         an early-exiting reader was seen. This corpus is full of `… | head -N`; a zero \
         here means the pipeline reader broke, not that the tree is clean"
    );
    // The command-substitution half needs its OWN vacuity witness. The two readers
    // fail independently, and this lint spent its first day green while seeing none
    // of this half — a state a combined count could never have distinguished from a
    // clean tree.
    assert!(
        sub_census > 0,
        "COULD NOT MEASURE: {units} shell unit(s) scanned and NOT ONE pipeline INSIDE a \
         `$( … )` ending in an early-exiting reader was seen. This corpus is full of \
         `v=$(… | head -1)`; a zero here means the substitution scanner broke, not that \
         the tree is clean"
    );

    if !findings.is_empty() {
        let n_bad = findings.len();
        let mut msg = format!(
            "{n_bad} pipeline(s) let SIGPIPE decide, out of {census} at the top level of a \
             line and {sub_census} inside a `$( … )` that end in an early-exiting reader, \
             across {units} shell unit(s).\n\n\
             The last stage exits before its input is exhausted, the writer is killed by \
             SIGPIPE and exits 141, and `pipefail` hands 141 back as the pipeline's \
             status, so a MATCH reads as a NON-MATCH.\n\n\
             It fires when the writer still owes output past the earliest match, and \
             that, NOT machine load, is what decides it. Measured serially with no \
             concurrency at all, a bash `printf` writer gives 0/400 at 4.8 KB and \
             394/400 at 14.4 KB. Do not excuse a site because it runs once, alone, in \
             a nightly lane: a writer enumerating a large corpus there is in the \
             NEAR-CERTAIN regime, not the rare one. The turnover is a property of the \
             writer, not of the pipe, and for a shell builtin it sits well BELOW the \
             pipe's capacity.\n\n\
             Ask the question without a pipe (a shell loop, `[[ $s == *needle* ]]`), or \
             end the pipeline in a reader that runs to EOF (`sort`, `awk`, `wc`). For a \
             `v=$(… | head -N)` whose status a decision reads, `v=$(… | awk 'NR<=N')` \
             keeps the value and drops the early exit; `v=$(…) || true` only silences \
             the symptom and leaves the next reader of `$?` wrong.\n\n",
        );
        for f in &findings {
            msg.push_str(&format!("  {}:{}: {}\n      {}\n", f.label, f.line, f.text, f.why));
        }
        panic!("{msg}");
    }
}

// ── the model checked against the thing it models ───────────────────────────────

fn bash_or_skip() -> Option<&'static str> {
    let ok = std::process::Command::new("bash")
        .args(["-c", "exit 0"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    ok.then_some("bash")
}

/// Does bash let a `$( … )` failing with 7 abort a `set -e` shell from this statement?
fn bash_propagates(stmt: &str) -> bool {
    let script = format!("set -e\nf() {{ {stmt}; echo REACHED; }}\nf\n");
    let out = std::process::Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .unwrap_or_else(|e| panic!("COULD NOT MEASURE: bash would not run: {e}"));
    !String::from_utf8_lossy(&out.stdout).contains("REACHED")
}

/// What the lint believes about the same statement.
fn lint_propagates(stmt: &str) -> bool {
    command_substitutions(stmt)
        .into_iter()
        .filter(|s| s.depth == 0)
        .any(|s| substitution_status_propagates(stmt, &s))
}

/// WHICH SUBSTITUTION POSITIONS CARRY A STATUS IS A CLAIM ABOUT BASH, so it is put to
/// bash rather than to anyone's memory of bash.
///
/// This matters more than a usual unit test. The lint's whole new half turns on this
/// one predicate, and both directions of an error are silent: believing `local x=$(…)`
/// propagates makes the lint cry wolf until somebody weakens it, and believing
/// `x="pre $(…)"` does not propagate makes it green over a live defect. Neither shows
/// up in a corpus run, because the corpus is clean.
#[test]
fn the_substitution_position_rule_agrees_with_bash() {
    if bash_or_skip().is_none() {
        panic!(
            "COULD NOT MEASURE: no working `bash`, so the lint's model of command-\
             substitution status propagation was not checked against anything"
        );
    }
    let cases = [
        "x=$(exit 7)",
        "x=\"pre $(exit 7)\"",
        "a=1 b=$(exit 7)",
        "local x=$(exit 7)",
        "export x=$(exit 7)",
        "declare x=$(exit 7)",
        "readonly x=$(exit 7)",
        "echo \"$(exit 7)\"",
        "true \"$(exit 7)\"",
        "printf '%s' \"$(exit 7)\"",
        "x=$(exit 7) true",
        "if x=$(exit 7); then echo T; fi",
    ];
    let mut wrong = Vec::new();
    for stmt in cases {
        let (bash, lint) = (bash_propagates(stmt), lint_propagates(stmt));
        // `if x=$(…)` is a CONSUMPTION shape, not a position one: bash's `if` swallows
        // the status so `set -e` never sees it, while the lint must still call the
        // position propagating so the condition rule can fire. It is listed to keep
        // that difference visible, and it is the one case where the two must differ.
        let expect_agreement = stmt != "if x=$(exit 7); then echo T; fi";
        if expect_agreement && bash != lint {
            wrong.push(format!("  {stmt:38}  bash={bash}  lint={lint}"));
        }
        if !expect_agreement && !(lint && !bash) {
            wrong.push(format!("  {stmt:38}  expected lint=true bash=false, got lint={lint} bash={bash}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "the lint's model of `$( … )` status propagation disagrees with bash on \
         {} of {} statements:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

/// THE CLASS ITSELF, THROUGH A SUBSTITUTION — the thing the corpus half of this file
/// cannot show, because the corpus is (now) clean.
///
/// `yes` never finishes, so it is always still writing when `head -1` goes: this is
/// the far end of the near-certain band rather than a sample of the racing one, which
/// is why five runs is a witness and not a coin flip.
#[test]
fn a_substitution_assignment_really_does_carry_141_under_errexit() {
    if bash_or_skip().is_none() {
        panic!("COULD NOT MEASURE: no working `bash`, so the mechanism was not demonstrated");
    }
    let mut aborted = 0;
    let mut codes = Vec::new();
    for _ in 0..5 {
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg("set -euo pipefail\nv=$(yes hello | head -1)\necho \"REACHED v=$v\"\n")
            .output()
            .unwrap_or_else(|e| panic!("COULD NOT MEASURE: bash would not run: {e}"));
        let code = out.status.code().unwrap_or(-1);
        codes.push(code);
        if !String::from_utf8_lossy(&out.stdout).contains("REACHED") {
            aborted += 1;
        }
    }
    assert!(
        aborted > 0,
        "COULD NOT MEASURE: `v=$(yes hello | head -1)` under `set -euo pipefail` reached \
         the next statement all 5 times (exit codes {codes:?}). Either SIGPIPE is being \
         suppressed on this box or the shell is not bash, in both cases this file's \
         premise went unverified, which is not the same as safe"
    );
    assert!(
        codes.contains(&141),
        "the abort did not carry 141: exit codes {codes:?}. The class is defined by \
         SIGPIPE's 141 reaching a consumer; another code means something else failed \
         and this witness proves nothing"
    );
}
