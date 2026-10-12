# Synapse 4 protocol facts

Facts pulled from Razer Synapse 4.0.827. This file has tables only. No Synapse code is copied
here. Read it alongside `PROTOCOL.md`, which has the USB captures.

## Where the data lives

`app.asar` is only the Electron shell: HID transport, USB detection and windows. It has no device
tables and no command builders. All of the device logic is in a web module that Synapse downloads
per product:

- `https://apps.razer.com/synapse/products/182/mw/…` is the middleware: command builders and
  device classes. Product 182 = 0x00B6.
- `https://apps.razer.com/synapse/products/182/ui/…` is the UI. It holds its own copy of the
  device info.

Synapse caches both in Chromium CacheStorage:
`%LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Default\Service Worker\CacheStorage`. I dumped that
cache read-only into `$TEMP` and ran it through a JS beautifier. Nothing was executed.

§1 covers product 182 in depth. §4 and `docs/devices.json` cover all 74 mice Synapse can load.

Evidence keys used below:

| key | cached URL (hash suffix as of 2026-10-11) | anchor |
|---|---|---|
| `core` | `products/182/mw/6259.98d162c59be4fff20fa2.js` | shared rzDevice25 command library. Cite by the command description string, e.g. `"Set Single Button Assignment"` |
| `mwmain` | `products/182/mw/main.7d7bac778fbfcdb02c10.js` | `DeviceInfo` object (`productId: 182`), feature injector (`calibrationInterfaces`) |
| `uimain` | `products/182/ui/static/js/main.db20a7c4.js` | UI `DeviceInfo`, generic mouse `fwID` table |
| `orca5` | `products/182/mw/orca5CalibrationEngineManager.*.js` | lift-off engine |
| `log` | `User Data\Logs\products_182_mw {D86431B6-…}.log` | Synapse's own runtime log, read passively (not my execution) |

Confidence levels:

- **read**: read directly in the code.
- **read+log**: read in the code and confirmed by the runtime log or a capture.
- **inferred**: deduced; the reasoning is given in the row.

## 1. Device table

Only the DeathAdder V3 Pro (DAV3P) is available locally.

| field | value | evidence | conf |
|---|---|---|---|
| name | Razer DeathAdder V3 Pro (editions 0, 128 White, 129–132 incl. Fortnite) | `mwmain` `deviceName`, `products/182/mw/manifest.json` `editionId` | read |
| PIDs | wired `00B6` (`productId: 182`), dongle `00B7` (`dongleId: 183`), no BLE id | `mwmain` / `uimain` DeviceInfo | read+log |
| VID | 1532 | DeviceInfo `vendorId: 5426` | read |
| device class | `rzDevice25DualLinkMouse`; report length 90 + report id; interface 0 | `core` `this.name = "rzDevice25DualLinkMouse"`, `log` "claimInterface:0" | read+log |
| DPI | min 100, max 30000, step 50, separate X/Y | DeviceInfo `minDPI`, `maxDPI: 3e4`, `dpiStep: 50`, `supportXYDPI` | read |
| polling (UI list) | 125, 500, 1000 | `mwmain` / `uimain` list `{content:"125"}…{content:"1000"}` | read |
| HyperPolling | `highSpeedPollingRate: yes` is registered, but without `isHyperpollingDevice`. Synapse switches to 00/40 only when the mouse is on a HyperPolling dongle (dual-link connected and "hyperPollingMaster"). On the stock 00B7 dongle it uses 00/0e, and wireless is clamped to 1000 | `mwmain` `useFeature("highSpeedPollingRate","yes")`, `core` `isHyperpollingDevice` / `hyperPollingMaster` | read (corrected: the first pass missed the `useFeature` calls) |
| onboard memory | `isOBMDevice: true`, `OBMSlots: 1`, `hasHardwareProfileSwitch: true` | DeviceInfo `OBMSpecs` | read |
| OBM features | DPI, polling, brightness. No macros | `OBMSpecs.supportedFeature` (`macros: !1`) | read |
| bind groups | disable, hypershift, keyboard, mouse, multimedia, sensitivity (DPI_Clutch, DPI_Up, DPI_Down, DPI_CycleUp, DPI_CycleDown). UI also offers repeat-scroll | `OBMSpecs.supportedMappings`, `extendSupportMappings` | read |
| buttons | LeftClick 01, RightClick 02, ScrollButton 03, Button4 04, Button5 05, ScrollUp 09, ScrollDown 0a, DPI button `DKM_SB_03` 60 | default mapping list in `mwmain`, fwIDs from `uimain` table, `log` buttonIds 1,2,3,4,5,9,10,96 | read+log |
| lift-off | `orca5CalibrationEngine` (asymmetric cut-off, "up to 26 levels"). Defaults: trackingDistance 2, liftOff 13, landing 12 | `log` "Supported flags: [\"orca5CalibrationEngine\"]", `mwmain` `calibration` defaults, `installer-manifest.json` description | read+log |
| low battery warning | 5 % UI toast. The 07/01 threshold write is separate (§3) | `mwmain` `hasLowBatteryValue.config.level: 5` | read |

> Gotcha: `mwmain` has an injector default of `calibrationInterfaces: { smartPixartCalibrationEngine }`.
> The module's own startup then registers `useFeature("calibrationInterfaces","orca5CalibrationEngine")`,
> and the log confirms orca5: "smartPixartCalibrationEngine not in feature flag list! Supported flags:
> [orca5CalibrationEngine]". The orca5 byte sequence is the one that matches `cap:settings02`.
> For every product, read the `useFeature(...)` calls, not the injector defaults.

### Generic mouse fwID table (all mice, `uimain`)

These are the button ids used in 02/0c and 02/8c byte 1. Only some of them exist on a given mouse.

| fwID | input | | fwID | input |
|---|---|---|---|---|
| 01 | LeftClick | | 0b | DKM_M_01 (default DPI up) |
| 02 | RightClick | | 0c | DKM_M_02 (default DPI down) |
| 03 | ScrollButton | | 0e | DKM_SB_01 (default profile) |
| 04 | Button4 | | 0f | DKM_SB_02 (default sensitivity) |
| 05 | Button5 | | 34 / 35 | ScrollLeft / ScrollRight |
| 06 / 07 | DKM_M_03 / DKM_M_04 | | 40–4b | side-pad 1–12 |
| 09 / 0a | ScrollUp / ScrollDown | | 50–56 | HPad 1–7 |
| 60 | DKM_SB_03 (DPI cycle) | | 6a / 6d | DKM_M_106 / DKM_SB_08 (scroll mode) |

Firmware button enum names (`core`, enum containing `HyperShift: 57`): 0b DPIUp, 0c DPIDown,
0e ProfileSwitch, 0f SniperButton, 39 HyperShift, 60 DPI_Cycle_Up, 61 DPI_Cycle_Down,
6a ScrollFree_NotchSwitch.

## 2. Command builders

Framing (`core` `_createDataSend`, `_calculateChecksum`), conf **read**. This matches PROTOCOL.md:

- `[0]` = 0, `[1]` = txn id, `[2..4]` = 0, `[5]` = size, `[6]` = class, `[7]` = id, `[8..]` = args
- `[88]` = XOR of bytes 2..87

Each command constant is `[size, class, id]`. The read form is id | 0x80.

### Transaction id

Conf **read**. Evidence: `core` `_getTransactionId` in `rzDevice25`, `rzDevice25DualLinkMouse`,
`rzDevice25DualLinkKeyboard`, plus `rzDevice25Linker*.js`.

| class | id sequence |
|---|---|
| rzDevice25, rzDevice25DualLinkMouse (DAV3P) | 00,01,…,1e, then back to 00. It is per device object and counts every command |
| rzDevice25DualLinkKeyboard | 0x80 \| (00..1e) |
| rzDevice25Linker* (dongle hubs) | 0xE0 \| (00..1e) |
| DualLinkMouse, 00/41 (pairing mode) and 00/46 (device scan) only | the id is also OR-ed with 0x80 |

**Why it rotates.** In Synapse the id is only used to match replies on the host side. Nothing shows
the firmware checking it.

- Reply matching (`core` `_getUSBTransferInResult`, conf **read**). After SET_REPORT, Synapse waits
  `sleepTimeBetweenOutIn` (25 ms on DAV3P, per `log`). It then does GET_REPORT up to `maxRetryIn`
  times (10), 25 ms apart. It accepts a reply only when all three hold:
  - reply `[1]` == sent `[1]`
  - reply `[6]` == sent class
  - reply `[7]` == sent id

  On any mismatch it logs "different transId" or "different command" and re-sends the **same**
  buffer, same id, up to `maxRetryOut` times (20).
- Status byte `[0]` handling, conf **read** (`core` and shell `protocol25Const.js`):

  | status | name | what Synapse does |
  |---|---|---|
  | 00 | new command, not processed yet | re-send |
  | 01 | busy | GET again |
  | 02 | ok | accept |
  | 03 | failed | stop, fail |
  | 04 | timeout | stop, fail |
  | 05 | not supported | stop, fail |

  PROTOCOL.md's "03 = fail/unsupported" should be split into 03 failed and 05 not supported.
- The Electron shell's transport (`electron/Protocol/protocol25.js`) checks only status and
  class/id (`custom_command_mismatch: 225`). It never looks at byte 1.
- Both layers serialize access: `navigator.locks` per device, and an HID mutex
  (`hid.sendFeatureReportMutex`) shared across processes. So several Synapse tabs/processes can talk
  to one dongle. The rotating id lets a sender reject a reply that belongs to someone else's
  request, or to its own earlier request. **inferred** from the code's structure.
- The high bits differ by device class: 0x80 for the dual-link keyboard, 0xE0 for Linker dongles,
  and 0x80 on the mouse's pairing/scan commands. On shared HyperSpeed dongles the top bits likely
  route the request to the keyboard, the mouse or the dongle itself. **inferred**: the firmware
  side is not visible. A fixed 0x1F keeps the high bits 0, i.e. it targets the mouse, which fits
  fang's working reads.

**02/8c reply byte 2 not echoing the requested layer.** This is firmware behaviour, not a
transaction-id or stale-read effect.

- In `captures/settings03.pcapng` (frames 351/354), Synapse uses rotating ids and gets matching ids
  back. A layer-00 read of "right" still replies with byte 2 = 01.
- On hardware, byte 2 holds the layer of the most recent write to that button, on both layer reads.
  The bind data that comes back is correct for the layer that was asked for. Full write-up:
  PROTOCOL.md, "Hypershift".

Synapse's parser just stores reply byte 2 as `mode` and has no handling for this, conf **read**.
Treat reply byte 2 as "last-written layer", not as an echo of the request.

### Byte 0 of DPI / polling / bind args = profile id

Conf: **read** for the name, **read+log** for the value.

- The code names this argument `classId` (DPI) or `profileId` (polling, buttons).
- For DAV3P, Synapse always passes **1**:
  - the DPI task runner calls `setDPIStages(1, …)` and `getDPIStages(1)`
  - the polling helper defaults `profileId = 1`
  - the log shows `setProfileDPIClassData classId:1` and `setSingleButtonMapping profileId: 1`
- The OBM engine loops over the ids returned by 05/81. DAV3P has `OBMSlots: 1`.
- Synapse never sends **0** for these commands on DAV3P. "0 = live/no-store, 1 = stored slot" is
  the OpenRazer reading (NOSTORE/VARSTORE). It is **inferred** and not confirmed here.
- Class 0b does **not** use this byte for a profile. See lift-off.

### DPI (class 04)

Conf **read+log**.

| cmd | size | args |
|---|---|---|
| 04/06 set stages | 0x50 | `<profile> <active 1-based> <count>`, then per stage 7 bytes: `<stage id> X_hi X_lo Y_hi Y_lo Z_hi Z_lo` |
| 04/86 get stages | 0x50 | send `<profile>`. Reply has the same layout |
| 04/05 set active DPI | 7 | `<profile> X(BE16) Y(BE16) Z(BE16)` |
| 04/85 get active DPI | 7 | send `<profile>` |

- Synapse sends size 7·count + 3 for 04/06, not 0x50.
- The trailing `00 00` in each stage is a **Z DPI** field (`dpiZ`), not padding. Synapse sends 0.
- The DPI button cycles through stages. Whether the stage id is 0- or 1-based is not set by the
  builder; it passes through what the caller gives. Captures show 0-based on write and 1-based on
  read. Keep PROTOCOL.md's rule.
- The DAV3P flag `taskRunnerSetDPI.setDpiLevel` is absent (log), so Synapse sets DPI only through
  04/06. Evidence: `core` "Set Profile DPI Class Data", "Set Profile Active DPI Setting".

### Polling (class 00)

Conf **read**. Evidence: `core`, the enum containing `RATE_8000Hz: 1`, and "Set USB High Speed Polling Period".

| cmd | size | args | used when |
|---|---|---|---|
| 00/05 set polling | 1 | `<code A>` | device is not OBM |
| 00/0e set profile polling | 2 | `<profile> <code A>` | OBM device with `pollingRate` feature (**DAV3P**) |
| 00/40 set high-speed polling | 2 | `<profile> <code B>` | HyperPolling device |

Reads are 00/85, 00/8e and 00/c0, with the same layout.

| Hz | code A | code B |
|---|---|---|
| 8000 | — | 01 |
| 4000 | — | 02 |
| 2000 | — | 04 |
| 1000 | 01 | 08 |
| 500 | 02 | 10 |
| 333 | 03 | — |
| 250 | 04 | 20 |
| 200 | 05 | — |
| 125 | 08 | 40 |
| 100 | 0a | — |

Code B is a one-hot bitmask.

### Device mode (00/04, size 2)

Conf **read+log**.

- Args are `<mode> <param>`. The log shows `setDeviceMode() mode:3, param:0` and `mode:0, param:0`.
- Modes: 00 normal, 01 bootloader, 02 testing, 03 driver, 04 download, 05 pairing, 06 standby,
  08 RF test, 09 retail.
- 00/84 replies `<mode> <param>`.

### Button binds (class 02)

Conf **read+log**. Evidence: `core` "Set Single Button Assignment", the reply parser with
`modeEnum`, the enums `{NormalMode: 0, HypershiftMode: 1}` and `{Off: 0, ButtonCode: 1, …}`, and
`log` "setSingleButtonMapping buttonId: … buttonMode …".

02/0c write, size 0x50 (zero-padded), args:

`<profile> <button fwID> <mode> <fn id> <data len> <data…>`

02/8c read: send `<profile> <button> <mode>`. The reply uses the same layout, with data in bytes 5..9.

**Byte 2 is the layer.** 00 = normal layer, 01 = **Hypershift** layer. This refutes the "01 = layer 1
mirrors layer 0" hypothesis:

- Synapse's first-connect rewrite writes the default identity map with `isHyperShift: true`. The log
  shows `buttonMode 1` for buttons 1,2,3,4,5,9,10,96.
- User binds go to `buttonMode 0`. The log shows F13 on Button4 as
  `buttonMode 0 fnId 2 KeyCode fnDataByte 00 68`.
- So the rewrite fills the Hypershift layer with identity, and user binds change only the normal
  layer. That fits fang's hardware read: L1 back/forward stayed mouse 04/05 after the F13/F14 binds.

Function ids (byte 3) and data layouts:

| fn | name | len | data |
|---|---|---|---|
| 00 | Off (disabled) | 0 | — |
| 01 | ButtonCode | 1 | `<button enum>`: 01 L, 02 R, 03 middle, 04 back, 05 fwd, 09 wheel up, 0a wheel down, 68 tilt L, 69 tilt R |
| 02 | KeyCode | 2 | `<modifier mask> <HID usage>` |
| 03 | MacroTypeI (once / N times) | 3 | `<macro id BE16> <repeat count>` (once = 1) |
| 04 | MacroTypeII (repeat while held) | 2 | `<macro id BE16>` |
| 05 | MacroTypeIII (toggle) | 2 | `<macro id BE16>` |
| 06 | DPI | 1 or 5 | 01 up, 02 down, 06 cycle up, 07 cycle down; clutch = `05 X(BE16) Y(BE16)` (len 5) |
| 07 | Profile | 1 or 2 | 01 up, 02 down, 04 cycle up, 05 cycle down; specific = `03 <profile id>` (len 2) |
| 08 | Lighting | ? | not built for mice |
| 09 | PowerKeys | 1 | 81 power, 82 sleep, 83 wake |
| 0a | MediaKeys | 2 | consumer usage BE16 (see below) |
| 0b | DoubleClick | 1 | `01` |
| 0c | ModeButtonkey = **Hypershift** | 1 | `01` |
| 0d | TurboModeKey | 4 | `<modifier> <HID> <period ms BE16>`, period = floor(1000 / keys per second), default 142 |
| 0e | TurboModeButton | 3 | `<button enum> <period ms BE16>`. Repeat-scroll is this with button 09/0a/68/69 |
| 0f | MacroTypeIV (sequence) | 2 | `<macro id BE16>` |
| 10 | Controller | 1 | gamepad fwID (controllers only) |
| 11 | RazerKey | 1 | Razer key code |
| 12 | ScrollWheelMode | 1 | 01 toggle free-spin; scrolling group 02 up, 03 down, 04 cycle up, 05 cycle down |
| 15 | Win8ShortcutsKey | 1 | 63 screenshot, 64 Xbox Game Bar, 65 cycle apps |

Fn id table: **read**. Mouse/keyboard/DPI layouts: **read+log**. The rest: **read**.

Keyboard modifier mask, conf **read**:

| bit | key |
|---|---|
| 01 | LCtrl |
| 02 | LShift |
| 04 | LAlt |
| 08 | LGUI |
| 10 | RCtrl |
| 20 | RShift |
| 40 | RAlt |
| 80 | RGUI |

If the bound key is itself a modifier (HID E0–E7), Synapse sends `<its mask bit> 00` instead of
`00 <usage>` when "handle N-key rollover" is set.

Media usages (consumer page, BE16), conf **read**:

| usage | function |
|---|---|
| 00b8 | eject |
| 00e2 | mute |
| 00ea | vol down |
| 00e9 | vol up |
| 00b6 | previous |
| 00b7 | stop |
| 00cd | play/pause |
| 00b5 | next |
| 029f | cycle apps |
| 006f | brightness up |
| 0070 | brightness down |
| 0221 | search |

Other class-02 ids, conf **read**:

- 02/0f, 02/8f key assignment list
- 02/8e button assignment list: send `<profile> <mode> <id offset>`; reply `<profile> <mode> <offset> <bytes…>`
- 02/25 button mapping mode

### Lift-off (class 0b, "Proximity Sensor")

Conf **read+log**, from `core` "Set Proximity Sensor State/Lift Setting/Configuration" and `orca5`.
It matches `cap:settings02`.

**`00 04` prefix = `<class id> <sensor id>`:**

- Class id 00 is `SURFACE_CALIBRATION_CLASS_ID`. A "NEW" class id 01 exists for other sensors. It is not a profile.
- Sensor id 04 is `Avago_Proximity_sensor`.
- The other sensor ids: 01 lift detection, 02/03 hand-move sensors, 05 Philips proximity.

| cmd | size | args |
|---|---|---|
| 0b/03 set state | 3 | `00 04 <0/1>` |
| 0b/0b set lift setting | 4 | `00 04 <lift mode> <lift height>` |
| 0b/05 set configuration | 10 | `00 04 <parm0..parm7>` (Synapse sends only the bytes it has) |
| 0b/85 get configuration | 10 | reply `00 04 <parm0..parm8>` |

Lift modes:

| value | mode |
|---|---|
| 01 | Universal_Lift_Cut_off |
| 02 | Universal_Lift_Hysteresis |
| 03 | Preset_Calibration_Lift_Cut_off |
| 04 | Preset_Calibration_Hysteresis |
| 05 | Manual_Calibration_Lift_Cut_off |
| 06 | Manual_Calibration_Hysteresis |

Sequences the orca5 engine sends (DAV3P):

- **Symmetric** (UI "tracking distance" 1..3):
  1. 0b/03 `00 04 00`
  2. 0b/0b `00 04 01 <tracking−1>`
- **Asymmetric**:
  1. 0b/03 `00 04 01`
  2. 0b/0b `00 04 04 00`
  3. 0b/05 `00 04 <liftOff−1> <landing−1>`

So 0b/03's third byte is "asymmetric/hysteresis on". That matches PROTOCOL.md's "00 preset, 01 asymmetric".

The 0b/85 reply's extra `04 00` (PROTOCOL.md) is parm2..: the reply parser reads 9 params after
`00 04`. **inferred**: the firmware stores more fields than Synapse writes.

## 3. Unknown ids from the captures

| id | meaning | reply layout | evidence | conf |
|---|---|---|---|---|
| 00/86 | Get Edition Information | `<keyboard layout> <edition> <firmware id>` | `core` "Get Edition Information" | read |
| 00/87 | Get Extended Firmware Version | `<major> <minor> <internal> <reserved>` | `core` "Get Extended Firmware Version" | read |
| 00/93 | Get Dock Extended Firmware Version | same as 00/87 | `core` | read |
| 00/bf | Get Multiple Device Wireless Connection Status V2 | `<count>` then per device `<status> <PID BE16>` | `core` (DualLink `queryDualLinkInfo`) | read |
| 00/c5 | Get Multiple Device Primary Device | `<type> <PID BE16>`. type picks master/slave | `core` | read |
| 05/80 | Get Number of Profiles | `<n>` | `core` "Get Number of Profiles" | read |
| 05/81 | Get Profiles ID List | `<total> <id…>` | `core` "Get Profiles ID List" | read |
| 05/8a | Get Max Profiles Supported | `<max>` | `core` "Get Max Profiles Supported" | read |
| 07/01 | Set Battery Critical Level | `<ceil(pct·255/100)>`. `4d` = 30 % (default `lowPowerMode: 30`) | `core` "Set Battery Critical Level", `mwmain` `lowPowerMode: 30` | read (value match inferred) |

Related class-07 facts, conf **read**:

- 07/80 Get Battery Level replies `<battery id> <level 0..255>`; percent = floor(level·100/255).
- 07/83 is Get Time to Sleep (BE16 seconds).
- 07/84 is Get Charging Status.

Other class-05 ids:

| id | function |
|---|---|
| 05/02 | create profile |
| 05/03 | delete profile |
| 05/04 | set active profile |
| 05/08 | profile name with offset |
| 05/0b | profile reset |
| 05/0c | profile color |

## 4. All mice Synapse can load

Full facts for every mouse are in `docs/devices.json`, one object per product, each with a `source`
URL. I fetched these on 2026-10-11 with plain sequential GETs from Razer's public CDN and executed
nothing.

How the list was built:

1. **Catalog.** `https://apps.razer.com/synapse/dashboard/AvailableDevices.json` lists 332 products.
   - Each entry has `productId` and optional `dongleId`, `bleId`, `wiredId`, `xBoxId`, `psModeIds`.
   - Synapse's background manager loads it (`DeviceDetector.getLatestDeviceData`).
   - It maps any connected PID to its product id ("repId"). The log shows
     `classified:available:182` for PID 183.
   - Module URL scheme: `https://apps.razer.com/synapse/products/<productId>/{mw,ui}/`.
2. **Mouse filter.** A product counts as a mouse when its `mw/index.html` favicon is `MOUSE.svg`.
   That gives 74 mice. Other icons seen: KEYBOARD, AUDIO, MOUSEMAT, and so on.
3. **Fetch.** Each mouse's `mw/main.<hash>.js` holds three things:
   - the `DeviceInfo` object
   - the polling option list and default DPI/mapping data
   - the `useFeature(...)` registrations: device class, lift-off engine, HyperPolling
4. **HyperSpeed multi-device info** comes from `synapse/dashboard/DualDongleCompatibleDevices.json`
   (cached). It gives the dongle PID and sibling PIDs.
5. **Button fwIDs** come from the generic table in §1. A `null` fwID in the JSON means the input name
   (e.g. `DKM_SB_210`, `DKM_HYPERSHIFT`, `DKM_AI_PROMPT`) is not in that table. The per-product
   `ui/` bundle would have it; I did not fetch those (about 6 MB each).

Facts that hold across all 74, conf **read**:

- Every mouse uses the rzDevice25 command family: plain, Legacy or LegacyNoBrightness. So framing,
  transaction ids (00..1e, wrap), class ids and bind layouts are the ones in §2.
- `isDualLinkDevice: true` means the `rzDevice25DualLinkMouse` class. The id sequence is the same.
- "HyperPolling" in the table is read from the `highSpeedPollingRate` feature:
  - `isHyperpollingDevice` means 00/40 on a wired or direct connection.
  - `isHyperpollingWireless` means 00/40 over the mouse's own dongle.
  - Bare `yes` means 00/40 only on a separate HyperPolling dongle.
  - Otherwise Synapse uses 00/0e.
- The Viper 8KHz (0091) UI offers 5000 Hz, which is not in the 00/40 code table. Its encoding is unknown.
- DPI step `1` (several newer mice) is what the module declares; it is not a parsing error.
- Mice without a default-mapping array in mw main (e.g. Deathadder Elite 005c, Abyssus) have
  `buttons: null`.
- `OBMSlots` is missing on some older mice (legacy class). Treat them as no onboard profiles.
- Lift-off engines:

  | engine | sequence |
  |---|---|
  | orca5 | §2 sequence |
  | orca5+ | same 0b ids. Symmetric sends state **01**, not 00, before the 0b/0b (`orca5PlusCalibrationEngine`) |
  | smartPixart, pixart, pixart3349 (class id **01**), philips | other 0b sequences, not decoded here |

| PID | name | dongle | BLE | DPI max / step | polling max | HyperPolling | OBM slots | Hypershift | lift-off engine |
|---|---|---|---|---|---|---|---|---|---|
| 0046 | Razer Mamba Elite | - | - | 16000 / 50 | 1000 | - | 0 | - | philips |
| 0050 | Razer Naga Hex V2 | - | - | 16000 / 1 | 1000 | - | - | - | philips |
| 0053 | Razer Naga Chroma | - | - | 16000 / 1 | 1000 | - | - | - | philips |
| 0059 | Razer Lancehead | 005a | - | 16000 / 50 | 1000 | - | 4 | yes | philips |
| 005c | Razer Deathadder Elite | - | - | 16000 / 50 | 1000 | - | - | - | pixart |
| 0060 | Razer Lancehead TE | - | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| 0062 | Razer Atheris | 0062 | 0061 | 7200 / 200 | 1000 | - | 1 | yes | pixart |
| 0063 | Razer Jugan | - | - | 7200 / 100 | - | - | - | - | - |
| 0064 | Razer Basilisk | - | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| 0065 | Razer Basilisk Essential | - | - | 6400 / 100 | 1000 | - | 1 | yes | pixart |
| 0067 | RAZER NAGA TRINITY | - | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| 006a | D.VA Razer Abyssus Elite | - | - | 7200 / 100 | - | - | - | - | pixart |
| 006b | Razer Abyssus Essential | - | - | 7200 / 100 | - | - | - | - | pixart |
| 006c | Razer Mamba Elite | - | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| 006e | Razer DeathAdder Essential | - | - | 6400 / 100 | 1000 | - | - | - | - |
| 0070 | Razer Lancehead Wireless | 006f | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| 0071 | Razer DeathAdder Essential | - | - | 6400 / 100 | 1000 | - | - | - | - |
| 0073 | Razer Mamaba Wireless | 0072 | - | 16000 / 100 | 1000 | - | 4 | yes | smartPixart |
| 0074 | Razer Abyssus Lite | - | - | 6400 / 100 | 1000 | - | - | - | - |
| 0075 | Razer Turret Mouse Xbox One Edition | 0904 | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| 0078 | Razer Viper | - | - | 16000 / 50 | 1000 | - | 1 | yes | pixart |
| 007a | Razer Viper Ultimate | 007b | - | 20000 / 50 | 1000 | - | 4 | yes | - |
| 007c | Razer DeathAdder V2 Pro | 007d | 008e | 20000 / 50 | 1000 | - | 4 | yes | smartPixart |
| 0080 | Razer Pro Click | 0077 | 0076 | 16000 / 50 | - | - | 1 | yes | - |
| 0083 | Razer Basilisk X Hyperspeed | 0083 | 0082 | 16000 / 50 | 1000 | - | 1 | yes | smartPixart |
| 0084 | RAZER DEATHADDER V2 | - | - | 20000 / 50 | 1000 | - | 4 | yes | - |
| 0085 | Razer Basilisk V2 | - | - | 20000 / 50 | - | - | 4 | yes | - |
| 0086 | Razer Basilisk Ultimate | 0088 | - | 20000 / 50 | 1000 | - | 4 | yes | - |
| 008a | Razer Viper Mini | - | - | 8500 / 100 | 1000 | - | 1 | yes | pixart |
| 008c | Razer Deathadder V2 Mini | - | - | 8500 / 100 | 1000 | - | 1 | yes | pixart |
| 008d | RAZER NAGA LEFT-HANDED EDITION | - | - | 20000 / 100 | 1000 | - | 4 | yes | - |
| 008f | RAZER NAGA PRO | 0090 | 0092 | 20000 / 50 | 1000 | - | 4 | yes | smartPixart |
| 0091 | Razer Viper 8Khz | - | - | 20000 / 50 | 8000 | wired | 4 | yes | - |
| 0093 | Razer Naga Classic Edition | - | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| 0094 | Razer Orochi V2 | 0094 | 0095 | 18000 / 50 | 1000 | - | 1 | yes | smartPixart |
| 0096 | RAZER NAGA X | - | - | 16000 / 50 | 1000 | - | 1 | yes | pixart |
| 0098 | Razer DeathAdder Essential | - | - | 6400 / 100 | 1000 | - | 1 | yes | - |
| 0099 | Razer Basilisk V3 | - | - | 26000 / 50 | 1000 | - | 4 | yes | smartPixart |
| 009a | Razer Pro Click Mini | 009a | 009b | 12000 / 50 | - | - | 1 | yes | - |
| 009c | Razer DeathAdder V2 X Hyperspeed | 009c | 009d | 14000 / 100 | 1000 | - | - | yes | smartPixart |
| 009e | Razer Viper Mini Signature Edition | 009f | - | 30000 / 50 | 1000 | wireless | 1 | yes | orca5+ |
| 00a1 | Razer Deathadder V2 Lite | - | - | 8500 / 100 | 1000 | - | 1 | yes | pixart |
| 00a3 | Razer Cobra | - | - | 8500 / 100 | 1000 | - | 1 | yes | pixart3349 |
| 00a5 | RAZER VIPER V2 PRO | 00a6 | - | 30000 / 50 | 1000 | via HP dongle | 1 | yes | orca5 |
| 00a7 | RAZER NAGA V2 PRO | 00a8 | 00a9 | 30000 / 50 | 1000 | - | 4 | yes | orca5 |
| 00aa | Razer Basilisk V3 Pro | 00ab | 00ac | 30000 / 50 | 1000 | via HP dongle | 4 | yes | orca5+ |
| 00af | Razer Cobra Pro | 00b0 | 00b1 | 30000 / 50 | 1000 | via HP dongle | 4 | yes | orca5+ |
| 00b2 | Razer DeathAdder V3 | - | - | 30000 / 50 | 1000 | wired | 4 | yes | orca5 |
| 00b4 | Razer Naga V2 Hyperspeed | 00b4 | 00b5 | 30000 / 50 | 1000 | - | 1 | yes | orca5 |
| 00b6 | Razer DeathAdder V3 Pro | 00b7 | - | 30000 / 50 | 1000 | via HP dongle | 1 | yes | orca5 |
| 00b8 | Razer Viper V3 HyperSpeed | 00b8 | - | 30000 / 50 | 1000 | via HP dongle | 1 | yes | orca5 |
| 00b9 | Razer Basilisk V3 X Hyperspeed | 00b9 | 00ba | 18000 / 50 | 1000 | - | 1 | yes | pixart3349 |
| 00be | Razer DeathAdder V4 Pro | 00bf | - | 45000 / 50 | 8000 | wired+wireless | 1 | yes | orca5+ |
| 00c0 | Razer Viper V3 Pro | 00c1 | - | 35000 / 50 | 1000 | wireless | 1 | yes | orca5+ |
| 00c2 | Razer DeathAdder V3 Pro Hyperpolling Technology | 00c3 | - | 30000 / 50 | 1000 | wireless | 1 | yes | orca5 |
| 00c4 | Razer DeathAdder V3 HyperSpeed | 00c5 | - | 20000 / 50 | 1000 | via HP dongle | 1 | yes | orca5+ |
| 00c7 | Razer Pro Click V2 Vertical Edition | 00c8 | 00c9 | 30000 / 50 | - | - | 1 | yes | - |
| 00cb | Razer Basilisk V3 35K | - | - | 35000 / 1 | 1000 | - | 4 | yes | orca5+ |
| 00cc | Razer Basilisk V3 Pro 35K | 00cd | 00ce | 35000 / 50 | 1000 | via HP dongle | 4 | yes | orca5+ |
| 00d0 | Razer Pro Click V2 | 00d1 | 00d2 | 30000 / 50 | - | - | 1 | yes | - |
| 00d3 | Razer Basilisk Mobile | 00d4 | 00d5 | 18000 / 50 | 1000 | - | 1 | yes | pixart3349 |
| 00d6 | Razer Basilisk V3 Pro 35K Phantom Green Edition | 00d7 | 00d8 | 35000 / 50 | 1000 | via HP dongle | 4 | yes | orca5+ |
| 00da | Razer Cobra HyperSpeed | 00db | 00dc | 26000 / 50 | 1000 | via HP dongle | 4 | yes | orca5 |
| 00dd | Razer Krakoff | 00dd | - | 45000 / 1 | 8000 | wireless | 1 | yes | orca5+ |
| 00de | Razer Viper V3 Pro SE | 00df | - | 35000 / 1 | 1000 | via HP dongle | 1 | yes | orca5+ |
| 00e0 | Razer Orochi V2 | 00e0 | 00e1 | 18000 / 50 | 1000 | - | 1 | yes | - |
| 00e2 | Razer Basilisk V4 Pro | 00e3 | 00e4 | 50000 / 1 | 8000 | wired+wireless | 4 | yes | orca5+ |
| 00e5 | Razer Viper V4 Pro | 00e6 | - | 50000 / 50 | 8000 | wired+wireless | 1 | yes | orca5+ |
| 00e7 | RAZER NAGA V3 PRO | 00e8 | 00e9 | 30000 / 50 | 1000 | via HP dongle | 4 | yes | orca5+ |
| 00eb | Razer Basilisk V4 HyperSpeed | 00ea | 00ec | 33000 / 1 | 8000 | wired | 4 | yes | orca5+ |
| 00ef | Razer DeathAdder V4 Pro Carbon Fiber Edition | 00f0 | - | 50000 / 50 | 8000 | wired+wireless | 1 | yes | orca5+ |
| 0a22 | Razer Turret Mouse Gears Of War 5 Edition | 0a21 | - | 16000 / 50 | 1000 | - | 4 | yes | pixart |
| b005 | Razer Viper V4 HyperSpeed | b005 | b006 | 33000 / 50 | 8000 | wireless | 1 | yes | orca5+ |
| b00a | Razer Basilisk V3 Pro 35K - Xbox 25 Anniversary Edition | b00b | b00c | 35000 / 50 | 1000 | via HP dongle | 4 | yes | orca5+ |

## Conflicts with PROTOCOL.md

1. **02/0c byte 2** is the layer selector (00 normal, 01 Hypershift), not "mirror". fang's
   "use 00 for user binds" is still correct.

   Reply byte 2 of 02/8c is the layer of the last write to that button, not the requested layer.
   Captures and hardware both show this; see §2 and PROTOCOL.md "Hypershift". No conflict.
2. **04/06 stage tail `00 00`** is a Z-DPI field, not padding. The bytes on the wire are the same.
3. **0b `00 04`** is class id 00 + sensor id 04. It is not a VARSTORE flag.
4. **07/01** is the battery critical level, a write (`4d` = 30 %). It is not a read of the
   low-battery threshold.
5. **00/bf** is multi-device wireless status for the dongle; it lists the PIDs present. 00/c5 is
   the primary device. PROTOCOL.md's "dongle pairing info" is roughly right.

## Still unknown

- Per-product `ui/` bundles were not fetched, so a few button fwIDs are `null` and the lift-off ranges of non-orca engines are unknown.
- Whether profile id 0 means "live, not stored" on DAV3P firmware. Synapse never sends it.
- Lighting (fn 08) bind layout for mice, and the full Razer-key code table (fn 11).
- The PID of a HyperPolling dongle for DAV3P. The bundle only checks a feature flag.
