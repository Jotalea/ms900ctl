#!/bin/bash
# Capture the USB traffic between the Windows VM and the MS900 with usbmon.
#
# One-time root setup (the only part that needs sudo):
#   sudo modprobe usbmon
#   sudo setfacl -m u:$USER:r /dev/usbmon*     # or: sudo chmod o+r /dev/usbmon*
#
# Then:  tools/capture-usbmon.sh out.pcapng   (Ctrl-C to stop)
# and decode with: tools/decode-capture.py out.pcapng
set -euo pipefail
OUT=${1:-ms900.pcapng}
BUS=$(for d in /sys/bus/usb/devices/*; do
        [ -f "$d/idVendor" ] && [ "$(cat "$d/idVendor")" = 258a ] && [ "$(cat "$d/idProduct")" = 0029 ] && cat "$d/busnum"; done | head -1)
if [ -z "$BUS" ]; then echo "MS900 not found on any USB bus" >&2; exit 1; fi
if [ ! -r /dev/usbmon"$BUS" ]; then
  echo "/dev/usbmon$BUS is not readable. Run:" >&2
  echo "  sudo modprobe usbmon && sudo setfacl -m u:$USER:r /dev/usbmon*" >&2
  exit 1
fi
echo "capturing bus $BUS to $OUT (Ctrl-C to stop)"
exec dumpcap -i usbmon"$BUS" -w "$OUT"
