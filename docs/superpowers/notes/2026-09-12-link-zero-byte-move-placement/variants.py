#!/usr/bin/env python3
"""Write the reduction variants of engine/system/boot_data.emp into the scratch dir.

Every variant starts from the pristine aeon cae58661 boot_data.emp (constants.emp keeps its
cae58661 copy of VDP_REG_PLANE_SIZE unless the variant says otherwise; see the runner).

  v_const_end    : const + comment moved to file end, NO ensure anywhere (constants.emp's
                   const removed, so the dc.b forward-references the local const)
  v_ensure_end   : the f3664087 ensure alone appended at file end, const stays in constants.emp
  v_trivial_end  : a one-line `ensure(1 == 1, "zbm")` appended at file end; nothing else changes
  v_red_nouse    : the red form but PLANE_* imported on its own new use line (the f3664087 style)
"""
import subprocess

S = "/home/volence/sonic_hacks/.scratch/link-zero-byte-move"
base = open(f"{S}/boot_data.cae58661.emp").read()
f366 = subprocess.run(
    ["git", "-C", "/home/volence/sonic_hacks/aeon", "show", "f3664087:engine/system/boot_data.emp"],
    check=True, capture_output=True, text=True).stdout
start = f366.index("// Reg $10, the plane size: the HARDWARE half")
const_line = "const VDP_REG_PLANE_SIZE = $11\n"
ens_start = f366.index("ensure((PLANE_H_CELLS == 32", start)
ens_end = f366.index("\n", f366.index("       \"BootData_VDPRegs writes reg $10")) + 1
comment = f366[start:f366.index(const_line, start)]
ensure_txt = f366[ens_start:ens_end]

USE_OLD = "use engine.constants.{Z80_RAM, Z80_BUS_REQUEST, Z80_RESET, VDP_DATA, VDP_CTRL, VDP_REG_0C_BOOT, VDP_REG_PLANE_SIZE}\n"
USE_NEW = "use engine.constants.{Z80_RAM, Z80_BUS_REQUEST, Z80_RESET, VDP_DATA, VDP_CTRL, VDP_REG_0C_BOOT}\n"
USE2_OLD = "use engine.constants.{VRAM_SPRITE_TABLE, VRAM_HSCROLL_TABLE}\n"
USE2_NEW = "use engine.constants.{VRAM_SPRITE_TABLE, VRAM_HSCROLL_TABLE, PLANE_H_CELLS, PLANE_V_CELLS}\n"
assert base.count(USE_OLD) == 1 and base.count(USE2_OLD) == 1


def tail(src, block):
    if not src.endswith("\n"):
        src += "\n"
    return src + "\n" + block


v = {}
v["v_const_end"] = tail(base.replace(USE_OLD, USE_NEW), comment + const_line)
v["v_ensure_end"] = tail(base.replace(USE2_OLD, USE2_NEW), ensure_txt)
v["v_trivial_end"] = tail(base, 'ensure(1 == 1, "zbm")\n')
v["v_red_nouse"] = tail(
    base.replace(USE_OLD, USE_NEW).replace(USE2_OLD, USE2_OLD + "use engine.constants.{PLANE_H_CELLS, PLANE_V_CELLS}\n"),
    comment + const_line + ensure_txt)
for name, text in v.items():
    open(f"{S}/boot_data.{name}.emp", "w").write(text)
    print(name, text.count("\n"), "lines")
