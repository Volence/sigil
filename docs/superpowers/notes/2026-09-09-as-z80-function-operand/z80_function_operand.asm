	cpu z80
	org 0
zROMWindow:	equ 8000h
Z80_Clock:	equ 3579545
SegaPCM:	equ 0E8FC0h
zmake68kPtr  function addr,zROMWindow+(addr&7FFFh)
zmake68kBank function addr,(((addr&0FF8000h)/zROMWindow))
pcmLoopCounterBase function sampleRate,baseCycles, 1+(Z80_Clock/(sampleRate)-(baseCycles)+(13/2))/13
pcmLoopCounter function sampleRate, pcmLoopCounterBase(sampleRate,90)
	ld	a,zmake68kBank(SegaPCM)&1
	ld	a,zmake68kBank(SegaPCM)>>1
	ld	de,zmake68kPtr(SegaPCM)
	ld	b,pcmLoopCounter(16000)
	ld	de,(zmake68kPtr(SegaPCM))
	ld	a,(zmake68kPtr(SegaPCM))
	ld	(ix+zmake68kBank(SegaPCM)),c
	ld	hl,zmake68kPtr(SegaPCM)
	ld	hl,(zmake68kPtr(SegaPCM))
	jp	zmake68kPtr(SegaPCM)
	call	zmake68kPtr(SegaPCM)
	bit	zmake68kBank(SegaPCM)&7,a
	nop
	ret
