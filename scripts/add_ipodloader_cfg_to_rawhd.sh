# requires `mtools` to be installed
if ! [ -x "$(command -v mcopy)" ]; then
  echo 'please install `mtools` to use this script' >&2
  exit 1
fi

cat <<EOM >ipodloader.conf
# config file
debug = 1
EOM

IMG_FATPART="ipodhd.img@@$((12288 * 512))"

mcopy -i $IMG_FATPART ipodloader.conf ::
mdir -i $IMG_FATPART

# cleanup
rm ipodloader.conf
