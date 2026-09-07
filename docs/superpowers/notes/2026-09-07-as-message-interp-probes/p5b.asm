	cpu 68000
	padding off
	org 0
StartOfRom:
	dc.b 1
	dc.w EndOfRom-*
paddingSoFar equ 3
	message "ROM size is $\{EndOfRom-StartOfRom} bytes (\{(EndOfRom-StartOfRom)/1024.0} KiB). About $\{paddingSoFar} bytes are padding. "
	message "#\{n/1.0}: line=\{MOMLINE/1.0} PC=$\{(*)&$FFFFFFFF}"
n equ 7
	dc.b 5,6
EndOfRom:
	end
