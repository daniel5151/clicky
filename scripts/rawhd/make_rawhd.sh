# makes a 64Mb drive, and optionally copies a firmware binary into the firmware
# partition (if the firmware file is specified as the first arg)

dd if=/dev/zero of=ipodhd.img bs=512 count=0 seek=$((2 * 1024 * 64)) status=progress
sfdisk ipodhd.img << EOM
label: dos
label-id: 0x04206969
device: ipodhd.img
unit: sectors

ipodhd.img1 : start=        2048, size=       10240, type=0, bootable
ipodhd.img2 : start=       12288, size=      118784, type=b
EOM

if [ -n "$1" ]; then
    dd if=$1 of=ipodhd.img bs=512 seek=2048 conv=notrunc status=progress
fi

dd if=/dev/zero of=ipodhd_fat32.img bs=512 count=0 seek=$((118784)) status=progress
mkdosfs -F 32 ipodhd_fat32.img
dd if=ipodhd_fat32.img of=ipodhd.img bs=512 seek=12288 conv=notrunc status=progress

# cleanup
rm ipodhd_fat32.img
