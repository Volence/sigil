	cpu 68000
	padding off
	org 0
	move.w #"ab"-1,d0
	move.w #"ab"*2,d0
	move.w #-"ab",d0
	move.w #"ab"&$00FF,d0
	move.w #"ab">>8,d0
	move.w #~"ab",d0
	move.w #"ab"/2,d0
	move.w #"ab"|1,d0
	move.w #"ab"+1-1,d0
	end
