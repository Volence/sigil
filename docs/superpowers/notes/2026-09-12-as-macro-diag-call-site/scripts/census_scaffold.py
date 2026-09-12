#!/usr/bin/env python3
"""SCAFFOLD, never committed: make SourceMap::label append every label that
carries a trail to the file named by SIGIL_TRAIL_CENSUS, with the thread name
(libtest names a test's thread after the test) and argv[1] (a CLI child's input
path). Refuses unless its anchor occurs exactly once."""
import sys
PATH = "/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312/crates/sigil-span/src/lib.rs"
src = open(PATH).read()
old = """        out.push_str(&format!(":{col}"));
        Some(out)
    }"""
new = """        out.push_str(&format!(":{col}"));
        if !trail.is_empty() {
            if let Some(p) = std::env::var_os("SIGIL_TRAIL_CENSUS") {
                use std::io::Write as _;
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                    let _ = writeln!(
                        f,
                        "thread={:?} arg1={:?} label={out}",
                        std::thread::current().name(),
                        std::env::args().nth(1)
                    );
                }
            }
        }
        Some(out)
    }"""
if src.count(old) != 1:
    print(f"REFUSED: anchor occurs {src.count(old)} times"); sys.exit(1)
open(PATH, "w").write(src.replace(old, new))
assert new in open(PATH).read()
print("SCAFFOLD APPLIED to", PATH)
