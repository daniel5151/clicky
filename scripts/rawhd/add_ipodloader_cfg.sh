DEFAULT_DEBUG_VAL=1

# requires `mtools` to be installed
if ! [ -x "$(command -v mcopy)" ]; then
  echo 'please install `mtools` to use this script' >&2
  exit 1
fi

DEBUG_VAL=${1:-$DEFAULT_DEBUG_VAL}

cat <<EOM >ipodloader.conf
# config file
debug = $DEBUG_VAL
timeout = 3
EOM

IMG_FATPART="ipodhd.img@@$((12288 * 512))"

mcopy -o -i $IMG_FATPART ipodloader.conf ::
mdir -i $IMG_FATPART

# cleanup
rm ipodloader.conf
