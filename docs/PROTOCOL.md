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
| 00/bf | multi-device wireless status | count, then status + PID per device | synapse |
| 00/c5 | primary device | type + PID | synapse |
| 04/85 | get current DPI | `01` -> `01 XXXX YYYY 00 00` (BE) | synapse |
| 04/86 | get DPI stages | reply `01 <active> <count>` then per stage `<id 1-based> X X Y Y ZZ ZZ` | synapse |
| 04/06 | set DPI stages | `<profile> <active> <count>` then per stage `<id 0-based> XXXX YYYY ZZZZ` (Z-DPI, 0) | synapse |
| 07/80 | battery | reply `00 ff` = full | synapse |
| 07/84 | charging | reply `00 01` | synapse |
| 07/83 | idle timeout | reply `01 2c` = 300 s | synapse |
| 07/01, 07/81 | set/get battery critical level | ceil(pct * 255 / 100); `4d` = 30% | synapse |
| 0f/80, 0f/84 | RGB matrix / brightness | status 03: no RGB on this mouse | synapse |
| 02/8c | read button bind | `01 <button> <layer>` -> `01 <button> <layer> <fn> <len> <data...>` | synapse |
| 02/0c | write button bind | `<profile> <button> <layer> <fn> <len> <data...>` | synapse |
| 00/0e | set polling rate | `<profile> <rate>`: 01=1000, 02=500, 03=333, 04=250, 05=200, 08=125, 0a=100 | synapse |
| 00/8e | get polling rate | `01 00` -> `01 <rate>` | synapse |
| 0b/03 | set lift-off mode | `00 04 <mode>`: 00 preset, 01 asymmetric. `00 04` = calibration class 0, sensor 4 | synapse |
| 0b/0b | lift-off preset / mode param | preset: `00 04 01 <lvl>` (00 low, 01 medium, 02 high). asymmetric: `00 04 04 00` | synapse |
| 0b/05 | asymmetric lift / land distance | `00 04 <lift> <land>` raw units, land < lift | synapse |
| 00/86, 00/87, 00/93 | edition info, extended fw, dock fw | | synapse |
| 05/80, 05/81, 05/8a | profile count, profile ids, max profiles | | synapse |

First arg `01` on DPI / polling / bind commands is the profile id (1 onboard slot on this
mouse), not a VARSTORE flag. Synapse never sends 0; what 0 does is unknown. Full Synapse
findings, bind function table and modifier mask: `docs/SYNAPSE.md`.

Factory DPI stages: 400, 800, 1600 (active), 3200, 6400.

DPI: `<active>` is 1-based. X and Y are independent (`cap:settings01` set stage 1 to
X 1000 / Y 1100 = `03 e8 04 4c`). Synapse also sent a single-stage form once
(`01 01 01 00 04e2 04e2`, count 1) before resending all 5; full 5-stage write is enough.

## Button binds

Buttons: 01 left, 02 right, 03 wheel, 04 back, 05 forward, 09 scroll up, 0a scroll down,
60 bottom DPI button.

Functions: 01 mouse (len 01, value = button), 02 keyboard (len 02, `<modifier> <HID usage>`),
06 DPI (`06 01 06` = cycle). Rest in `docs/SYNAPSE.md`.

Byte 2 = layer: 00 normal, 01 Hypershift (synapse source + Synapse log). Synapse's
first-connect rewrite wrote the identity map to Hypershift; user F13/F14 went to normal.

`cap:settings01`: back = F13 `01 04 00 02 02 00 68`, forward = F14 `01 05 00 02 02 00 69`.
Mouse replied status 02.

Factory layer 0 is identity (left=left ... DPI button=DPI cycle). Layer 1 (Hypershift?)
reads empty on a fresh mouse.

Open (hw): reply byte 2 does not always echo the requested layer. `fang status` (request
sends layer at args[2], checked) after Synapse closed: L0 back/forward = F13/F14 with reply
byte 2 `00`; L1 back/forward = mouse 04/05 with `00`; every other button replies `01` on
both layers. Synapse's parser assumes reply byte 2 = requested layer. Settle with a
write + read-back on both layers when writes are allowed.

## Reads confirmed on hardware (`fang status`)

00/84 mode, 00/8e polling, 04/85, 04/86, 02/8c, 07/80, 07/83, 07/84, and lift-off reads
(write id | 0x80, same args): 0b/83 -> `00 04 <mode>`, 0b/8b -> `00 04 04 00`,
0b/85 -> `00 04 04 00 <lift> <land>` (note extra `04 00` vs the write).

Device mode reads `00` normal with Synapse closed: driver mode does not stick.

## Writes confirmed on hardware (fang, fixed tid 0x1F, read-back matched)

- 02/0c binds on layer 0: back/forward = mouse 04/05, then F13/F14; left = mouse 01.
- 04/06 DPI: factory 400/800/1600*/3200/6400, then 1000x1100*/800/1600/3200/6400.
- Lift-off: preset low/medium/high, asymmetric 13/12 and 26/25.

Lift-off read-back: 0b/83 replies `00 04 01` in both modes, so it does not show the mode.
0b/8b does: `00 04 01 <tracking-1>` symmetric, `00 04 04 00` asymmetric.

## Hypershift (`cap:settings03` + hw)

- Hypershift key: fn `0c`, len 01, data `01`. Synapse writes it on **both** layers for the
  same button (`01 04 00 0c 01 01` and `01 04 01 0c 01 01`), so the button still acts as
  Hypershift while the layer is active. fang should do the same.
- Hypershift bind: same 02/0c with layer `01`, e.g. `01 05 01 02 02 00 53` (forward = NumLock).
- Synapse resends every non-default bind on each change.

Bind reads (settled, hw): the read's layer argument selects the layer, and the values are
per layer (fang wrote layer 1 forward = F15, layer 0 forward stayed F14). Reply byte 2 is
NOT the requested layer: it is the layer of the last write to that button, on both reads.
Ignore it; compare fn/len/data only. Synapse ignores it too.

## Lift-off

Asymmetric: raw byte = Synapse slider value - 1 (`cap:settings02`: lift 2/13/26 =
`01`/`0c`/`19`, land 1/12/25 = `00`/`0b`/`18`). Lift slider 2..26 (raw 01..19), land 1..25
(raw 00..18), land < lift.
