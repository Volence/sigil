	cpu 68000
	padding off
	org 0
n	:= 42
s	:= "\{n}"
neg	:= -1
sneg	:= "\{neg}"
	dc.b s
	dc.b $ff
	message "len=\{strlen(s)}"
	message "lenneg=\{strlen(sneg)}"
	end
