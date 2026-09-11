"""mutate.py <name> : apply one named half-fix to the worktree source.

Each mutation replaces an exact original text that must occur EXACTLY once
(asserted), writes the file, then reads the file back from disk and prints the
mutated line(s) with their line numbers, so the proof shows the patch landed.
Restoring is not done here: the caller restores from the committed baseline
with `git show HEAD:<path> > <path>`.
"""
import sys
WT = '/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a3c01b40c82d95852/'
EVAL = 'crates/sigil-frontend-as/src/eval.rs'
LEX = 'crates/sigil-frontend-as/src/lexer.rs'
ESC = 'crates/sigil-frontend-as/src/escape.rs'
M = {
    # escapes in dc.b strings, NOT in charset string targets
    'charset-target-raw': (EVAL,
        "                        match self.literal_value(raw, false) {\n                            Ok(s) => Some(s),",
        "                        match Ok::<String, crate::escape::EscapeError>(raw.clone()) {\n                            Ok(s) => Some(s),"),
    # escapes in charset string targets, NOT in dc.b strings
    'dcb-raw': (EVAL,
        "                match self.literal_value(raw, false) {\n                    Ok(s) => {\n                        let cs = &self.state.charset;",
        "                match Ok::<String, crate::escape::EscapeError>(raw.clone()) {\n                    Ok(s) => {\n                        let cs = &self.state.charset;"),
    # \NNN (first digit 1-9) read in the wrong base: octal instead of decimal
    'decimal-as-octal': (ESC,
        "            let v = radix_value(text, 10);",
        "            let v = radix_value(text, 8);"),
    # \0NNN read in the wrong base: decimal instead of octal
    'octal-as-decimal': (ESC,
        "            let v = radix_value(text, 8);\n            if text.bytes().any(|d| d > b'7') || v > 0xFF {",
        "            let v = radix_value(text, 10);\n            if text.bytes().any(|d| d > b'9') || v > 0xFF {"),
    # an escaped character becomes a raw byte that bypasses the live page (dc.b)
    'dcb-escape-bypasses-page': (EVAL,
        "                        let bytes: Vec<u8> = s.chars().map(|c| cs.map_char(c)).collect();\n                        self.emit(&bytes, vec![], span);\n                    }\n                    Err(e) => self.err(gspan, e.to_string()),",
        "                        let bytes: Vec<u8> = s.chars().map(|c| if raw.contains('\\\\') { c as u8 } else { cs.map_char(c) }).collect();\n                        self.emit(&bytes, vec![], span);\n                    }\n                    Err(e) => self.err(gspan, e.to_string()),"),
    # an escaped character in a character constant bypasses the live page
    'char-escape-bypasses-page': (LEX,
        "                for &ch in &body {\n                    v = (v << 8) | i64::from(cs.map_char(ch as char));\n                }",
        "                let escaped = line[i + 1..close].contains('\\\\');\n                for &ch in &body {\n                    v = (v << 8) | if escaped { i64::from(ch) } else { i64::from(cs.map_char(ch as char)) };\n                }"),
    # character constants left unescaped: `charset '\H'` is refused again
    'char-unescaped': (LEX,
        "                let body = crate::escape::unescape_bytes(&line[i + 1..close]).map_err(|e| {",
        "                let body = Ok::<Vec<u8>, crate::escape::EscapeError>(line[i + 1..close].as_bytes().to_vec()).map_err(|e| {"),
}
name = sys.argv[1]
path, old, new = M[name]
p = WT + path
text = open(p, encoding='utf-8').read()
n = text.count(old)
assert n == 1, (name, 'original text occurs %d times' % n)
text = text.replace(old, new)
open(p, 'w', encoding='utf-8').write(text)
back = open(p, encoding='utf-8').read().split('\n')
first = new.split('\n')[0]
for i, l in enumerate(back, 1):
    if l == first:
        print('MUTATION %s APPLIED, %s:%d' % (name, path, i))
        for j in range(i, i + new.count('\n') + 1):
            print('  %5d| %s' % (j, back[j - 1]))
