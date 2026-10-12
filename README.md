# fang

One-shot config writer for the Razer DeathAdder V3 Pro. Writes DPI, polling rate,
lift-off and button binds to the mouse's onboard memory, then exits. Nothing stays
running: no daemon, no input hook, settings work on any PC.

Protocol knowledge comes from reading OpenRazer and Snakecharmer (ideas only, no code)
plus our own USBPcap captures.

## Use

```
fang status                                  # read everything stored on the mouse
fang bind back key:F13                       # every write is read back and checked
fang dpi 1000x1100,800,1600,3200,6400 active 1
fang polling 1000
fang liftoff 13 12                           # asymmetric lift / land, or 1-3 for a preset
```

`fang` with no arguments prints the full usage. The protocol lives in `src/lib.rs`
(`fang::Mouse`); the CLI and a future GUI are thin layers over it.

## Gotcha

Keep Synapse uninstalled after writing. When it opens it rewrites onboard memory with
its own profile.

## License

AGPL-3.0-only
