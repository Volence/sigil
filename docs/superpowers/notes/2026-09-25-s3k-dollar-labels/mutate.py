#!/usr/bin/env python3
"""mutate.py <id>: apply one mutation of the IMPLEMENTATION to the worktree and
print the mutated line(s). Each is an exact, single-occurrence replacement."""
import pathlib, sys
W = pathlib.Path("/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a620ff60a900c195e/crates/sigil-frontend-as/src")
M = {
    # `$$` names global: the scope is dropped from the key.
    "M1": ("eval.rs", 'format!("{name}@{}", self.temp_scope)', 'format!("{name}@{}", "")'),
    # The directives that write asl's own symbols (and endstruct) do not move the scope.
    "M2": ("eval.rs", "    fn open_temp_scope(&mut self, symbol: &str) {\n        self.temp_scope = symbol.to_string();",
                      "    fn open_temp_scope(&mut self, symbol: &str) {\n        let _ = symbol;"),
    # A plain PC label does not move the scope.
    "M3": ("eval.rs", "            // caller `$$z:`, are `#1000` (probe `g01`).\n            self.temp_scope = name.to_string();",
                      "            // caller `$$z:`, are `#1000` (probe `g01`).\n            let _ = &self.temp_scope;"),
    # A body's PC `$$x:` is filed globally instead of in the instance.
    "M4": ("eval.rs", "            match self.expansion_labels.last_mut() {\n                Some(e) => {\n                    e.written.insert(base.clone());",
                      "            match None::<&mut ExpansionLabelScope> {\n                Some(e) => {\n                    e.written.insert(base.clone());"),
    # Ownership ignores the previous pass (only this pass's filings count).
    "M5": ("eval.rs", ".find(|e| e.written.contains(&base) || prev.is_some_and(|p| p.contains(&e.key)))",
                      ".find(|e| e.written.contains(&base) || (prev.is_some() && false))"),
    # `ifdef`/DEFINED answer for a `$$` name as for any other.
    "M6": ("eval.rs", "if !is_temp_sym(n) && self.resolve_sym(n).is_some())", "if self.resolve_sym(n).is_some())"),
    # A `$$` value binding moves the scope like a plain one.
    "M7": ("eval.rs", "if !name.starts_with('.') && !is_temp_sym(name) {\n            self.open_scope(name);",
                      "if !name.starts_with('.') {\n            self.open_scope(name);"),
    # `sym_key` not idempotent on a built temp key.
    "M8": ("eval.rs", "        if name.contains('@') {\n            return name.to_string();\n        }\n", ""),
    # A value binder does not move the `$$` scope (only the `.` one).
    "M9": ("eval.rs", "    fn open_scope(&mut self, name: &str) {\n        self.temp_scope = name.to_string();",
                      "    fn open_scope(&mut self, name: &str) {"),
    # The lexer reads `$$name` only under the 68000.
    "M10": ("lexer.rs", "            b'$' if bytes.get(i + 1) == Some(&b'$')\n                && bytes.get(i + 2)",
                        "            b'$' if cpu == Cpu::M68000 && bytes.get(i + 1) == Some(&b'$')\n                && bytes.get(i + 2)"),
}
mid = sys.argv[1]
f, old, new = M[mid]
p = W / f
s = p.read_text()
assert s.count(old) == 1, (mid, s.count(old))
p.write_text(s.replace(old, new))
print(f"{mid} applied to {f}:")
for l in (new or "(deleted: " + old.strip().splitlines()[0] + " ...)").splitlines():
    print("    " + l)
