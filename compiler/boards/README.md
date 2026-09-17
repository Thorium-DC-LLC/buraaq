# Board packs for `buraaq flash`.
#
# Each folder is a pico-sdk (or similar) project that implements the `std.led`
# C ABI (`buraaq_led_on`, `buraaq_led_off`, `buraaq_led_wait_ms`, …).
#
# Programmers never touch these files — they write `use std.led` and run
# `buraaq flash`.
