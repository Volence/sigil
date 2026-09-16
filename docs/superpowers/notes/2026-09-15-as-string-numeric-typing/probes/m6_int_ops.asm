	cpu 68000
	padding off
	org 0
	dc.w "ab"-1
	dc.w "ab"*2
	dc.w -"ab"
	dc.w "ab"&$00FF
	dc.w "ab">>8
	dc.w "ab"|1
	dc.w "ab"^1
	dc.w ~"ab"
	dc.w "ab"/2
	dc.w ("ab")
	dc.w "ab"+1-1
	dc.w 1+2+"ab"
	end
