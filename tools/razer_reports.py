"""Print Razer 90-byte feature reports from a USBPcap capture.

usage: uv run tools/razer_reports.py <capture.pcapng> [device_address]
SET = host->mouse (SET_REPORT), GET = mouse->host (GET_REPORT response).
"""
import json
import subprocess
import sys

TSHARK = r"C:\Program Files\Wireshark\tshark.exe"


def crc(r: bytes) -> int:
    c = 0
    for b in r[2:88]:
        c ^= b
    return c


def main(path: str, addr: str = "4") -> None:
    out = subprocess.run(
        [TSHARK, "-r", path, "-x", "-T", "json",
         "-Y", f"usb.device_address=={addr} && (usb.data_len==90 || usb.data_len==98)"],
        capture_output=True, check=True,
    ).stdout
    for pkt in json.loads(out):
        layers = pkt["_source"]["layers"]
        raw = bytes.fromhex(layers["frame_raw"][0])
        r = raw[-90:]
        kind = "SET" if layers["usb"]["usb.src"] == "host" else "GET"
        t = float(layers["frame"]["frame.time_relative"])
        size = r[5]
        args = r[8:8 + min(size, 80)].hex(" ")
        ok = "" if r[88] == crc(r) else " BADCRC"
        print(f"{layers['frame']['frame.number']:>5} {t:8.3f} {kind} st={r[0]:02x} tid={r[1]:02x} "
              f"cls={r[6]:02x} id={r[7]:02x} sz={size:02x} args={args}{ok}")


if __name__ == "__main__":
    main(*sys.argv[1:])
