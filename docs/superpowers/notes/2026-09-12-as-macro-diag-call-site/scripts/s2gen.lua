-- s2gen.lua: generate the inputs build.lua writes before it assembles, WITHOUT
-- assembling anything: the PCM and DPCM conversions (pure Lua, build.lua's own
-- functions), and every song's `.inc` in its UNCOMPRESSED form (so no asl and no
-- saxman run). Run from the root of a scratch copy of the corpus.
package.path = "./?.lua;" .. package.path
local common = require "build_tools.lua.common"
common.convert_pcm_files_in_directory("sound/PCM")
common.convert_dpcm_files_in_directory("sound/DAC")
os.execute("mkdir -p sound/music/generated")
local listing = io.popen("find sound/music -maxdepth 1 -name '*.asm'", "r")
local n = 0
for path in listing:lines() do
	local stem = path:match("sound/music/(.*)%.asm$")
	local f = io.open("sound/music/generated/" .. stem .. ".inc", "w")
	f:write(string.format(".is_compressed = FALSE\n\tinclude \"sound/music/%s.asm\"\n", stem))
	f:close()
	n = n + 1
end
listing:close()
print("songs written uncompressed: " .. n)
