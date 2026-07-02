use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::parse_str;

#[test]
fn section_decls() {
    let (f, diags) = parse_str(concat!(
        "module m\n",
        "section z80_driver (cpu: z80, vma: $0000) {\n",
        "    data X: [u8; 2] = [1, 2]\n",
        "}\n"));
    assert!(diags.is_empty(), "{diags:?}");
    let Item::Section(s) = &f.items[0] else { panic!() };
    assert_eq!(s.name, "z80_driver");
    assert_eq!(s.attrs.len(), 2);
    assert_eq!(s.attrs[0].0, "cpu");
    assert_eq!(s.items.len(), 1);
}

/// The spec's Appendix-D pitcher-plant exhibit must parse with zero diagnostics.
#[test]
fn appendix_d_pitcher_plant() {
    let src = concat!(
        "module badniks.pitcher_plant in obj_bank\n",
        "\n",
        "vars PitcherPlantV: sst_custom { timer: u8 }\n",
        "\n",
        "const WAIT_TIME: u8 = 64\n",
        "const ATTACK_RANGE: u16 = $60\n",
        "enum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }\n",
        "\n",
        "pub data Def = ObjDef{ code: init, map: Map_PitcherPlant,\n",
        "                       art: ArtTile{tile: VRAM_PITCHER_PLANT, pal: 0, pri: 0},\n",
        "                       col: Collision.Hurt, size: Size{w: 16, h: 28}, anim: Anim.Idle }\n",
        "\n",
        "proc init (a0: *Sst) falls_into wait {\n",
        "    move.b  #WAIT_TIME, timer(a0)\n",
        "}\n",
        "proc wait (a0: *Sst) {\n",
        "    subq.b  #1, timer(a0)\n",
        "    bne     .draw\n",
        "    move.b  #WAIT_TIME, timer(a0)\n",
        ".draw:\n",
        "    jmp     Draw_Sprite\n",
        "}\n");
    let (f, diags) = parse_str(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(f.module.path.segments, vec!["badniks", "pitcher_plant"]);
    assert_eq!(f.module.in_section.as_deref(), Some("obj_bank"));
    assert_eq!(f.items.len(), 7); // vars(1) + const(2) + enum(1) + data(1) + proc(2)
}

/// A comptime exhibit in the spec's Appendix-A style must also parse clean.
#[test]
fn appendix_a_style_comptime() {
    let src = concat!(
        "module engine.parallax\n",
        "\n",
        "comptime fn deform_sine(amplitude: int, period: int) -> [i8; 256] {\n",
        "    ensure(256 % period == 0, \"deform_sine: 256 not divisible by period {period}\")\n",
        "    return comptime for i in 0..256 { as.int(amplitude * as.sin(TAU * i / period)) }\n",
        "}\n",
        "\n",
        "data RockingDeform: [i8; 256] = deform_sine(amplitude: 20, period: 64)\n");
    let (_, diags) = parse_str(src);
    assert!(diags.is_empty(), "{diags:?}");
}

/// One broken decl must not poison the rest of the file.
#[test]
fn error_recovery_continues_after_bad_decl() {
    let (f, diags) = parse_str(concat!(
        "module m\n",
        "const = 5\n",                 // broken
        "const GOOD: u8 = 1\n"));      // must still parse
    assert!(!diags.is_empty());
    assert!(f.items.iter().any(|i| matches!(i, Item::Const(c) if c.name == "GOOD")));
}
