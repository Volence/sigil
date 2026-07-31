//! `eval_all_pub_consts` — the harvest half of the Stage-3 P5 ownership flip
//! (Option A). It resolves every `pub const` in a module to its integer value
//! so the harness can inject them as guarded AS `-D` defines. A derived const
//! must read its already-resolved siblings; a non-`pub` const is not harvested.

use sigil_frontend_emp::eval::eval_all_pub_consts;
use sigil_frontend_emp::parse_str;

#[test]
fn harvests_every_pub_const_including_derived() {
    let src = "\
        module m\n\
        pub const A = 40\n\
        pub const B = 8\n\
        pub const C = 16\n\
        pub const TOTAL = A + B + C\n\
        pub const SHIFTED = 1 << 11\n\
        const PRIVATE = 999\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pdiags:?}");
    let (vals, diags) = eval_all_pub_consts(&file, None, &[]);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "harvest: {diags:?}");

    let map: std::collections::HashMap<_, _> = vals.iter().cloned().collect();
    assert_eq!(map.get("A"), Some(&40));
    assert_eq!(map.get("B"), Some(&8));
    assert_eq!(map.get("C"), Some(&16));
    assert_eq!(map.get("TOTAL"), Some(&64), "derived const must read resolved siblings");
    assert_eq!(map.get("SHIFTED"), Some(&2048));
    assert!(!map.contains_key("PRIVATE"), "a non-pub const must not be harvested");
    assert_eq!(vals.len(), 5, "exactly the five pub consts, in source order: {vals:?}");
}
