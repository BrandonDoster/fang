# DeathAdder V3 Pro protocol notes

Source of every row is listed. `cap:synapseInstall` = `captures/synapseInstall.pcapng`
(Synapse first connect after install, no user changes yet), decoded with
`uv run tools/razer_reports.py <capture>`.

Levels: **guess** (ideas from OpenRazer/Snakecharmer), **synapse** (seen Synapse send it),
**hw** (fang sent it and the mouse behaved).

## Report

90-byte HID feature report, report id 0, over the dongle (1532:00B7) interface 0.
Synapse sends SET_REPORT, then GET_REPORT for the reply.

| byte | field |
|---|---|
| 0 | status: host sends 00; mouse replies 02 ok, 03 fail/unsupported |
| 1 | transaction id. Synapse counts 00..1E and wraps; mouse echoes it |
| 5 | args size |
| 6 | command class |
| 7 | command id (high bit set = read) |
| 8..87 | args |
| 88 | CRC: XOR of bytes 2..87 |

Level: synapse. Transaction id: any value seems accepted (echoed), not fixed 0x1F.

## Commands seen

| cls/id | what | args / reply | level |
|---|---|---|---|
| 00/82 | serial | reply ASCII `XXXXXXXXXXXXXXX` | synapse |
| 00/04 | set device mode | `03 00` = driver mode (Synapse takes over) | synapse |
| 00/84 | get device mode | | synapse |
| 00/bf, 00/c5 | dongle pairing info | reply contains PID `00 b7` | synapse |
| 04/85 | get current DPI | `01` -> `01 XXXX YYYY 00 00` (BE) | synapse |
| 04/86 | get DPI stages | reply `01 <active> <count>` then per stage `<id 1-based> X X Y Y 00 00` | synapse |
| 04/06 | set DPI stages | `01 <active> <count>` then per stage `<id 0-based> XXXX YYYY 00 00`. `01` = persistent (VARSTORE) | synapse |
| 07/80 | battery | reply `00 ff` = full | synapse |
| 07/84 | charging | reply `00 01` | synapse |
| 07/83 | idle timeout | reply `01 2c` = 300 s | synapse |
| 07/01, 07/81 | set/get low-battery threshold (?) | `4d` | synapse |
| 0f/80, 0f/84 | RGB matrix / brightness | status 03: no RGB on this mouse | synapse |
| 02/8c | read button bind | `01 <button> <layer>` -> `01 <button> 00 <cat> <len> <value...>` | synapse |
| 02/0c | write button bind | `01 <button> 01 <cat> <len> <value...>` | synapse |
| 00/86, 00/87, 00/8e, 00/93, 05/80, 05/81, 05/8a | unknown | | synapse |

Factory DPI stages: 400, 800, 1600 (active), 3200, 6400.

## Button binds

Buttons: 01 left, 02 right, 03 wheel, 04 back, 05 forward, 09 scroll up, 0a scroll down,
60 bottom DPI button.

Categories: 01 mouse (len 01, value = button), 06 DPI (`06 01 06` = cycle). Keyboard is
02 per Snakecharmer (len 02, `<modifier> <HID usage>`): guess until captured.

Factory layer 0 is identity (left=left ... DPI button=DPI cycle). Layer 1 (Hypershift?)
reads empty.

Open: byte 2 of the write is `01`, while reads ask for layer `00`/`01` and reply `00`.
Unclear whether that byte is the layer or a flag. Capture an actual bind change to settle it.

## Polling and lift-off

Not identified yet. Candidates among the unknown ids above.
