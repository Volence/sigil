-- post_steps.lua <disasm-dir> <s2.h> <amend-target.bin> <final-target.bin>
-- Applies build.lua's two post-p2bin steps, copied verbatim in effect:
--   amend_sound_driver_size (build.lua:138-167) to <amend-target>
--   common.fix_header (common.lua:700-727) to <final-target>, AFTER the amend
-- <final-target> must start as a copy of the p2bin output; the amend is applied
-- to it first, so it ends as build.lua's final image.
local dir, hfile, amend_bin, final_bin = arg[1], arg[2], arg[3], arg[4]
package.path = dir .. "/?.lua;" .. package.path
local oldcwd = nil
local common = dofile(dir .. "/build_tools/lua/common.lua")

local function amend(romname)
  local comp_z80_size, movewZ80CompSize
  for line in io.lines(hfile) do
    local b, e = string.find(line, "comp_z80_size")
    if b ~= nil then comp_z80_size = tonumber(line:match("0x%x+", e)) end
    local b2, e2 = string.find(line, "movewZ80CompSize")
    if b2 ~= nil then movewZ80CompSize = tonumber(line:match("0x%x+", e2)) end
  end
  print(string.format("s2.h: comp_z80_size=%s movewZ80CompSize=%s",
    comp_z80_size and string.format("0x%X", comp_z80_size) or "nil",
    movewZ80CompSize and string.format("0x%X", movewZ80CompSize) or "nil"))
  if comp_z80_size ~= nil and movewZ80CompSize ~= nil then
    local rom = io.open(romname, "r+b")
    rom:seek("set", movewZ80CompSize + 2)
    rom:write(string.pack(">I2", comp_z80_size))
    rom:close()
    print(string.format("amend: wrote %04X at file offset 0x%X in %s", comp_z80_size, movewZ80CompSize + 2, romname))
  end
end

amend(amend_bin)
amend(final_bin)
common.fix_header(final_bin)
print("fix_header applied to " .. final_bin)
