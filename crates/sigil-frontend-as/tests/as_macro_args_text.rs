//! A macro argument is TEXT, pasted as written.
//!
//! Sonic 2 writes `Pal_SS1_2p:palette Special Stage 1 2p.bin` three times, and
//! asl pastes the words into `BINCLUDE "art/palettes/path"`. sigil lexed the
//! invocation line as expression tokens, so `2p` was a malformed number and the
//! line was refused; and wherever the tokens DID lex, sigil pasted them back
//! re-rendered rather than as written, which is silent: `$10` became `16`,
//! `007` became `7`, `%101` became `5`, `12h` became `18`, `'A'` became `65`,
//! and a tab inside an argument became a space, each at exit 0. A parameter's
//! default text had the same defect.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -i .`, one construct
//! per probe (`m5*.asm` in
//! `docs/superpowers/notes/2026-09-11-s2-as-small-features/probes/`). Every
//! expected byte and every expected message text is read out of a run that
//! exited 0: the listing prints each expansion's substituted line beside its
//! bytes. Refusals are that build's non-zero answer, read for accept-or-refuse
//! only.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | the unlexable words accepted, the lexable ones still re-rendered (`2p` fixed, `$10` not) | `every_argument_is_pasted_as_written` |
//! | the lexable ones pasted as written, the unlexable ones still refused (`$10` fixed, `2p` not) | `every_argument_is_pasted_as_written`, `sonic_2_s_palette_call_assembles` |
//! | `ALLARGS` taken from the raw operand rather than the trimmed arguments | `arguments_split_at_commas_and_keep_their_insides` |
//! | a keyword value left untrimmed | `a_keyword_argument_splits_at_its_first_equals_sign` |
//! | a parameter's default still re-rendered | `a_parameter_default_is_pasted_as_written` |

use sigil_frontend_as::{assemble_root_located_warned, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const HEAD_Z80: &str = "\tcpu z80\n\torg 0\n";

struct Out {
    bytes: Vec<u8>,
    messages: Vec<String>,
}

fn assemble_with(head: &str, body: &str) -> Result<Out, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    let a = assemble_root_located_warned(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&a.module.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .map_err(|e| vec![format!("{e:?}")])?;
    Ok(Out { bytes: sigil_link::flatten(&linked, 0x00).unwrap(), messages: a.messages })
}

/// `(invocation, the text asl's listing shows substituted, then the bytes
/// after it)`: each call assembles to the text's own bytes followed by `tail`.
fn check_pasted(head: &str, mac: &str, cases: &[(&str, &str, &[u8])]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(call, text, tail)| {
            let mut want = text.as_bytes().to_vec();
            want.extend_from_slice(tail);
            match assemble_with(head, &format!("{mac}{call}")) {
                Ok(o) if o.bytes == want => None,
                Ok(o) => Some(format!("  {call:?}\n    asl   {text:?}\n    sigil {:?}", String::from_utf8_lossy(&o.bytes))),
                Err(d) => Some(format!("  {call:?}\n    asl   {text:?}\n    sigil refused {d:?}")),
            }
        })
        .collect();
    assert!(wrong.is_empty(), "{} of {} calls differ from asl:\n{}", wrong.len(), cases.len(), wrong.join("\n"));
}

/// `(invocation, asl's message text, asl's bytes)`.
fn check_messages(head: &str, mac: &str, cases: &[(&str, &str, &[u8])]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(call, msg, bytes)| match assemble_with(head, &format!("{mac}{call}")) {
            Ok(o) if o.messages == [msg.to_string()] && o.bytes == *bytes => None,
            Ok(o) => Some(format!("  {call:?}\n    asl   {msg:?} {bytes:02X?}\n    sigil {:?} {:02X?}", o.messages, o.bytes)),
            Err(d) => Some(format!("  {call:?}\n    asl   {msg:?}\n    sigil refused {d:?}")),
        })
        .collect();
    assert!(wrong.is_empty(), "{} of {} calls differ from asl:\n{}", wrong.len(), cases.len(), wrong.join("\n"));
}

const PAL: &str = "pal\tmacro path\n\tdc.b \"path\"\n\tendm\n";

/// Every shape the probes tried, pasted into a string. The first group is the
/// lexer's refusals (digit-led words, `\`, `@`); the second is what the lexer
/// reads as a number and a re-rendering changes.
#[test]
fn every_argument_is_pasted_as_written() {
    check_pasted(
        HEAD,
        PAL,
        &[
            ("\tpal Special Stage 1 2p.bin", "Special Stage 1 2p.bin", &[]),
            ("\tpal 1up", "1up", &[]),
            ("\tpal 0x41", "0x41", &[]),
            ("\tpal 1st 2nd 3rd 4th.bin", "1st 2nd 3rd 4th.bin", &[]),
            ("\tpal 0FFh 1F 2G", "0FFh 1F 2G", &[]),
            ("\tpal a  1   2p", "a  1   2p", &[]),
            ("\tpal $10", "$10", &[]),
            ("\tpal 007", "007", &[]),
            ("\tpal %101", "%101", &[]),
            ("\tpal 12h", "12h", &[]),
            ("\tpal 1.5e3", "1.5e3", &[]),
            ("\tpal 'A'", "'A'", &[]),
            ("\tpal Special Stage 1.bin", "Special Stage 1.bin", &[]),
            ("\tpal 2.bin", "2.bin", &[]),
            ("\tpal 1+2", "1+2", &[]),
            ("\tpal -1", "-1", &[]),
        ],
    );
    // An escape in the argument is processed where the pasted string is used:
    // `pal \x41` into `dc.b "<path>"` is `3C 41 3E`.
    check_pasted(HEAD, "pal\tmacro path\n\tdc.b \"<path>\"\n\tendm\n", &[("\tpal \\x41", "<A>", &[])]);
    check_pasted(
        HEAD_Z80,
        "pal\tmacro path\n\tdb \"path\"\n\tendm\n",
        &[
            ("\tpal Special Stage 1 2p.bin", "Special Stage 1 2p.bin", &[]),
            ("\tpal 0101b 1Fh 0FFh", "0101b 1Fh 0FFh", &[]),
        ],
    );
}

/// Sonic 2's `palette` macro, with `dc.b` in place of `BINCLUDE`: a
/// `{INTLABEL}` capture, a trailing comment on the call, and a second argument.
#[test]
fn sonic_2_s_palette_call_assembles() {
    let src = "palette macro {INTLABEL},path,path2\n__LABEL__ label *\n\tdc.b \"path\"\n\
               \x20   if \"path2\"<>\"\"\n\tdc.b \"path2\"\n\x20   endif\n__LABEL___End label *\n\tendm\n\
               Pal_SS1_2p:palette Special Stage 1 2p.bin ; Special Stage 1 2p palette\n\
               Pal_SS2:   palette Special Stage 2.bin,Sonic and 2p.bin ; two\n\
               \tdc.l Pal_SS1_2p_End-Pal_SS1_2p\n\tdc.l Pal_SS2_End-Pal_SS2";
    let o = assemble_with(HEAD, src).expect("asl assembles this, exit 0");
    let mut want = b"Special Stage 1 2p.binSpecial Stage 2.binSonic and 2p.bin".to_vec();
    want.extend_from_slice(&[0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x23]);
    assert_eq!(String::from_utf8_lossy(&o.bytes), String::from_utf8_lossy(&want));
}

const MSG3: &str = "m\tmacro pa,pb,pc\n\tmessage \"(pa)(pb)(pc)[ALLARGS]\"\n\tdc.b ARGCOUNT\n\tendm\n";

/// Commas split, outside parentheses, brackets and literals; each argument is
/// trimmed and keeps what is inside it, a tab included; `ALLARGS` is the
/// arguments rejoined with bare commas; `ARGCOUNT` counts written arguments.
#[test]
fn arguments_split_at_commas_and_keep_their_insides() {
    check_messages(
        HEAD,
        MSG3,
        &[
            ("\tm   aa  ,  bb  ,cc   ; comment", "(aa)(bb)(cc)[aa,bb,cc]", &[0x03]),
            ("\tm (1,2),3", "((1,2))(3)()[(1,2),3]", &[0x02]),
            ("\tm [1,2],3", "([1,2])(3)()[[1,2],3]", &[0x02]),
            ("\tm ,bb,", "()(bb)()[,bb,]", &[0x03]),
            ("\tm aa,   ,cc", "(aa)()(cc)[aa,,cc]", &[0x03]),
            ("\tm 1up,2p.bin,3rd", "(1up)(2p.bin)(3rd)[1up,2p.bin,3rd]", &[0x03]),
            ("\tm $10,007,%101", "($10)(007)(%101)[$10,007,%101]", &[0x03]),
            ("\tm a\t\tb", "(a\t\tb)()()[a\t\tb]", &[0x01]),
            ("\tm a@b", "(a@b)()()[a@b]", &[0x01]),
        ],
    );
    let quoted = "m\tmacro pa,pb\n\tdc.b pa\n\tmessage \"(pb)\"\n\tdc.b ARGCOUNT\n\tendm\n";
    check_messages(
        HEAD,
        quoted,
        &[
            ("\tm \"a,b\",c", "(c)", &[0x61, 0x2C, 0x62, 0x02]),
            ("\tm \"a;b\",c", "(c)", &[0x61, 0x3B, 0x62, 0x02]),
            ("\tm \" a \",c", "(c)", &[0x20, 0x61, 0x20, 0x02]),
        ],
    );
    check_messages(
        HEAD,
        "m\tmacro pa,pb\n\tdc.l pa\n\tmessage \"(pb)\"\n\tdc.b ARGCOUNT\n\tendm\n",
        &[("\tm 'a,b',c", "(c)", &[0x00, 0x61, 0x2C, 0x62, 0x02])],
    );
    check_messages(
        HEAD_Z80,
        "m\tmacro pa,pb\n\tmessage \"(pa)(pb)\"\n\tdb ARGCOUNT\n\tendm\n",
        &[("\tm af',bb", "(af')(bb)", &[0x02])],
    );
}

/// The FIRST `=` outside parentheses and literals splits a keyword argument,
/// and both sides are trimmed. `2<=3` is a keyword named `2<`, which asl
/// refuses (`#1811`).
#[test]
fn a_keyword_argument_splits_at_its_first_equals_sign() {
    check_messages(
        HEAD,
        MSG3,
        &[
            ("\tm pb==5", "()(=5)()[pb==5]", &[0x01]),
            ("\tm pb= 5", "()(5)()[pb= 5]", &[0x01]),
            ("\tm pb =5", "()(5)()[pb =5]", &[0x01]),
            ("\tm aa, pb = 5 ", "(aa)(5)()[aa,pb = 5]", &[0x02]),
            ("\tm (pb=5)", "((pb=5))()()[(pb=5)]", &[0x01]),
        ],
    );
    let refused = assemble_with(HEAD, &format!("{MSG3}\tm 2<=3"));
    assert!(
        matches!(&refused, Err(d) if d.iter().any(|m| m.contains("keyword argument `2<`"))),
        "asl refuses `m 2<=3` (#1811): {:?}",
        refused.map(|o| o.messages)
    );
}

/// A parameter's DEFAULT is the callee's text, pasted as written too.
#[test]
fn a_parameter_default_is_pasted_as_written() {
    check_pasted(HEAD, "m\tmacro pa=$10\n\tdc.b \"pa\"\n\tendm\n", &[("\tm", "$10", &[])]);
    check_pasted(HEAD, "m\tmacro pa=007 %101\n\tdc.b \"pa\"\n\tendm\n", &[("\tm", "007 %101", &[])]);
}

/// Text in, text out: an argument that is not a number is still refused where
/// the body spends it as one (asl `#1020` for `dc.b 2p`), and one that is gives
/// its value (`$10` is `10`, `007` is `07`).
#[test]
fn a_pasted_argument_is_still_judged_where_it_is_used() {
    let val = "val\tmacro n\n\tdc.b n\n\tendm\n";
    assert!(assemble_with(HEAD, &format!("{val}\tval 2p")).is_err(), "asl refuses dc.b 2p");
    check_pasted(HEAD, val, &[("\tval $10", "", &[0x10]), ("\tval 007", "", &[0x07])]);
}
