# A2RS Phase 8 Groundwork

This build keeps the Phase 7 safe-read behavior intact while adding opt-in experimental knobs for the next round of Disk II work.

## Experimental config

```json
"experimental": {
  "disk_sequencer_mode": "safe",
  "weak_bits": false,
  "write_splice": false,
  "disk_debug_logging": false
}
```

### disk_sequencer_mode

- `safe`: current stable behavior
- `transitional`: reserved for partial strictness work
- `strict`: reserved for future fully strict sequencer work

The current release keeps all three modes behaviorally compatible on purpose, so the stable boot path is preserved while the configuration surface and internal state are ready.

## Goal

Prepare the codebase for:

- weak bits
- write splice handling
- stricter sequencer transitions
- future copy-protection experiments
