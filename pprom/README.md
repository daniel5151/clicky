# pprom

PortalPlayer-based iPod ROM parsing.

## Usage

```
$ cargo run -p pprom internal_rom_000000-0FFFFF-in1g.bin info
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/pprom internal_rom_000000-0FFFFF-in1g.bin info`
model:  IpodNano1g
length: 1048576 bytes
SCfg:   12 keys
```

```
$ cargo run -p pprom internal_rom_000000-0FFFFF-in1g.bin keys
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/pprom internal_rom_000000-0FFFFF-in1g.bin keys`
SCfg v1.1: 12 keys

key: SrNm
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: FwId
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: HwId
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: Btry
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: RtcA
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: HwVr
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: Regn
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: Mod#
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: HwO1
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: Cont
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: MLBN
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
key: DrmV
0000: FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF  ................
```

Data is blank in the above examples for obvious reasons.
