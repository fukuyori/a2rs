# A2RS Phase 8 Roadmap

Phase 8 focuses on hardening the current stable core instead of widening the feature surface too quickly.

## Priorities
1. Stricter Disk II sequencer behavior without regressing boot reliability
2. Weak bits / write splice support for more demanding disk images
3. True micro-op CPU work for dummy reads, RMW sequences, and page-cross timing
4. Regression automation around timing, disk boot, and floating bus behavior

## Guiding principle
The current v0.4.2 core is the stable baseline. New accuracy work should be introduced behind focused validation so that DOS/ProDOS/Lode Runner class boots stay green.
