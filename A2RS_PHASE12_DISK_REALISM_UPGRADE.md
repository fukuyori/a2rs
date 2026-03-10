# A2RS Phase12: Disk II realism upgrade

## Goal
Expose and preserve Disk II realism-related runtime state so it can be inspected and survive quick save/load.

## Included
- Save state format v3
- Preserve motor-off scheduled cycle across quick save/load
- Disk realism HUD (Ctrl+F7)
- Detailed per-drive realism diagnostics

## HUD contents
- controller mode / sequencer mode
- motor state and pending spin-down cycles
- RWTS session / speed mode / latch / shift register
- per-drive loaded / write-protected / dirty state
- track / half-track / phase / phase_precise
- spinning / write light / media position

## Why
Strict sequencer, write support, and motor spin-down realism are harder to debug without a direct visibility path.
This phase keeps the code practical while avoiding deeper WOZ/NIB-specific changes for now.
