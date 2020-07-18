# makes a 64Mb drive, and optionally copies a firmware binary into the firmware
# partition (if the firmware file is specified as the first arg)

dd if=/dev/zero of=ipodhd.img count=$((2 * 1024 * 64)) bs=512
sfdisk ipodhd.img << EOM
label: dos
label-id: 0x04206969
device: ipodhd.img
unit: sectors

ipodhd.img1 : start=        2048, size=       10240, type=0, bootable
ipodhd.img2 : start=       12288, size=      118784, type=b
EOM

if [ -n "$1" ]; then
    dd if=$1 of=ipodhd.img bs=512 seek=2048 conv=notrunc
fi

dd if=/dev/zero of=ipodhd_fat32.img count=$((118784)) bs=512
mkdosfs -F 32 ipodhd_fat32.img
dd if=ipodhd_fat32.img of=ipodhd.img bs=512 seek=12288 conv=notrunc

# cleanup
rm ipodhd_fat32.img
