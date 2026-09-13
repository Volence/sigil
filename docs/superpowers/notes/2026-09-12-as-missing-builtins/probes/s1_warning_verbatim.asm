	cpu 68000
	padding off
	org 0
signedToString function number,substr("-",0,-sgn(number))+"$\{abs(number)}"
v_chunk0collision equ $FFFFFF0C
	warning	"RAM variable declarations are \{signedToString(-6)} bytes smaller than expected. Some variables may be missing or not aligned correctly!"
	warning "v_chunk0collision needs to be at address $FFFFFF00 so that FindNearestTile works correctly (currently offset by \{signedToString(v_chunk0collision-$FFFFFF00)} bytes) ."
	dc.b $EE
	end
