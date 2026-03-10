# A2RS Phase 4 Hybrid

この版は、既存の命令単位6502実装を維持しつつ、system tick 上で CPU / Video / Disk を
1 CPU cycle ごとに同期するためのハイブリッド実装である。

含まれるもの:
- `CpuMicroState` による命令内マイクロサイクルの可視化
- `VideoScanner` と floating bus の scanline/cycle 駆動
- `Disk2InterfaceCard::tick_disk_bit()` による 4 cycles/bit 分周

未到達:
- 全命令の真の read/write micro-op 分解
- page crossing の bus 単位精密再現
- nibble シフトレジスタの完全 cycle-accurate 化

そのため位置づけとしては **Phase 4 のコンパイル可能な実装基盤** であり、
完全な MAME 級 micro-op CPU ではない。
