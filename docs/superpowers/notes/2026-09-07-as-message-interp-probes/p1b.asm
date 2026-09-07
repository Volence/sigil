	cpu 68000
	padding off
	org 0
n equ 42
h equ $FF
	message "int \{42}"
	message "hex \{$FF}"
	message "sym \{n}"
	message "paren \{(n+3)*2}"
	message "neg \{-1}"
	message "zero \{0}"
	message "intdiv \{7/2}"
	message "here \{*}"
	message "mask \{(*)&$FFFFFFFF}"
	message "char \{'A'}"
	message "cmp \{3<4}"
	message "two \{n} and \{h}"
	message "plain text with no interpolation"
	dc.b 1
	end
