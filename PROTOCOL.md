# Havit MS900 (Sinowealth 258a:0029) configuration protocol

This document is the result of reverse engineering the Windows software
`OemDrv.exe` 1.0.6 ("MS900 Gaming Mouse", project name *Combo-8948*,
build dir `d:\OEM\Beiying\Src\Combo-8948\v105`) with static analysis, and of
cross-checking the result with libratbag's `driver-sinowealth.c`, which
targets the same OEM software family (it even references the same
`Cfg.ini` / `MDNUM` keys).

Legend used below:

* **[cap]** – confirmed with a usbmon capture of the Windows software and a
  byte-for-byte comparison with what ms900ctl writes (identical).
* **[bin]** – read directly from the disassembly of `OemDrv.exe`.
* **[ratbag]** – matches libratbag's documented layout for this chip family.

## USB device

| field | value |
|-------|-------|
| VID:PID | `258a:0029`, "SINOWEALTH Wired Gaming Mouse", bcdDevice 1.03 |
| interface 0 | HID boot mouse, EP 0x81, report descriptor 71 bytes |
| interface 1 | HID boot keyboard + vendor collections, EP 0x82, report descriptor 213 bytes |
| sensor | PixArt PMW3327, id `0x0e` **[bin]** (Cfg.ini `Sensor=0x3327`) |
| DPI range | 200 .. 12400 in 100 DPI steps (Cfg.ini `DR=0x500`, UI slider) |

The vendor collections are on interface 1: usage page `0xff00`, usage `1`
**[bin]** (`OpenHidDevice(0xff00, 1, ...)` in `CDev8948::FindDev`).  The
Windows code opens them by *feature report length*: one collection whose
feature report is 6 bytes (command channel) and one whose feature report is
0x208 = 520 bytes (data channel) **[bin]** (`Aly8948_1` / `Aly8948_2`
validators).  On Linux both are reachable through the single hidraw node of
interface 1 by report id.

## Transport

| report | id | size | direction | purpose |
|--------|----|------|-----------|---------|
| command | `0x05` | 6 | SET + GET feature | select a block / short queries |
| data | `0x04` | 520 | SET + GET feature | config, buttons, macro blocks |

`SET_FEATURE(05, cmd, a, b, c, d)` then `Sleep(20 ms)` then `GET_FEATURE(04)`
reads a block **[bin] [cap]**.  The firmware answers the version query with
the ASCII string `V284` on this unit.  The GET answer repeats the command in byte 1; the
Windows code retries three times with 200 ms pauses **[bin]**.

Commands **[bin]**:

| cmd | args | meaning |
|-----|------|---------|
| `0x01` | – | firmware version: GET report 5 returns 4 bytes at 2..5 (compared with Cfg.ini `PSD=56,32,38,34`) |
| `0x02` | – | GET report 5 byte 2 = active on-board profile (1..3) |
| `0x02` | n | SET: switch to profile n (1..3) |
| `0x11`/`0x21`/`0x31` | – | select the config block of profile 1/2/3 |
| `0x12`/`0x22`/`0x32` | – | select the button block of profile 1/2/3 |
| `0x30` | – | macro block (write only, report 4 byte 2 = `0x02`) |

Writing a block: SET report 4 with byte 1 = block command, byte 3 =
`block_length - 8` (`0x7b` for a 131-byte config, `0x50` for the 88-byte
button block) **[bin]** (`mov byte [esp+0x12], 0x7b`, `0x50` in the two
writers at `0x413a70` / `0x413b20`).  The MS900 software only ever writes
profile 1 (Cfg.ini `MDNUM=3` is the three-"mode" variant that stays hidden).

## Config block (`0x11`)

Byte offsets are report offsets (byte 0 = report id 4).  The block is 131
bytes on this mouse (`GET_FEATURE` returns 131, written with byte 3 =
`0x7b`) **[cap]**.  Anchors `+9` sensor id, `+11` DPI count/active, `+12`
disabled mask, `+129` lift-off distance come from the binary **[bin]**;
the whole block written by ms900ctl is identical to the Windows one **[cap]**.
Byte 8 is `0x64` on this firmware and is preserved as read.

| offset | size | content |
|--------|------|---------|
| 1 | 1 | command (`0x11`) |
| 3 | 1 | write length (`len-8`) when writing, 0 when reading |
| 9 | 1 | sensor id (`0x0e` = PMW3327) |
| 10 | 1 | low nibble polling rate: 1=125, 2=250, 3=500, 4=1000 Hz; bit 7: X/Y independent DPI |
| 11 | 1 | low nibble: number of DPI stages (6); high nibble: active stage, 1-based counting enabled stages only |
| 12 | 1 | bit mask of *disabled* stages |
| 13 | 16 | DPI values, one byte per stage = 1-based index into the DPI table below, or x/y pairs when bit 7 of +10 is set **[cap]** |
| 29 | 24 | RGB colour of each stage (8 × 3) |
| 53 | 1 | lighting effect, see table |
| 54 | 1 | Colorful Streaming: speed (low nibble 1..3) / brightness (high nibble 1..4) |
| 55 | 1 | Colorful Streaming direction (0 up / 1 down) |
| 56 | 1 | Steady brightness (high nibble only, e.g. `0x40` = 100 %) |
| 57 | 3 | Steady colour |
| 60 | 1 | Breathing speed/brightness |
| 61 | 1 | Breathing colour count (1..7) |
| 62 | 21 | Breathing colours |
| 83 | 1 | Tail speed/brightness |
| 84 | 1 | Neon speed/brightness |
| 85 | 1 | Colorful Steady brightness (high nibble only) |
| 86 | 18 | Colorful Steady colours (6 zones) |
| 116 | 1 | Flicker speed/brightness |
| 117 | 6 | Flicker colours (2) |
| 123 | 1 | Stars twinkle speed/brightness |
| 124 | 1 | Wave speed/brightness |
| 125 | 1 | Reaction speed (low nibble only) |
| 126 | 3 | Reaction colour |
| 129 | 1 | lift-off distance (1 = 2 mm, 2 = 3 mm) |
| 130 | 1 | lighting off flag (written as 0 by the MS900 dialog) |

### DPI table (PMW3327) **[bin] [cap]**

The raw DPI byte is `position + 1` in this list (table 11 of the sensor
tables built in `OemDrv.exe` at `0x46db20`; 1200 -> `0x0b`, 6200 -> `0x3d`,
10000 -> `0x44` in the capture):

```
200 300 ... 6200 (100 steps), 6500 7000 7500 8000 8500 9000 10000 10500 11000 11500 12000 12400
```

Effect ids **[bin]** (the Windows software builds the table at startup:
entries 1..9 in combo-box order, then `2 + 8 = 0x0a` for *Reaction*, then 0
for *None*; `LedOpt6=6,...` in Cfg.ini confirms *Colorful Steady* = 6):

| UI name | id | | UI name | id |
|---------|----|-|---------|----|
| None | 0 | | Colorful Steady | 6 |
| Colorful Streaming | 1 | | Flicker | 7 |
| Steady | 2 | | Stars twinkle | 8 |
| Breathing | 3 | | Wave | 9 |
| Tail | 4 | | Reaction | 0x0a **[cap]** |
| Neon | 5 | | | |

Colorful Steady (6), Reaction (0x0a) and None (0) were captured; the others
come from the same table.  Byte 130 stays 0 even for *None*.

## Button block (`0x12`)

88 bytes: 8 header bytes then 20 slots of 4 bytes; the MS900 uses slots
0..13 in UI order 1-8, S1-S6 and writes `50 01 00 00` (Disable) into the six
unused slots **[cap]**.  Slot encoding `[type, a, b, c]`, taken from
`keyinfo_to_hardware_code` at `0x414360` **[bin]**:

| function | bytes |
|----------|-------|
| Left / Right / Middle / Forward / Back | `11 01` / `11 02` / `11 04` / `11 10` / `11 08` |
| Scroll Up / Down | `12 01` / `12 ff` |
| Scroll Right / Left (not shown for MS900) | `13 01` / `13 ff` |
| Double / Three Click | `31 01 32 02` / `31 01 32 03` |
| Fire key (mouse button, interval ms, count; 0 = while held) | `31 mask interval count` |
| Fire key on a keyboard key | `32 usage interval count` |
| Fire key on a modifier | `33 modbits interval count` |
| Key Combination | `21 modifiers usage 00` (modifiers: 1 Ctrl, 2 Shift, 4 Alt, 8 Win) |
| Multimedia | `22 m1 m2 m3`: m1 1 Next, 2 Previous, 4 Stop, 8 Play/Pause, 0x10 Mute, 0x40 Vol+, 0x80 Vol-; m2 1 Media Player, 2 Explorer, 0x10 Email, 0x20 Calculator; m3 1 Search, 2 Home page, 4 Back, 8 Forward, 0x10 Stop, 0x20 Refresh, 0x40 Favorites |
| DPI + / DPI - / DPI Loop | `41 01` / `41 02` / `41 00` |
| DPI Lock | `42 raw` (raw = dpi/100 - 1) |
| Disable | `50 01` |
| Switch polling rate / mode / effect | `50 04` / `50 06` / `50 07` |
| Macro | `70 index mode count` (index 1-based macro buffer; mode 1 = N times, 2 = until released, 4 = until any key) |

## Macro block (`0x30`)

`[04 30 02 00 00 00 00 00 index 00 count ev...]`, each event 3 bytes
`[cmd, delay_ms, code]` with cmd `0x10` button press, `0x90` button release,
`0x50` key press, `0xd0` key release; at most 168 events **[ratbag] [cap]**.
The delay byte belongs to the event it follows (1 = no delay).  The Windows
"Apply" writes the macro block first, then the button block, then reads and
rewrites the config block **[cap]**.

## Things the Windows program does *not* send to the mouse

The "Mouse Parameter" panel (sensitivity, pointer precision, scroll lines,
scroll one page, double-click speed) only calls the Windows
`SystemParametersInfo` API; ms900ctl applies the equivalents through
gsettings / KDE config / xinput.

## `.bkm` backups

See `src/bkm.rs` for the file layout used by "Export profile".  Macro bodies
inside the file are encrypted with an 8-byte block cipher (ECB, constant
`c3 12 8f 7c 57 c9 bb 9e` for zero blocks) and are not imported.

## Verification status

The "Apply" of the Windows software was captured with usbmon
(`tools/capture-usbmon.sh`, decoded with `tools/decode-capture.py`), the
same profile was then applied from Linux with `ms900ctl --apply-json`, and
the three written feature reports (macro, buttons, config) were identical
byte for byte.  `ms900ctl --dump` reads the on-board blocks back and shows
the same bytes.  Switching the Windows software to Reaction, None and
Colorful Steady was captured as well (effect byte 0x0a / 0x00 / 0x06).
