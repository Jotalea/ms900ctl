# ms900ctl

Native Linux (and, untested, macOS) replacement for the Windows "MS900 Gaming
Mouse" configuration software of the **Havit MS900 / MS760** (USB
`258a:0029`, Sinowealth controller, PMW3327 sensor).  Written in Rust with
egui; the window is a pixel-for-pixel re-creation of the original using its
own skin artwork, and every feature of the original is implemented:

* 14 programmable keys (1-8, S1-S6) with the full function menu: mouse
  buttons, scroll, three click, fire key, key combination, macro, disable,
  DPI +/-/loop, multimedia keys, DPI lock
* 6 DPI stages (200-12400) with on/off, colour and current stage
* Lighting: Colorful Streaming, Steady, Breathing, Tail, Neon, Colorful
  Steady, Flicker, Stars twinkle, Wave, Reaction, None – with the same
  per-effect speed / brightness / colour / direction controls
* Mouse Parameter panel (sensitivity, pointer precision, scrolling speed,
  scroll one page, double-click speed) – applied to the desktop through
  gsettings (GNOME), KDE config or xinput (X11)
* Polling rate 125/250/500/1000 Hz
* Profiles: add, delete, rename, export, import (JSON and the original
  `.bkm` backups), restore defaults
* Macro editor with recording, manual event insertion and the three loop modes
* "Read from mouse" (the one addition): imports the configuration currently
  stored in the mouse

The protocol is documented in [PROTOCOL.md](PROTOCOL.md).

## Building

Requirements: Rust 1.75+ (`cargo`), `pkg-config`, libudev headers, and the
usual egui/winit runtime libraries (libxkbcommon, wayland, libGL, libX11,
libxcursor, libxi, libxrandr, fontconfig).

```sh
# Arch Linux
sudo pacman -S --needed rust pkgconf systemd-libs libxkbcommon wayland libgl libx11 libxcursor libxi libxrandr fontconfig

# Debian / Ubuntu
sudo apt install cargo pkg-config libudev-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev \
                 libx11-dev libxcursor-dev libxi-dev libxrandr-dev libfontconfig1-dev

cd ms900ctl
make            # = cargo build --release  ->  target/release/ms900ctl
make test       # unit tests (protocol encoding, .bkm import)
```

### Arch package

```sh
cd ms900ctl
makepkg -si     # builds and installs the ms900ctl package (binary, udev rule, .desktop, icon)
```

### Any distro, via Docker

```sh
cd ms900ctl
make docker     # builds inside rust:bookworm, copies dist/ms900ctl and dist/70-ms900.rules
sudo install -Dm644 dist/70-ms900.rules /etc/udev/rules.d/70-ms900.rules
sudo udevadm control --reload && sudo udevadm trigger
```

### Manual install

```sh
sudo make install        # /usr/bin/ms900ctl, udev rule, desktop file, icon
sudo udevadm control --reload && sudo udevadm trigger   # or re-plug the mouse
```

## Running

```sh
ms900ctl                 # or start "MS900 Gaming Mouse" from the app menu
RUST_LOG=debug ms900ctl  # verbose HID logging

ms900ctl --dump                 # print firmware, raw blocks and the decoded on-board profile
ms900ctl --read-json out.json   # save the on-board configuration as a profile
ms900ctl --apply-json p.json    # apply a profile (JSON or .bkm) without the GUI
```

The udev rule (`packaging/70-ms900.rules`) gives the logged-in user access to
the mouse's hidraw node; without it the app reports *Device is
disconnected* / a permission error.  The mouse must not be passed through
to a VM while the app is running.

Profiles are stored in `~/.config/ms900ctl/profiles.json`.

### macOS

The code has no Linux-only dependencies except the "Mouse Parameter"
backend (which is simply skipped), and hidapi supports macOS, so
`cargo build --release` should work; this has not been tested.

## How the protocol was obtained

`OemDrv.exe` was disassembled (see `PROTOCOL.md` for the exact routines),
the original UI was driven in a Windows VM through QMP to screenshot every
dialog (`tools/vm-drive.sh`), and the resulting layout was compared with
libratbag's Sinowealth driver.  `tools/capture-usbmon.sh` +
`tools/decode-capture.py` capture and decode the USB traffic of the Windows
program (requires `modprobe usbmon` as root).  The reports written by
ms900ctl were compared with such a capture and are identical; see the
"Verification status" section of `PROTOCOL.md`.

## Layout of this repository

```
src/model.rs      data model (profiles, key functions, lighting, ...)
src/protocol.rs   HID report encoding / decoding
src/device.rs     hidraw transport (hidapi)
src/store.rs      profiles.json persistence, export / import
src/bkm.rs        importer for the original .bkm backups
src/sysmouse.rs   "Mouse Parameter" desktop backend
src/app/          egui user interface
assets/skins/     artwork of the original software (not MIT licensed)
packaging/        udev rule, .desktop file
tools/            USB capture / VM automation helpers
```
