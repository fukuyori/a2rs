# A2RS Phase 8.5: Strict Disk Sequencer

This drop advances Disk II sequencing from transitional timing toward a stricter cycle-driven model.

## Implemented

- `4 CPU cycles = 1 disk bit`
- `8 bits = 1 nibble byte`
- strict mode advances media position inside `tick_disk_bit()`
- strict mode updates `latch`/`shift_reg` when a full nibble becomes ready
- strict mode keeps `C0EC` reads non-destructive (reads no longer advance media position)
- strict mode emits sync/header detection from the tick path

## Current behavior

- `Safe`: legacy byte-visible semantics
- `Transitional`: tick builds nibble readiness but CPU reads still advance media position
- `Strict`: tick fully advances media position; CPU reads observe the current latch

## Intended next steps

- stricter write timing / splice behavior
- per-track bitstream materialization for `.nib` / `.woz`
- save-state coverage for strict sequencer internals
