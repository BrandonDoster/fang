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
| 00/0e | set polling rate | `01 <rate>`: 01 = 1000 Hz, 02 = 500 Hz | synapse |
| 00/8e | get polling rate | `01 00` -> `01 <rate>` | synapse |
| 0b/03 | set lift-off mode | `00 04 <mode>`: 00 preset, 01 asymmetric | synapse |
| 0b/0b | lift-off preset / mode param | preset: `00 04 01 <lvl>` (00 low, 01 medium, 02 high). asymmetric: `00 04 04 00` | synapse |
| 0b/05 | asymmetric lift / land distance | `00 04 <lift> <land>` raw units, land < lift | synapse |
| 00/86, 00/87, 00/93, 05/80, 05/81, 05/8a | unknown | | synapse |

Factory DPI stages: 400, 800, 1600 (active), 3200, 6400.

DPI: `<active>` is 1-based. X and Y are independent (`cap:settings01` set stage 1 to
X 1000 / Y 1100 = `03 e8 04 4c`). Synapse also sent a single-stage form once
(`01 01 01 00 04e2 04e2`, count 1) before resending all 5; full 5-stage write is enough.

## Button binds

Buttons: 01 left, 02 right, 03 wheel, 04 back, 05 forward, 09 scroll up, 0a scroll down,
60 bottom DPI button.

Categories: 01 mouse (len 01, value = button), 02 keyboard (len 02, `<modifier> <HID usage>`),
06 DPI (`06 01 06` = cycle).

`cap:settings01`: back = F13 `01 04 00 02 02 00 68`, forward = F14 `01 05 00 02 02 00 69`.
Mouse replied status 02.

Factory layer 0 is identity (left=left ... DPI button=DPI cycle). Layer 1 (Hypershift?)
reads empty on a fresh mouse.

Open: byte 2. Synapse's first-connect default rewrite used `01`; after that both read
layers 00 and 01 return the bind with `01`. The user binds in `cap:settings01` used `00`.
fang copies Synapse: `00` for user binds. Settle with a read-back after fang writes.

`fang status` after Synapse closed (hw): L0 back/forward = F13/F14 with byte 2 `00`;
L1 back/forward = plain mouse 04/05 with `00`; every other button reads `01` on both layers.
Hypothesis: `01` = layer 1 mirrors layer 0, `00` = layer 0 only (layer 1 falls back to
default). Check against Synapse source in the REA pass.

## Reads confirmed on hardware (`fang status`)

00/84 mode, 00/8e polling, 04/85, 04/86, 02/8c, 07/80, 07/83, 07/84, and lift-off reads
(write id | 0x80, same args): 0b/83 -> `00 04 <mode>`, 0b/8b -> `00 04 04 00`,
0b/85 -> `00 04 04 00 <lift> <land>` (note extra `04 00` vs the write).

Device mode reads `00` normal with Synapse closed: driver mode does not stick.

## Lift-off

Asymmetric: raw byte = Synapse slider value - 1 (`cap:settings02`: lift 2/13/26 =
`01`/`0c`/`19`, land 1/12/25 = `00`/`0b`/`18`). Lift slider 2..26 (raw 01..19), land 1..25
(raw 00..18), land < lift.
