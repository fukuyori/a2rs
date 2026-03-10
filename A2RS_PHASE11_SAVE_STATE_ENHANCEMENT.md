# A2RS Phase 11: Save State Enhancement

## Goal

Bring quick save / quick load up to date with the current emulator core so that state restore is consistent with:

- strict / transitional Disk II sequencer state
- tick-driven video timing state
- floating bus state
- pending CPU cycle execution state
- drive metadata needed for write-back workflows

## What is now saved

### CPU
- A/X/Y/SP/PC/P
- total cycles
- IRQ/NMI pending

### Memory
- main RAM
- language card RAM
- LC soft switches
- video soft switches
- keyboard latch

### Disk II
- current drive
- latch / motor / write mode / load mode
- shift register / nibble shift state / raw bit timing counters
- controller state and sequencer mode
- sync window / pending write latch
- per-drive disk image bytes
- per-drive `dsk_data`, format, filename, modified flag
- per-drive byte position / phase / phase_precise / spinning / write light

### Video / Timing
- flash state
- flash counter
- video scanner frame / scanline / scanline cycle
- pending CPU cycles
- floating bus value / address
- total cycles / frame count

## Compatibility

- save format version is bumped to `2`
- loader accepts version `1` and `2`
- version `1` states fall back to reconstructing video scanner position from total cycle count

## Notes

- quick save remains slot-based only
- normal save is still intentionally not implemented
- pending speaker click queue is cleared on load to avoid stale audio clicks after restore
