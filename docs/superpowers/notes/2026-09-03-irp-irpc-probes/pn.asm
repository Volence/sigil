	cpu 68000
	padding off
	org $1000
z0 = 0
z1 = 1
z5 = 5
	dc.b ~~z0,~~z1,~~z5,~z0&$FF,~z1&$FF
	end
