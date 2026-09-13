#!/usr/bin/env python3
"""Write mut/<label>.old and mut/<label>.new for every red-first mutation, and
mut/plan.tsv (label, file, test binary, the test that must go red). Each .old
is text that must occur exactly once in the committed file (mutate.py checks)."""
import os

D = os.path.dirname(os.path.abspath(__file__))
EVAL = "crates/sigil-frontend-as/src/eval.rs"
ESC = "crates/sigil-frontend-as/src/escape.rs"
T = "as_string_concat"

M = []


def add(label, path, old, new, red):
    M.append((label, path, old, new, red))


# --- the tests this successor added -------------------------------------
add("m1-rescan-value", EVAL,
    "            if let Some(s) = self.eval_str(&expanded) {\n                let cs = &self.state.charset;",
    "            if let Some(s) = self.eval_str(&expanded).map(|s| self.interp_text(&s)) {\n                let cs = &self.state.charset;",
    "a_string_value_is_not_scanned_again_for_interpolation")
add("m2-no-literal-fold", EVAL,
    "                if raw.contains(\"\\\\{\") {\n                    let value = self",
    "                if raw.contains(\"\\\\{\") && false {\n                    let value = self",
    "an_interpolation_is_folded_where_its_literal_is")
add("m3-set-binds-call-string", EVAL,
    "        self.eval_str(rest)?;\n        let folded = self.fold_literal_interps(rest, true).ok()?;",
    "        let rest = &self.expand_calls(rest, 0);\n        self.eval_str(rest)?;\n        let folded = self.fold_literal_interps(rest, true).ok()?;",
    "a_function_result_bound_by_equ_keeps_its_integer_reading")
add("m4-set-binds-unfolded-call", EVAL,
    "        self.eval_str(rest)?;\n        let folded = self.fold_literal_interps(rest, true).ok()?;\n        self.eval_str(&folded)",
    "        let called = self.expand_calls(rest, 0);\n        self.eval_str(&called)",
    "a_string_valued_function_bound_by_set_is_never_silently_wrong")
add("m5-interp-probe-unexpanded", EVAL,
    "        let expanded = self.expand_calls(&toks, 0);\n        if self.eval_str(&expanded).is_some() {",
    "        let expanded = self.expand_calls(&toks, 0);\n        if self.eval_str(&toks).is_some() {",
    "a_string_valued_function_pastes_its_string_into_an_interpolation")
add("m6a-lexer-closes-at-inner-quote", ESC,
    "            b'\\\\' if quote == b'\"' && bytes.get(i + 1) == Some(&b'{') => {",
    "            b'\\\\' if false && quote == b'\"' && bytes.get(i + 1) == Some(&b'{') => {",
    "a_quote_inside_an_interpolation_belongs_to_it")
add("m6b-interp-ends-at-first-brace", ESC,
    "            b'\"' | b'\\'' => i = literal_end(bytes, i)? + 1,",
    "            b'\"' | b'\\'' if false => i = literal_end(bytes, i)? + 1,",
    "a_quote_inside_an_interpolation_belongs_to_it")
add("m7-paste-spelling", EVAL,
    "                    let pasted = match self.fold_const(arg) {",
    "                    let pasted = match None::<i64> {",
    "a_pasted_argument_is_its_decimal_value")
add("m8-word-has-underscore-dot", EVAL,
    "                .find(|c: char| !c.is_ascii_alphanumeric())\n                .unwrap_or(rest.len());",
    "                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))\n                .unwrap_or(rest.len());",
    "a_parameter_word_is_letters_and_digits_only")
add("m9-no-pasted-arg-check", EVAL,
    "                if Self::literal_mentions(&body, param) {",
    "                if false && Self::literal_mentions(&body, param) {",
    "a_string_or_float_argument_pasted_into_a_literal_is_refused")
add("m10-fold-const-no-int-builtins", EVAL,
    "        let expanded = self.expand_int_builtin_opt(&expanded)?;",
    "        let expanded = expanded;",
    "a_string_valued_function_pastes_its_string_into_an_interpolation")

# --- the predecessor's half-fix table, one mutation per row ----------------
add("h1-concat-two-literals-only", EVAL,
    "        if let Some(parts) = split_top_plus(toks) {",
    "        if let Some(parts) = split_top_plus(toks).filter(|p| p.len() == 2 && p.iter().all(|x| matches!(x, [Token { tok: Tok::Str(_), .. }]))) {",
    "a_chain_of_concatenations_folds_left_to_right")
add("h2-concat-captures-numbers", EVAL,
    "                out.push_str(&self.eval_str(part)?);",
    "                out.push_str(&self.eval_str(part).or_else(|| self.fold_const(part).map(|v| v.to_string()))?);",
    "plus_over_non_strings_stays_numeric")
add("h3-param-only-inside-interp", EVAL,
    "                Some(arg) => {\n                    let pasted = match self.fold_const(arg) {",
    "                Some(arg) => {\n                    if !out.ends_with(\"\\\\{\") {\n                        out.push_str(word);\n                        rest = tail;\n                        continue;\n                    }\n                    let pasted = match self.fold_const(arg) {",
    "a_function_parameter_reaches_inside_a_string_literal")
add("h4-param-as-substring", EVAL,
    "            let word_len = rest\n                .find(|c: char| !c.is_ascii_alphanumeric())\n                .unwrap_or(rest.len());",
    "            let word_len = params\n                .iter()\n                .find(|p| rest.starts_with(p.as_str()))\n                .map_or(0, |p| p.len());",
    "a_function_parameter_reaches_inside_a_string_literal")
add("h5-paste-unparenthesised", EVAL,
    "                    out.push('(');\n                    out.push_str(&pasted);\n                    out.push(')');",
    "                    out.push_str(&pasted);",
    "a_function_parameter_reaches_inside_a_string_literal")
add("h6-interp-bare-literal-only", EVAL,
    "                match self.fold_literal_interps(&called, false) {\n                    Ok(t) => t,",
    "                match Ok::<Vec<Token>, (Span, crate::escape::EscapeError)>(called.clone()) {\n                    Ok(t) => t,",
    "an_interpolation_is_folded_in_a_computed_string")
add("h8-no-concat-at-all", EVAL,
    "        if let Some(parts) = split_top_plus(toks) {",
    "        if let Some(parts) = split_top_plus(toks).filter(|_| false) {",
    "sonic_1_signed_to_string_assembles")

with open(os.path.join(D, "plan.tsv"), "w") as plan:
    for label, path, old, new, red in M:
        open(os.path.join(D, f"{label}.old"), "w").write(old)
        open(os.path.join(D, f"{label}.new"), "w").write(new)
        plan.write(f"{label}\t{path}\t{T}\t{red}\n")
print(f"wrote {len(M)} mutations")
