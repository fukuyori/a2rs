# A2RS Phase 5 Completed Floating Bus

This update completes the practical floating-bus model used by A2RS.

## What changed

- The floating bus is now driven only during active video-byte fetch cycles.
- During HBL and VBL, the last fetched display byte remains latched on the bus.
- The bus source is selected from the active display mode:
  - text
  - mixed text bottom area
  - lores
  - hires
- 80-column text alternates aux/main memory sampling by display fetch column.

## Effect

This moves the implementation from a scanline-position approximation to a
fetch-phase-aware bus latch model, which is the important missing piece for
Phase 5 in the current A2RS architecture.
