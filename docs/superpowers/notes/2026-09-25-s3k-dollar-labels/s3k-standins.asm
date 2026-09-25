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
