#!/usr/bin/env python3
"""MEASUREMENT SCAFFOLD, never committed: report every macro call whose
argument text as written differs from its tokens re-rendered (the text the
pre-parcel front end pasted). Env-gated on SIGIL_ARGTEXT_CENSUS."""
P = "/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a35527f94a0279a97/crates/sigil-frontend-as/src/eval.rs"
src = open(P).read()
old = '''            if let Some(operand) = operand {
                return split_macro_args(operand)
                    .into_iter()
                    .map(|(s, e)| CallArg {
                        text: operand[s..e].to_string(),
                        span: Span {
                            source: *source,
                            start: head.end + s as u32,
                            end: head.end + e as u32,
                        },
                    })
                    .collect();
            }'''
new = '''            if let Some(operand) = operand {
                let out: Vec<CallArg> = split_macro_args(operand)
                    .into_iter()
                    .map(|(s, e)| CallArg {
                        text: operand[s..e].to_string(),
                        span: Span {
                            source: *source,
                            start: head.end + s as u32,
                            end: head.end + e as u32,
                        },
                    })
                    .collect();
                if std::env::var_os("SIGIL_ARGTEXT_CENSUS").is_some() {
                    let rendered: Vec<String> = if arg_toks.is_empty() {
                        Vec::new()
                    } else {
                        split_top_commas(arg_toks).into_iter().map(render_tokens).collect()
                    };
                    let raw: Vec<&str> = out.iter().map(|a| a.text.as_str()).collect();
                    if rendered != raw {
                        eprintln!(
                            "SIGIL-ARGTEXT\\t{}\\t{:?}\\t{:?}",
                            self.sources.label(head).unwrap_or_default(),
                            rendered,
                            raw
                        );
                    }
                }
                return out;
            }'''
assert src.count(old) == 1
src = src.replace(old, new)
open(P, "w").write(src)
print("census scaffold applied")
