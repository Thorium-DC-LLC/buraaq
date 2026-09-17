# Blink the Pico W LED from Buraaq.

```powershell
buraaq run      # host stub prints LED on/off
buraaq flash    # hold BOOTSEL, plug USB — onboard LED blinks
```

Source is only `src/main.bq` (`use std.led`). No `extern`, no CMake in the project.
`buraaq flash` uses the toolchain board pack for `pico_w`.
