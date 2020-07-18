# takes one argument: a path to the rockbox.zip
# if no argument is given, automatically downloads a rockbox release from the web instead

if ! [ -x "$(command -v unzip)" ]; then
  echo 'please install `unzip` to use this script' >&2
  exit 1
fi

if ! [ -x "$(command -v mcopy)" ]; then
  echo 'please install `mtools` to use this script' >&2
  exit 1
fi

ROCKBOX_ZIP_PATH=$1
if [ -z "$1" ]; then
    echo "Didn't provide path to self-built rockbox.zip. Downloading a binary instead."
    if ! [ -x "$(command -v wget)" ]; then
      echo 'please install `wget` to use this script' >&2
      exit 1
    fi
    ROCKBOX_ZIP_PATH=/tmp/rockbox-ipod4g-3.15.zip
    wget -nc -P /tmp/ https://download.rockbox.org/release/3.15/rockbox-ipod4g-3.15.zip
fi

unzip -o $ROCKBOX_ZIP_PATH -d /tmp/rockbox

IMG_FATPART="ipodhd.img@@$((12288 * 512))"

mcopy -o -s -i $IMG_FATPART /tmp/rockbox/.rockbox ::
mdir -i $IMG_FATPART
