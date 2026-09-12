#!/usr/bin/env python3
"""Reconstruct the never-committed red boot_data.emp from aeon cae58661.

Takes the pristine cae58661 boot_data.emp and:
  * drops VDP_REG_PLANE_SIZE from the engine.constants `use` line,
  * folds PLANE_H_CELLS, PLANE_V_CELLS into the existing VRAM_SPRITE_TABLE `use` line,
  * appends the f3664087 comment + const + ensure block at file end (after the dc.b use).
Writes the result to argv[2].
"""
import subprocess, sys

src = open(sys.argv[1]).read()
out_path = sys.argv[2]

old_use = "use engine.constants.{Z80_RAM, Z80_BUS_REQUEST, Z80_RESET, VDP_DATA, VDP_CTRL, VDP_REG_0C_BOOT, VDP_REG_PLANE_SIZE}\n"
new_use = "use engine.constants.{Z80_RAM, Z80_BUS_REQUEST, Z80_RESET, VDP_DATA, VDP_CTRL, VDP_REG_0C_BOOT}\n"
assert src.count(old_use) == 1
src = src.replace(old_use, new_use)

old_use2 = "use engine.constants.{VRAM_SPRITE_TABLE, VRAM_HSCROLL_TABLE}\n"
new_use2 = "use engine.constants.{VRAM_SPRITE_TABLE, VRAM_HSCROLL_TABLE, PLANE_H_CELLS, PLANE_V_CELLS}\n"
assert src.count(old_use2) == 1
src = src.replace(old_use2, new_use2)

# The block exactly as f3664087 added it (comment, const, ensure), taken from aeon history.
f366 = subprocess.run(
    ["git", "-C", "/home/volence/sonic_hacks/aeon", "show", "f3664087:engine/system/boot_data.emp"],
    check=True, capture_output=True, text=True).stdout
start = f366.index("// Reg $10, the plane size: the HARDWARE half")
end = f366.index("\n", f366.index("       \"BootData_VDPRegs writes reg $10")) + 1
block = f366[start:end]
assert block.count("const VDP_REG_PLANE_SIZE = $11") == 1 and block.count("ensure(") == 1

if not src.endswith("\n"):
    src += "\n"
src += "\n" + block
open(out_path, "w").write(src)
print("wrote", out_path, "lines", src.count("\n"))
