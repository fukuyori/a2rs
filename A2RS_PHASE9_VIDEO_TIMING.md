# A2RS Phase 9-A: Video Timing

## Scope

This phase intentionally excludes NTSC artifact color simulation.
The goal is to make the internal video beam timing deterministic and to
align floating-bus sampling with the current tick-driven beam position.

## Model

- `system.tick() = 1 CPU cycle`
- `VideoScanner::tick() = 1 CPU cycle`
- `65 CPU cycles = 1 scanline`
- `262 scanlines = 1 frame`
- visible region: `192 scanlines x 40 byte fetch columns`
- horizontal blank: remaining `25 cycles` per scanline
- vertical blank: scanlines `192..=261`

## Added runtime state

`VideoScanner` now exposes `VideoPosition`:

- frame
- scanline
- scanline_cycle
- hblank
- vblank
- visible_row
- visible_column
- fetch_phase

`Memory` now mirrors the scanner state via:

- `scanline`
- `scanline_cycle`
- `frame_counter`
- `hblank`
- `vblank`

## Floating bus

`sample_floating_bus()` now samples from the current `VideoPosition`.
The floating bus is only updated during active visible byte fetches.
During HBL/VBL, the last latched value remains visible.

## Soft switches

`$C019 (RDVBL)` now uses the mirrored `memory.vblank` state instead of
re-deriving the state from the scanline ad hoc.

## Public API

`Apple2::video_position()` returns the current tick-driven beam state.
This is intended for:

- floating-bus debug
- timing overlays
- future raster effects
- later NTSC work, if ever needed

## Intentional non-goals for this phase

- NTSC artifact color rendering
- composite blur / chroma simulation
- sub-cycle colorburst phase modeling

This phase is only the timing foundation.
