# A2RS Phase7 Disk II State Machine

この段階では、Disk II の Q6/Q7 線から導出されるコントローラ状態を明示的に保持する。

- `DiskControllerState`
  - `ReadSequencing`
  - `CheckWriteProtect`
  - `WriteLoad`
  - `WriteShift`
- `controller_edge_count`
- `last_controller_edge_cycle`
- `sync_bit_window`
- `pending_write_latch`

## 目的

Phase6 safe-read で確保したブート互換性を壊さずに、次の要素のための土台を作る。

- nibble write flush の厳密化
- weak bit / copy protection 対応
- sequencer line transition の診断
- Disk II status 可視化

## 現状

CPU から見える read path は従来互換のままにし、内部でのみ状態機械を前進させる。
そのため起動互換性を維持しながら Phase8 以降へ拡張できる。
