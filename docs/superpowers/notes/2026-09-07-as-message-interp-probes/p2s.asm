	cpu 68000
	padding off
	org 0
fs equ 2.5
fi equ 1024.0
n equ 42
	message "fdiv \{(2048-1024)/1024.0}"
	message "fdiv2 \{1536/1024.0}"
	message "one \{n/1.0}"
	message "flit \{1024.0}"
	message "flit2 \{3.5}"
	message "third \{1/3.0}"
	message "twothird \{2/3.0}"
	message "tenth \{0.1}"
	message "sum \{0.1+0.2}"
	message "big \{1.0e17}"
	message "bigger \{1.0e21}"
	message "small \{1.0e-5}"
	message "smaller \{1.5e-7}"
	message "negf \{-2.5}"
	message "negint \{-1024.0}"
	message "mixed \{123456789.5}"
	message "sixdig \{123456.0}"
	message "sevendig \{1234567.0}"
	message "eightdig \{12345678.0}"
	message "long \{1.23456789012345}"
	message "fsym \{fs}"
	message "fsymint \{fi}"
	message "mul \{2.5*4}"
	message "cmpf \{3.5<4}"
	message "million \{1000000.0}"
	message "tenmillion \{10000000.0}"
	message "point5 \{0.5}"
	message "hundredth \{0.01}"
	message "thousandth \{0.001}"
	message "tiny \{0.0001}"
	message "tinier \{0.00001}"
	message "zerof \{0.0}"
	message "hexf \{$400/1024.0}"
	dc.b 1
	end
