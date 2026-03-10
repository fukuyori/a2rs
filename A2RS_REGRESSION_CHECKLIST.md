# A2RS Regression Checklist (v0.4.2)

Use this checklist before advancing beyond the current stable baseline.

## Timing
- [ ] x1 speed reports ~1.023 MHz in TIMING logs
- [ ] MAX speed remains uncapped and switches cleanly back to x1
- [ ] timing log window resets correctly after speed changes

## Disk / Boot
- [ ] DOS 3.3 boot completes
- [ ] ProDOS boot completes
- [ ] `Lode_Runner.dsk` boots and reaches gameplay/menu
- [ ] RWTS session start/end is logged
- [ ] FastDisk remains active without breaking x1 timing

## Video / Floating Bus
- [ ] text mode renders correctly
- [ ] hi-res mode renders correctly
- [ ] mixed mode switches correctly
- [ ] floating bus soft-switch reads do not break boot or UI

## Input / Runtime
- [ ] keyboard input works after boot
- [ ] gamepad detection does not stall the emulator
- [ ] sound remains stable when switching speed modes

## Save / Config
- [ ] config file is read but not written back at runtime
- [ ] ROM / DISK directory overrides in config are respected
