	cpu 68000
	padding off
	org 0
s equ "abc"
t := "xyz"
n equ 42
	message "str \{s}"
	message "set \{t}"
	message "soundBank \{s} has $\{$8000+n-*} bytes free at end."
	message "Table \{s} has \{n/1.0} entries, but it should have \{(n)/1.0} entries"
	dc.b 1
	end
