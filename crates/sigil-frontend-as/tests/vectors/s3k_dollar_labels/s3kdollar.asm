	cpu 68000
	supmode on
; simplifying macros and functions
; nameless temporary symbols should NOT be used inside macros because they can interfere with the surrounding code
; normal labels should be used instead (which automatically become local to the macro)

; sign-extends a 32-bit integer to 64-bit
; all RAM addresses are run through this function to allow them to work in both 16-bit and 32-bit addressing modes
ramaddr function x,(-(x&$80000000)<<1)|x

; makes a VDP command
vdpComm function addr,type,rwd,(((type&rwd)&3)<<30)|((addr&$3FFF)<<16)|(((type&rwd)&$FC)<<2)|((addr&$C000)>>14)

; values for the type argument
VRAM = %100001
CRAM = %101011
VSRAM = %100101

; values for the rwd argument
READ = %001100
WRITE = %000111
DMA = %100111

; tells the VDP to copy a region of 68k memory to VRAM or CRAM or VSRAM
dma68kToVDP macro source,dest,length,type
	lea	(VDP_control_port).l,a5
	move.l	#(($9400|((((length)>>1)&$FF00)>>8))<<16)|($9300|(((length)>>1)&$FF)),(a5)
	move.l	#(($9600|((((source)>>1)&$FF00)>>8))<<16)|($9500|(((source)>>1)&$FF)),(a5)
	move.w	#$9700|(((((source)>>1)&$FF0000)>>16)&$7F),(a5)
	move.w	#(vdpComm(dest,type,DMA)>>16)&$FFFF,(a5)
	move.w	#vdpComm(dest,type,DMA)&$FFFF,(DMA_trigger_word).w
	move.w	(DMA_trigger_word).w,(a5)
	; From '  § 7  DMA TRANSFER' of https://emu-docs.org/Genesis/sega2f.htm:
	;
	; "In the case of ROM to VRAM transfers,
	; a hardware feature causes occasional failure of DMA unless the
	; following two conditions are observed:
	;
	; --The destination address write (to address $C00004) must be a word
	;   write.
	;
	; --The final write must use the work RAM.
	;   There are two ways to accomplish this, by copying the DMA program
	;   into RAM or by doing a final "move.w ram address $C00004""
    endm

; tells the VDP to fill a region of VRAM with a certain byte
dmaFillVRAM macro byte,addr,length
	lea	(VDP_control_port).l,a5
	move.w	#$8F01,(a5) ; VRAM pointer increment: $0001
	move.l	#(($9400|((((length)-1)&$FF00)>>8))<<16)|($9300|(((length)-1)&$FF)),(a5) ; DMA length ...
	move.w	#$9780,(a5) ; VRAM fill
	move.l	#vdpComm(addr,VRAM,DMA),(a5) ; Start at ...
	move.w	#(byte)|((byte)<<8),(VDP_data_port).l ; Fill with byte
.loop:	move.w	(a5),d1
	btst	#1,d1
	bne.s	.loop ; busy loop until the VDP is finished filling...
	move.w	#$8F02,(a5) ; VRAM pointer increment: $0002
    endm

; calculates initial loop counter value for a dbf loop
; that writes n bytes total at 4 bytes per iteration
bytesToLcnt function n,n>>2-1

; calculates initial loop counter value for a dbf loop
; that writes n bytes total at 2 bytes per iteration
bytesToWcnt function n,n>>1-1

; tells the Z80 to stop, and waits for it to finish stopping (acquire bus)
stopZ80 macro
	move.w	#$100,(Z80_bus_request).l ; stop the Z80
.loop:	btst	#0,(Z80_bus_request).l
	bne.s	.loop ; loop until it says it's stopped
    endm

; tells the Z80 to start again
startZ80 macro
	move.w	#0,(Z80_bus_request).l    ; start the Z80
    endm
; Stand-ins for names the extract uses and defines elsewhere in sonic3k.asm.
; Hardware ports at their real addresses; ROM routines and tables at distinct
; even addresses inside a word branch's reach; RAM through S3K's own `ramaddr`.
VDP_data_port		= $C00000
VDP_control_port	= $C00004
Z80_bus_request		= $A11100
SRAM_access_flag	= $A130F1
LockonHeader		= $200000
LockonSerialNumber	= $200180
SonicAndKnucklesStartup	= $3A00
Nem_Decomp		= $3A10
Queue_Kos		= $3A24
Do_Updates		= $3A38
Add_To_DMA_Queue	= $3A4C
Offs_PLC		= $3A60
SegaHeadersText		= $3A74
V_blank_cycles		= ramaddr($FFFFF62E)
H_int_flag		= ramaddr($FFFFF644)
Water_palette_data_addr	= ramaddr($FFFFF610)
H_int_counter		= ramaddr($FFFFF625)
Water_palette		= ramaddr($FFFFF080)
Do_Updates_in_H_int	= ramaddr($FFFFF64F)
Normal_palette		= ramaddr($FFFFFC00)
VDP_reg_1_command	= ramaddr($FFFFF60E)
H_int_counter_command	= ramaddr($FFFFF624)
V_scroll_value		= ramaddr($FFFFF616)
_unkF61A		= ramaddr($FFFFF61A)
DMA_queue		= ramaddr($FFFFFB00)
DMA_queue_slot		= ramaddr($FFFFFBFC)
Nem_decomp_queue	= ramaddr($FFFFF680)
Kos_module_queue	= ramaddr($FFFFFF7C)
Kos_modules_left	= ramaddr($FFFFF7DE)
Kos_last_module_size	= ramaddr($FFFFF7E0)
Kos_module_destination	= ramaddr($FFFFF7E2)
Kos_decomp_queue_count	= ramaddr($FFFFFF5E)
Kos_decomp_buffer	= ramaddr($FFFF9000)
Kos_decomp_queue	= ramaddr($FFFFFF60)
	org $2A6
Test_SystemString:
		lea	(LockonHeader).l,a0
		moveq	#0,d3
		moveq	#$10-1,d2

$$compareChars:
		move.b	(a1)+,d0
		cmp.b	(a0)+,d0
		beq.s	$$matchingChar
		moveq	#1,d3

$$matchingChar:
		dbf	d2,$$compareChars
		tst.b	d3
		beq.s	DetermineWhichGame
		dbf	d4,Test_SystemString
		moveq	#-1,d1
		move.l	(SegaHeadersText).l,d0		; test to see if SEGA is at the locked on ROM's $100
		cmp.l	(LockonHeader).l,d0
		bne.w	SonicAndKnucklesStartup

DetermineWhichGame:
		lea	(LockonSerialsText).l,a1
		moveq	#3,d1	; 3 Sonic 2 headers, 1 Sonic 3 header

$$compareSerials:
		lea	(LockonSerialNumber).l,a0
		moveq	#0,d3
		moveq	#$E-1,d2

$$compareChars:
		move.b	(a1)+,d0
		cmp.b	(a0)+,d0
		beq.s	$$matchingChar
		moveq	#1,d3

$$matchingChar:
		dbf	d2,$$compareChars
		tst.b	d3
		beq.s	S2orS3LockedOn
		dbf	d1,$$compareSerials
		bra.s	BlueSpheresStartup
; ---------------------------------------------------------------------------

S2orS3LockedOn:
		tst.w	d1
		beq.w	SonicAndKnucklesStartup
		move.b	#1,(SRAM_access_flag).l
		jmp	($300000).l				; May be changed at a later date to become compatible with S2K disassembly
; ---------------------------------------------------------------------------
LockonSerialsText:
		dc.b "GM 00001051-00"	; Sonic 2 REV00/1/2
		dc.b "GM 00001051-01"
		dc.b "GM 00001051-02"
		dc.b "GM MK-1079 -00"	; Sonic 3
BlueSpheresStartup:
	nop

DetectPAL:
		lea	(VDP_control_port).l,a5
		move.w	#$8174,(a5)		; VDP Command $8174 - Display on, VInt on, DMA on, PAL off
		moveq	#0,d0

$$waitForVBlankStart:
		move.w	(a5),d1
		andi.w	#8,d1
		beq.s	$$waitForVBlankStart

$$waitForVBlankEnd:
		move.w	(a5),d1
		andi.w	#8,d1
		bne.s	$$waitForVBlankEnd	; Wait for VBlank to run once

$$waitForNextVBlank:
		addq.w	#1,d0
		move.w	(a5),d1
		andi.w	#8,d1
		beq.s	$$waitForNextVBlank
		move.w	d0,(V_blank_cycles).w	; Count cycles between VBlanks (likely to detect PAL systems and/or for other timing mechanisms
		beq.s	HInt3_Done
		move.w	#0,(H_int_flag).w
		movem.l	d0-d1/a0-a2,-(sp)

		lea	(VDP_data_port).l,a1
		move.w	#$8AFF,VDP_control_port-VDP_data_port(a1)		; Reset HInt timing
		stopZ80
		movea.l	(Water_palette_data_addr).w,a2
		moveq	#$C,d0
		dbf	d0,*	; waste a few cycles here
		move.w	(a2)+,d1
		move.b	(H_int_counter).w,d0
		subi.b	#200,d0	; is H-int occuring below line 200?
		bcs.s	$$transferColors	; if it is, branch
		sub.b	d0,d1
		bcs.s	$$skipTransfer

$$transferColors:
		move.w	(a2)+,d0
		lea	(Water_palette).w,a0
		adda.w	d0,a0
		addi.w	#$C000,d0
		swap	d0
		move.l	d0,VDP_control_port-VDP_data_port(a1)	; write to CRAM at appropriate address
		move.l	(a0)+,(a1)	; transfer two colors
		move.w	(a0)+,(a1)	; transfer the third color
		nop
		nop
		moveq	#$24,d0
		dbf	d0,*	; waste some cycles
		dbf	d1,$$transferColors	; repeat for number of colors

$$skipTransfer:
		startZ80
		movem.l	(sp)+,d0-d1/a0-a2
		tst.b	(Do_Updates_in_H_int).w
		bne.s	HInt3_Do_Updates

HInt3_Done:
		rte
; ---------------------------------------------------------------------------

HInt3_Do_Updates:
		clr.b	(Do_Updates_in_H_int).w
		movem.l	d0-a6,-(sp)
		jsr	(Do_Updates).l
		movem.l	(sp)+,d0-a6
		rte

; ---------------------------------------------------------------------------
; Identical to HInt3 except it transfers colors from the above water palette
; Seems to be unused
; ---------------------------------------------------------------------------

HInt5:
		tst.w	(H_int_flag).w		; Seems to be a compliment to HInt 3, but doesn't seem to be used
		beq.s	HInt5_Done
		move.w	#0,(H_int_flag).w
		movem.l	d0-d1/a0-a2,-(sp)

		lea	(VDP_data_port).l,a1
		move.w	#$8AFF,VDP_control_port-VDP_data_port(a1)
		stopZ80
		movea.l	(Water_palette_data_addr).w,a2
		moveq	#$C,d0
		dbf	d0,*

		move.w	(a2)+,d1
		move.b	(H_int_counter).w,d0
		subi.b	#200,d0
		bcs.s	$$transferColors
		sub.b	d0,d1
		bcs.s	$$skipTransfer

$$transferColors:
		move.w	(a2)+,d0
		lea	(Normal_palette).w,a0
		adda.w	d0,a0
		addi.w	#$C000,d0
		swap	d0
		move.l	d0,VDP_control_port-VDP_data_port(a1)
		move.l	(a0)+,(a1)
		move.w	(a0)+,(a1)
		nop
		nop
		moveq	#$24,d0
		dbf	d0,*	; waste some cycles
		dbf	d1,$$transferColors

$$skipTransfer:
		startZ80
		movem.l	(sp)+,d0-d1/a0-a2
		tst.b	(Do_Updates_in_H_int).w
		bne.s	HInt5_Do_Updates

HInt5_Done:
		rte
; ---------------------------------------------------------------------------

HInt5_Do_Updates:
		clr.b	(Do_Updates_in_H_int).w
		movem.l	d0-a6,-(sp)
		jsr	(Do_Updates).l
		movem.l	(sp)+,d0-a6
		rte

; ---------------------------------------------------------------------------
; Identical to HInt3, except for faster systems
; ---------------------------------------------------------------------------

HInt4:
		tst.w	(H_int_flag).w
		beq.s	Hint4_Done
		move.w	#0,(H_int_flag).w
		movem.l	d0-d1/a0-a2,-(sp)

		lea	(VDP_data_port).l,a1
		move.w	#$8AFF,VDP_control_port-VDP_data_port(a1)
		stopZ80
		movea.l	(Water_palette_data_addr).w,a2
		moveq	#$1B,d0
		dbf	d0,*

		move.w	(a2)+,d1
		move.b	(H_int_counter).w,d0
		subi.b	#200,d0
		bcs.s	$$transferColors
		sub.b	d0,d1
		bcs.s	$$skipTransfer

$$transferColors:
		move.w	(a2)+,d0
		lea	(Water_palette).w,a0
		adda.w	d0,a0
		addi.w	#$C000,d0
		swap	d0
		move.l	d0,VDP_control_port-VDP_data_port(a1)
		move.l	(a0)+,(a1)
		move.w	(a0)+,(a1)
		nop
		moveq	#$33,d0
		dbf	d0,*	; waste some cycles
		dbf	d1,$$transferColors

$$skipTransfer:
		startZ80
		movem.l	(sp)+,d0-d1/a0-a2
		tst.b	(Do_Updates_in_H_int).w
		bne.s	HInt4_Do_Updates

Hint4_Done:
		rte
; ---------------------------------------------------------------------------

HInt4_Do_Updates:
		clr.b	(Do_Updates_in_H_int).w
		movem.l	d0-a6,-(sp)
		jsr	(Do_Updates).l
		movem.l	(sp)+,d0-a6
		rte

; ---------------------------------------------------------------------------
; Identical to HInt5, except for faster systems
; ---------------------------------------------------------------------------

HInt_6:
		tst.w	(H_int_flag).w
		beq.s	HInt6_Done
		move.w	#0,(H_int_flag).w
		movem.l	d0-d1/a0-a2,-(sp)

		lea	(VDP_data_port).l,a1
		move.w	#$8AFF,VDP_control_port-VDP_data_port(a1)
		stopZ80
		movea.l	(Water_palette_data_addr).w,a2
		moveq	#$1B,d0
		dbf	d0,*

		move.w	(a2)+,d1
		move.b	(H_int_counter).w,d0
		subi.b	#200,d0
		bcs.s	$$transferColors
		sub.b	d0,d1
		bcs.s	$$skipTransfer

$$transferColors:
		move.w	(a2)+,d0
		lea	(Normal_palette).w,a0
		adda.w	d0,a0
		addi.w	#$C000,d0
		swap	d0
		move.l	d0,VDP_control_port-VDP_data_port(a1)
		move.l	(a0)+,(a1)
		move.w	(a0)+,(a1)
		nop
		moveq	#$33,d0
		dbf	d0,*	; waste some cycles
		dbf	d1,$$transferColors

$$skipTransfer:
		startZ80
		movem.l	(sp)+,d0-d1/a0-a2
		tst.b	(Do_Updates_in_H_int).w
		bne.s	HInt6_Do_Updates

HInt6_Done:
		rte
; ---------------------------------------------------------------------------

HInt6_Do_Updates:
		clr.b	(Do_Updates_in_H_int).w
		movem.l	d0-a6,-(sp)
		jsr	(Do_Updates).l
Init_VDP:
		lea	(VDP_control_port).l,a0
		lea	(VDP_data_port).l,a1
		lea	(VDP_register_values).l,a2
		moveq	#19-1,d7

$$setRegisters:
		move.w	(a2)+,(a0)
		dbf	d7,$$setRegisters
		move.w	(VDP_register_values+2).l,d0	; get command for register #1
		move.w	d0,(VDP_reg_1_command).w	; and store it in RAM (for easy display blanking/enabling)
		move.w	#$8A00+224-1,(H_int_counter_command).w
		moveq	#0,d0
		move.l	#vdpComm($0000,VSRAM,WRITE),(VDP_control_port).l
		move.w	d0,(a1)
		move.w	d0,(a1)
		move.l	#vdpComm($0000,CRAM,WRITE),(VDP_control_port).l
		move.w	#bytesToWcnt($80),d7

$$clearCRAM:
		move.w	d0,(a1)
		dbf	d7,$$clearCRAM
		clr.l	(V_scroll_value).w
		clr.l	(_unkF61A).w
		move.l	d1,-(sp)
		dmaFillVRAM 0,$0000,$10000	; clear entire VRAM
		move.l	(sp)+,d1
		rts
; End of function Init_VDP

; ---------------------------------------------------------------------------
VDP_register_values:
		dc.w $8004	; H-int disabled
Process_DMA_Queue:
		lea	(VDP_control_port).l,a5
		lea	(DMA_queue).w,a1

$$loop:
		move.w	(a1)+,d0	; has a stop token been encountered?
		beq.s	$$stop	; if it has, branch
		move.w	d0,(a5)
		move.w	(a1)+,(a5)
		move.w	(a1)+,(a5)
		move.w	(a1)+,(a5)
		move.w	(a1)+,(a5)
		move.w	(a1)+,(a5)
		move.w	(a1)+,(a5)
		cmpa.w	#DMA_queue_slot,a1	; has the end of the queue been reached?
		bne.s	$$loop	; if not, loop

$$stop:
		move.w	#0,(DMA_queue).w
		move.l	#DMA_queue,(DMA_queue_slot).w
		rts
; End of function Process_DMA_Queue

; ---------------------------------------------------------------------------
; Nemesis decompression subroutine, decompresses art directly to VRAM
; Inputs:
; a0 = art address
; a VDP command to write to the destination VRAM address must be issued
; before calling this routine
; See http://www.segaretro.org/Nemesis_compression for format description
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Load_PLC:
		movem.l	a1-a2,-(sp)
		lea	(Offs_PLC).l,a1
		add.w	d0,d0
		move.w	(a1,d0.w),d0
		lea	(a1,d0.w),a1
		lea	(Nem_decomp_queue).w,a2

$$findFreeSlot:
		tst.l	(a2)	; is the current slot in the queue free?
		beq.s	$$getPieceCount	; if it is, branch
		addq.w	#6,a2	; otherwise check the next slot
		bra.s	$$findFreeSlot
; ---------------------------------------------------------------------------

$$getPieceCount:
		move.w	(a1)+,d0
		bmi.s	$$done

$$queuePieces:
		move.l	(a1)+,(a2)+	; store compressed data location
		move.w	(a1)+,(a2)+	; store destination in VRAM
		dbf	d0,$$queuePieces

$$done:
		movem.l	(sp)+,a1-a2
		rts
; End of function Load_PLC

; ---------------------------------------------------------------------------
; Loads a raw PLC from ROM
; Input: a1 = address of the PLC
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Load_PLC_Raw:
		lea	(Nem_decomp_queue).w,a2

$$findFreeSlot:
		tst.l	(a2)
		beq.s	$$getPieceCount
		addq.w	#6,a2
		bra.s	$$findFreeSlot
; ---------------------------------------------------------------------------

$$getPieceCount:
		move.w	(a1)+,d0
		bmi.s	$$done

$$queuePieces:
		move.l	(a1)+,(a2)+
		move.w	(a1)+,(a2)+
		dbf	d0,$$queuePieces

$$done:
		rts
; End of function Load_PLC_Raw

; ---------------------------------------------------------------------------
; Adds pattern load requests to the Nemesis decompression queue
; Differs from Load_PLC in that it clears the queue before loading
; Input: d0 = ID of the PLC to load
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Load_PLC_2:
		movem.l	a1-a2,-(sp)		; This differs from Load_PLC in that it overrides any PLCs already in the queue
		lea	(Offs_PLC).l,a1
		add.w	d0,d0
		move.w	(a1,d0.w),d0
		lea	(a1,d0.w),a1
		bsr.s	Clear_Nem_Queue
		lea	(Nem_decomp_queue).w,a2
		move.w	(a1)+,d0
		bmi.s	$$done

$$queuePieces:
		move.l	(a1)+,(a2)+
		move.w	(a1)+,(a2)+
		dbf	d0,$$queuePieces

$$done:
		movem.l	(sp)+,a1-a2
		rts
; End of function Load_PLC_2

; ---------------------------------------------------------------------------
; Clears the Nemesis decompression queue and its associated variables
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Clear_Nem_Queue:
		lea	(Nem_decomp_queue).w,a2
		moveq	#bytesToLcnt($80),d0	; clear till the end of Nem_decomp_vars

$$loop:
		clr.l	(a2)+
		dbf	d0,$$loop
		rts
; End of function Clear_Nem_Queue

; ---------------------------------------------------------------------------
; Initializes Nemesis decompression queue processing
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Load_PLC_Immediate:
		move.w	(a1)+,d1

$$decompPieces:
		movea.l	(a1)+,a0	; get source address
		moveq	#0,d0
		move.w	(a1)+,d0	; get destination VRAM address
		lsl.l	#2,d0
		lsr.w	#2,d0
		ori.w	#$4000,d0
		swap	d0	; d0 = VDP command to write to destination
		move.l	d0,(VDP_control_port).l
		bsr.w	Nem_Decomp
		dbf	d1,$$decompPieces
		rts
; End of function Load_PLC_Immediate

; ---------------------------------------------------------------------------
; Enigma decompression subroutine
; Inputs:
; a0 = compressed data location
; a1 = destination (in RAM)
; d0 = starting art tile
; See http://www.segaretro.org/Enigma_compression for format description
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Eni_Decomp:
		movem.l	d0-d7/a1-a5,-(sp)
		movea.w	d0,a3	; store starting art tile
		move.b	(a0)+,d0
		ext.w	d0
		movea.w	d0,a5	; store number of bits in inline copy value
		move.b	(a0)+,d4
		lsl.b	#3,d4	; store PCCVH flags bitfield
		movea.w	(a0)+,a2
		adda.w	a3,a2	; store incremental copy word
		movea.w	(a0)+,a4
		adda.w	a3,a4	; store literal copy word
		move.b	(a0)+,d5
		asl.w	#8,d5
		move.b	(a0)+,d5	; get first word in format list
		moveq	#$10,d6		; initial shift value

Eni_Decomp_Loop:
		moveq	#7,d0	; assume a format list entry is 7 bits
		move.w	d6,d7
		sub.w	d0,d7
		move.w	d5,d1
		lsr.w	d7,d1
		andi.w	#$7F,d1	; get format list entry
		move.w	d1,d2	; and copy it
		cmpi.w	#$40,d1	; is the high bit of the entry set?
		bhs.s	+
		moveq	#6,d0	; if it isn't, the entry is actually 6 bits
		lsr.w	#1,d2

+
		bsr.w	Eni_Decomp_FetchByte
		andi.w	#$F,d2	; get repeat count
		lsr.w	#4,d1
		add.w	d1,d1
		jmp	Eni_Decomp_Index(pc,d1.w)
; ---------------------------------------------------------------------------

Eni_Decomp_00:
		move.w	a2,(a1)+	; copy incremental copy word
		addq.w	#1,a2	; increment it
		dbf	d2,Eni_Decomp_00	; repeat
		bra.s	Eni_Decomp_Loop
; ---------------------------------------------------------------------------

Eni_Decomp_01:
		move.w	a4,(a1)+	; copy literal copy word
		dbf	d2,Eni_Decomp_01	; repeat
		bra.s	Eni_Decomp_Loop
; ---------------------------------------------------------------------------

Eni_Decomp_100:
		bsr.w	Eni_Decomp_FetchInlineValue

$$loop:
		move.w	d1,(a1)+	; copy inline value
		dbf	d2,$$loop	; repeat
		bra.s	Eni_Decomp_Loop
; ---------------------------------------------------------------------------

Eni_Decomp_101:
		bsr.w	Eni_Decomp_FetchInlineValue

$$loop:
		move.w	d1,(a1)+	; copy inline value
		addq.w	#1,d1	; increment
		dbf	d2,$$loop	; repeat
		bra.s	Eni_Decomp_Loop
; ---------------------------------------------------------------------------

Eni_Decomp_110:
		bsr.w	Eni_Decomp_FetchInlineValue

$$loop:
		move.w	d1,(a1)+	; copy inline value
		subq.w	#1,d1	; decrement
		dbf	d2,$$loop	; repeat
		bra.s	Eni_Decomp_Loop
; ---------------------------------------------------------------------------

Eni_Decomp_111:
		cmpi.w	#$F,d2
		beq.s	Eni_Decomp_Done

$$loop:
		bsr.w	Eni_Decomp_FetchInlineValue	; fetch new inline value
		move.w	d1,(a1)+	; copy it
		dbf	d2,$$loop	; and repeat
		bra.s	Eni_Decomp_Loop
; ---------------------------------------------------------------------------

Eni_Decomp_Index:
		bra.s	Eni_Decomp_00
; ---------------------------------------------------------------------------
		bra.s	Eni_Decomp_00
; ---------------------------------------------------------------------------
		bra.s	Eni_Decomp_01
; ---------------------------------------------------------------------------
		bra.s	Eni_Decomp_01
; ---------------------------------------------------------------------------
		bra.s	Eni_Decomp_100
; ---------------------------------------------------------------------------
		bra.s	Eni_Decomp_101
; ---------------------------------------------------------------------------
		bra.s	Eni_Decomp_110
; ---------------------------------------------------------------------------
		bra.s	Eni_Decomp_111
; ---------------------------------------------------------------------------

Eni_Decomp_Done:
		subq.w	#1,a0	; go back by one byte
		cmpi.w	#$10,d6
		bne.s	+
		subq.w	#1,a0	; and another one if needed

+
		move.w	a0,d0
		lsr.w	#1,d0
		bcc.s	+
		addq.w	#1,a0	; make sure it's an even address

+
		movem.l	(sp)+,d0-d7/a1-a5
		rts
; End of function Eni_Decomp

; ---------------------------------------------------------------------------
; Part of the Enigma decompressor
; Fetches an inline copy value and stores it in d1
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Eni_Decomp_FetchInlineValue:
		move.w	a3,d3	; copy starting art tile
		move.b	d4,d1	; copy PCCVH bitfield
		add.b	d1,d1	; is the priority bit set?
		bcc.s	+	; if not, branch
		subq.w	#1,d6
		btst	d6,d5	; is the priority bit set in the inline render flags?
		beq.s	+	; if not, branch
		ori.w	#$8000,d3	; otherwise set priority bit in art tile

+
		add.b	d1,d1	; is the high palette line bit set?
		bcc.s	+	; if not, branch
		subq.w	#1,d6
		btst	d6,d5
		beq.s	+
		addi.w	#$4000,d3

+
		add.b	d1,d1	; is the low palette line bit set?
		bcc.s	+	; if not, branch
		subq.w	#1,d6
		btst	d6,d5
		beq.s	+
		addi.w	#$2000,d3

+
		add.b	d1,d1	; is the vertical flip flag set?
		bcc.s	+	; if not, branch
		subq.w	#1,d6
		btst	d6,d5
		beq.s	+
		ori.w	#$1000,d3

+
		add.b	d1,d1	; is the horizontal flip flag set?
		bcc.s	+	; if not, branch
		subq.w	#1,d6
		btst	d6,d5
		beq.s	+
		ori.w	#$800,d3

+
		move.w	d5,d1
		move.w	d6,d7
		sub.w	a5,d7	; subtract length in bits of inline copy value
		bcc.s	$$enoughBits	; branch if a new word doesn't need to be read
		move.w	d7,d6
		addi.w	#$10,d6
		neg.w	d7	; calculate bit deficit
		lsl.w	d7,d1	; and make space for that many bits
		move.b	(a0),d5	; get next byte
		rol.b	d7,d5	; and rotate the required bits into the lowest positions
		add.w	d7,d7
		and.w	Eni_Decomp_Masks-2(pc,d7.w),d5
		add.w	d5,d1	; combine upper bits with lower bits

$$maskValue:
		move.w	a5,d0	; get length in bits of inline copy value
		add.w	d0,d0
		and.w	Eni_Decomp_Masks-2(pc,d0.w),d1	; mask value appropriately
		add.w	d3,d1	; add starting art tile
		move.b	(a0)+,d5
		lsl.w	#8,d5
		move.b	(a0)+,d5	; get next word
		rts
; ---------------------------------------------------------------------------

$$enoughBits:
		beq.s	$$justEnough	; if the word has been exactly exhausted, branch
		lsr.w	d7,d1	; get inline copy value
		move.w	a5,d0
		add.w	d0,d0
		and.w	Eni_Decomp_Masks-2(pc,d0.w),d1	; and mask it appropriately
		add.w	d3,d1	; add starting art tile
		move.w	a5,d0
		bra.s	Eni_Decomp_FetchByte
; ---------------------------------------------------------------------------

$$justEnough:
		moveq	#$10,d6	; reset shift value
		bra.s	$$maskValue
; End of function Eni_Decomp_FetchInlineValue

; ---------------------------------------------------------------------------
Eni_Decomp_Masks:
		dc.w     1
		dc.w     3
		dc.w     7
		dc.w    $F
		dc.w   $1F
		dc.w   $3F
		dc.w   $7F
		dc.w   $FF
		dc.w  $1FF
		dc.w  $3FF
		dc.w  $7FF
		dc.w  $FFF
		dc.w $1FFF
		dc.w $3FFF
		dc.w $7FFF
		dc.w $FFFF
; ---------------------------------------------------------------------------
; Part of the Enigma decompressor, fetches the next byte if needed
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Eni_Decomp_FetchByte:
		sub.w	d0,d6	; subtract length of current entry from shift value so that next entry is read next time around
		cmpi.w	#9,d6	; does a new byte need to be read?
		bhs.s	+	; if not, branch
		addq.w	#8,d6
		asl.w	#8,d5
		move.b	(a0)+,d5

+
		rts
; End of function Eni_Decomp_FetchByte

; ---------------------------------------------------------------------------
; Kosinski decompression subroutine
; Inputs:
; a0 = compressed data location
; a1 = destination
; See http://www.segaretro.org/Kosinski_compression for format description
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Queue_Kos_Module:
		lea	(Kos_module_queue).w,a2
		tst.l	(a2)	; is the first slot free?
		beq.s	Process_Kos_Module_Queue_Init	; if it is, branch
		addq.w	#6,a2	; otherwise, check next slot

$$findFreeSlot:
		tst.l	(a2)
		beq.s	$$freeSlotFound
		addq.w	#6,a2
		bra.s	$$findFreeSlot
; ---------------------------------------------------------------------------

$$freeSlotFound:
		move.l	a1,(a2)+	; store source address
		move.w	d2,(a2)+	; store destination VRAM address
		rts
; End of function Queue_Kos_Module

; ---------------------------------------------------------------------------
; Initializes processing of the first module on the queue
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Process_Kos_Module_Queue_Init:
		move.w	(a1)+,d3	; get uncompressed size
		cmpi.w	#$A000,d3
		bne.s	+
		move.w	#$8000,d3	; $A000 means $8000 for some reason

+
		lsr.w	#1,d3
		move.w	d3,d0
		rol.w	#5,d0
		andi.w	#$1F,d0	; get number of complete modules
		move.b	d0,(Kos_modules_left).w
		andi.l	#$7FF,d3	; get size of last module in words
		bne.s	+	; branch if it's non-zero
		subq.b	#1,(Kos_modules_left).w	; otherwise decrement the number of modules
		move.l	#$800,d3	; and take the size of the last module to be $800 words

+
		move.w	d3,(Kos_last_module_size).w
		move.w	d2,(Kos_module_destination).w
		move.l	a1,(Kos_module_queue).w
		addq.b	#1,(Kos_modules_left).w	; store total number of modules
		rts
; End of function Process_Kos_Module_Queue_Init

; ---------------------------------------------------------------------------
; Processes the first module on the queue
; ---------------------------------------------------------------------------

; =============== S U B R O U T I N E =======================================


Process_Kos_Module_Queue:
		tst.b	(Kos_modules_left).w
		bne.s	$$modulesLeft

$$done:
		rts
; ---------------------------------------------------------------------------

$$modulesLeft:
		bmi.s	$$decompressionStarted
		cmpi.w	#4,(Kos_decomp_queue_count).w
		bhs.s	$$done	; branch if the Kosinski decompression queue is full
		movea.l	(Kos_module_queue).w,a1
		lea	(Kos_decomp_buffer).w,a2
		bsr.w	Queue_Kos	; add current module to decompression queue
		ori.b	#$80,(Kos_modules_left).w	; and set bit to signify decompression in progress
		rts
; ---------------------------------------------------------------------------

$$decompressionStarted:
		tst.w	(Kos_decomp_queue_count).w
		bne.s	$$done	; branch if the decompression isn't complete

		; otherwise, DMA the decompressed data to VRAM
		andi.b	#$7F,(Kos_modules_left).w
		move.l	#$800,d3
		subq.b	#1,(Kos_modules_left).w
		bne.s	+	; branch if it isn't the last module
		move.w	(Kos_last_module_size).w,d3

+
		move.w	(Kos_module_destination).w,d2
		move.w	d2,d0
		add.w	d3,d0
		add.w	d3,d0
		move.w	d0,(Kos_module_destination).w	; set new destination
		move.l	(Kos_module_queue).w,d0
		move.l	(Kos_decomp_queue).w,d1
		sub.l	d1,d0
		andi.l	#$F,d0
		add.l	d0,d1	; round to the nearest $10 boundary
		move.l	d1,(Kos_module_queue).w	; and set new source
		move.l	#Kos_decomp_buffer,d1
		andi.l	#$FFFFFF,d1
		jsr	(Add_To_DMA_Queue).l
		tst.b	(Kos_modules_left).w
		bne.s	+	; return if this wasn't the last module
		lea	(Kos_module_queue).w,a0
		lea	(Kos_module_queue+6).w,a1
		move.l	(a1)+,(a0)+	; otherwise, shift all entries up
		move.w	(a1)+,(a0)+
		move.l	(a1)+,(a0)+
		move.w	(a1)+,(a0)+
		move.l	(a1)+,(a0)+
		move.w	(a1)+,(a0)+
		move.l	#0,(a0)+	; and mark the last slot as free
		move.w	#0,(a0)+
		move.l	(Kos_module_queue).w,d0
		beq.s	+	; return if the queue is now empty
		movea.l	d0,a1
		move.w	(Kos_module_destination).w,d2
		jmp	(Process_Kos_Module_Queue_Init).l

+
		rts
; End of function Process_Kos_Module_Queue
