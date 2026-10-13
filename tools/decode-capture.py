#!/usr/bin/env python3
"""Print the HID feature reports (SET/GET report 4 and 5) found in a usbmon
capture of the Windows MS900 software, decoded against PROTOCOL.md.

usage: decode-capture.py capture.pcapng
needs: tshark
"""
import json
import subprocess
import sys

CFG_FIELDS = [
    (9, 1, "sensor"), (10, 1, "rate/flags"), (11, 1, "dpi count|active"),
    (12, 1, "dpi disabled"), (13, 16, "dpi values"), (29, 24, "dpi colours"),
    (53, 1, "effect"), (54, 1, "streaming mode"), (55, 1, "streaming dir"),
    (56, 1, "steady mode"), (57, 3, "steady colour"), (60, 1, "breathing mode"),
    (61, 1, "breathing count"), (62, 21, "breathing colours"), (83, 1, "tail mode"),
    (84, 1, "neon mode"), (85, 1, "colorful steady mode"), (86, 18, "colorful steady colours"),
    (116, 1, "flicker mode"), (117, 6, "flicker colours"), (123, 1, "stars mode"),
    (124, 1, "wave mode"), (125, 1, "reaction mode"), (126, 3, "reaction colour"),
    (129, 1, "lod"), (130, 1, "led off"),
]


def main(path):
    fields = ["frame.number", "usb.src", "usb.dst", "usb.setup.bRequest", "usb.setup.wValue",
              "usb.data_fragment", "usbhid.data", "usb.capdata"]
    cmd = ["tshark", "-r", path, "-Y", "usb.transfer_type == 0x02 && usb.setup.bRequest",
           "-T", "json"] + sum([["-e", f] for f in fields], [])
    out = subprocess.run(cmd, capture_output=True, text=True, check=True).stdout
    for pkt in json.loads(out):
        layers = pkt["_source"]["layers"]
        req = layers.get("usb.setup.bRequest", [""])[0]
        wvalue = int(layers.get("usb.setup.wValue", ["0"])[0], 0)
        data = (layers.get("usbhid.data") or layers.get("usb.data_fragment") or layers.get("usb.capdata") or [""])[0]
        if not data:
            continue
        b = bytes.fromhex(data.replace(":", ""))
        kind = {"9": "SET", "1": "GET"}.get(req, req)
        rid = wvalue & 0xff
        print(f"#{layers['frame.number'][0]} {kind} report {rid} len {len(b)}: {b[:16].hex(' ')}{' ...' if len(b) > 16 else ''}")
        if rid == 4 and len(b) > 130 and b[1] in (0x11, 0x21, 0x31):
            for off, n, name in CFG_FIELDS:
                print(f"    +{off:<4}{name:<26}{b[off:off+n].hex(' ')}")
        elif rid == 4 and b[1] in (0x12, 0x22, 0x32):
            for i in range(20):
                s = b[8 + i * 4:12 + i * 4]
                if any(s):
                    print(f"    slot {i:2}: {s.hex(' ')}")
        elif rid == 4 and b[1] == 0x30:
            n = b[10]
            print(f"    macro index {b[8]} events {n}: " + " ".join(b[11 + i * 3:14 + i * 3].hex() for i in range(n)))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(1)
    main(sys.argv[1])
