; ---------------------------------------------------------------------------
; macro to simplify editing the demo scripts
; (taken from the Sonic 2 disassembly)
; ---------------------------------------------------------------------------

demoinput macro buttons,duration
btns_mask := 0
    irpc btn,"buttons"
      switch "btn"
	case "U"
btns_mask := btns_mask|btnUp
	case "D"
btns_mask := btns_mask|btnDn
	case "L"
btns_mask := btns_mask|btnL
	case "R"
btns_mask := btns_mask|btnR
	case "A"
btns_mask := btns_mask|btnA
	case "B"
btns_mask := btns_mask|btnB
	case "C"
btns_mask := btns_mask|btnC
	case "S"
btns_mask := btns_mask|btnStart
	elsecase
      endcase
    endm
	dc.b	btns_mask,duration-1
 endm

; ---------------------------------------------------------------------------
