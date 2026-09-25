#!/usr/bin/env bash
B=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex/bin
F=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex/four_shapes.sh
$F before $B/sigil-base $B/emit_sound_blob-base
$F after $B/sigil-tip $B/emit_sound_blob-tip
echo FOUR_BOTH_END
