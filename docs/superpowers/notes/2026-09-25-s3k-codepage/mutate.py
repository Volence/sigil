#!/usr/bin/env python3
"""mutate.py <id>: apply one named mutation to the SUBJECT (state.rs / eval.rs)."""
import sys

W = "/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a08cead7e136bc593/crates/sigil-frontend-as/src/"
M = {
    # a new page starts from the identity instead of copying the selected page
    "M1": ("state.rs",
           "None => base_page.unwrap_or_else(|| self.charset.clone()),",
           "None => base_page.unwrap_or_else(CodePage::identity),"),
    # restore does not reselect the saved page
    "M2": ("state.rs",
           "        self.select_code_page(&s.page, None)\n            .expect(\"a page selected at `save` exists at `restore`\");\n",
           "        let _ = &s.page;\n"),
    # a base on an existing page copies the base again
    "M3": ("state.rs",
           "Some(i) => self.other_pages.swap_remove(i).1,",
           "Some(i) => { let own = self.other_pages.swap_remove(i).1; base_page.clone().unwrap_or(own) }"),
    # the base is looked up only for a new page
    "M4": ("state.rs",
           "                None => return Err(CodePageError::UnknownBase(b.to_string())),",
           "                None if self.other_pages.iter().any(|(n, _)| n == name) => None,\n                None => return Err(CodePageError::UnknownBase(b.to_string())),"),
    # accept and ignore: the silent defect
    "M5": ("eval.rs",
           "    fn directive_codepage(&mut self, rest: &[Token], span: Span) {\n",
           "    fn directive_codepage(&mut self, rest: &[Token], span: Span) {\n        if !rest.is_empty() { return; }\n"),
    # names compare case-insensitively
    "M6": ("eval.rs",
           "                return Some(s.clone());",
           "                return Some(s.to_ascii_uppercase());"),
    # any single token is a name
    "M7": ("eval.rs",
           "        if let [Token { tok: Tok::Ident(s), .. }] = toks {",
           "        if let [Token { tok: Tok::Int(n), .. }] = toks { return Some(n.to_string()); }\n        if let [Token { tok: Tok::Ident(s), .. }] = toks {"),
}
mid = sys.argv[1]
f, old, new = M[mid]
p = W + f
s = open(p).read()
assert s.count(old) == 1, (mid, s.count(old))
open(p, "w").write(s.replace(old, new, 1))
print(f"{mid} applied to {f}")
