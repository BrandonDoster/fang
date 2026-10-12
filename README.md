# fang

One-shot config writer for the Razer DeathAdder V3 Pro. Writes DPI, polling rate,
lift-off and button binds to the mouse's onboard memory, then exits. Nothing stays
running: no daemon, no input hook, settings work on any PC.

Protocol knowledge comes from reading OpenRazer and Snakecharmer (ideas only, no code)
plus our own USBPcap captures.

## Use

```
cargo run -- devices    # list connected Razer HID interfaces
```

## Gotcha

Keep Synapse uninstalled after writing. When it opens it rewrites onboard memory with
its own profile.

## License

AGPL-3.0-only
