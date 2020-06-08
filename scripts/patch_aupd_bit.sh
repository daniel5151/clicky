# toggles a bit in the firmware to disable running the flash ROM update (which doesn't work rn)
printf "%b" "\001" | dd of=ipodhd.img bs=1 seek=$((0x00104230)) count=1 conv=notrunc
