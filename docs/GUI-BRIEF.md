# fang GUI: design brief

Paste this into Claude Design. Build target: a Tauri desktop app (HTML/CSS/JS frontend,
Rust backend over the `fang` library). Windows 11 first.

## What fang is

A small tool that writes settings to a Razer DeathAdder V3 Pro's onboard memory and then
gets out of the way. Nothing stays running: no tray icon, no background service, no
input hooks. Open it, change settings, Apply, close. The mouse keeps the settings on
any PC. It's the anti-Synapse: fast, quiet, no account, no bloat.

## Look and feel

- Dark by default. Clean, modern desktop app, not gamer-RGB clutter and not a mobile UI.
- One compact window (about 900 x 640), no scrolling for the main view if possible.
- One accent color for the active/selected state. Calm, precise, slightly technical:
  think a well-made audio-interface control panel, not a game launcher.
- Real numbers everywhere (DPI values, Hz, slider levels), never vague labels only.

## Flow

1. On launch, the app reads the mouse. Show a short "Reading mouse..." state.
   - No mouse found: friendly empty state with a Retry button.
2. The form shows what is stored on the mouse right now.
3. The user edits. Changed fields are visibly marked as pending.
4. **Apply** writes only the changed settings. Each write is read back and checked.
   - Success: brief confirmation, pending marks clear.
   - Failure: show which setting failed and the message; other settings stay applied.
5. **Revert** discards pending edits (re-reads the mouse).

## Header

Device name ("DeathAdder V3 Pro"), connection (wireless dongle / wired), battery %
with a charging indicator, firmware version. Small and secondary.

## Sections

### DPI stages
- 1 to 5 stages. Each stage has X and Y DPI. A "link X/Y" toggle per stage (on by default);
  when off, X and Y are edited separately.
- Range 100 to 30000, step 50. Inputs snap to the step.
- Exactly one stage is active (radio-style). Add / remove stage buttons (min 1, max 5).
- Note: the mouse's DPI button cycles through these stages.

### Polling rate
- Segmented control: 125 / 500 / 1000 Hz.

### Lift-off distance
- Mode toggle: **Preset** or **Asymmetric**.
- Preset: Low / Medium / High (values 1, 2, 3).
- Asymmetric: two sliders with numeric readouts. Lift-off 2 to 26, Landing 1 to 25.
  Landing must stay below lift-off: the UI should prevent an invalid pair.

### Buttons
- A top-down mouse diagram with clickable hotspots: left, right, wheel click, wheel up,
  wheel down, back (side), forward (side), DPI button (bottom).
- Selecting a button opens an editor with an action type:
  - Mouse button (left, right, middle, back, forward, wheel up, wheel down)
  - Keyboard key, with optional modifiers (Ctrl, Shift, Alt, Win). Includes F13 to F24,
    which are the main use case (push-to-talk keys nothing else uses). A "press a key to
    capture" field plus a searchable list for keys a keyboard doesn't have.
  - DPI cycle
  - Hypershift (hold): turns the button into a layer shift
  - Disabled
- Layer switch: **Standard** / **Hypershift**. Hypershift is only meaningful when some
  button is set to Hypershift; otherwise show the layer as unavailable with a hint.
- Each hotspot shows its current binding as a small label (e.g. "F13").

### Footer
- Apply (primary, disabled when nothing is pending), Revert.
- A one-line note: "Settings are saved on the mouse. fang doesn't need to stay open."

## Out of scope for now

Lighting (the mouse has none), macros, multiple profiles (the mouse has one onboard slot),
other mouse models (the library has data for 74, but only this one is tested).
