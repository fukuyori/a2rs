# A2RS Phase 6 – Disk II Nibble Timing

この版では Disk II の timing を **bit -> nibble** の二段階で進める。

- `tick_disk_bit()` は `4 CPU cycles = 1 disk bit` を維持
- Accurate モードでは 8bit ごとに 1 nibble をラッチ
- `read_write_nibble()` は Accurate 時に媒体位置を直接進めず、
  tick 側で進んだ最新ラッチを読む
- 書き込みも Accurate 時は 8bit 境界で媒体へ反映

## 到達点

- raw bit timing
- nibble boundary timing
- nibble latch timing
- shift register progression

## まだ残るもの

- Q6/Q7 のさらに厳密な state machine
- GCR decode / encode を媒体 bit stream と完全一致させること
- write splice / weak bits / copy-protection 固有挙動
- true micro-op CPU との bus 衝突

このため、この版は **Phase 6 の実用完成版** であり、
コピー保護まで含めた極限再現ではまだ拡張余地がある。
