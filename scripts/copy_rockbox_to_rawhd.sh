if ! [ -x "$(command -v wget)" ]; then
  echo 'please install `wget` to use this script' >&2
  exit 1
fi

if ! [ -x "$(command -v unzip)" ]; then
  echo 'please install `unzip` to use this script' >&2
  exit 1
fi

if ! [ -x "$(command -v mcopy)" ]; then
  echo 'please install `mtools` to use this script' >&2
  exit 1
fi

wget -nc -P /tmp/ https://download.rockbox.org/release/3.15/rockbox-ipod4g-3.15.zip
unzip -n /tmp/rockbox-ipod4g-3.15.zip -d /tmp/rockbox

IMG_FATPART="ipodhd.img@@$((12288 * 512))"

mcopy -s -i $IMG_FATPART /tmp/rockbox/.rockbox ::
mdir -i $IMG_FATPART
