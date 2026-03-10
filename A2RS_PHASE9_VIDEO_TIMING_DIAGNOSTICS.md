# A2RS Phase9-A Video Timing Diagnostics

## 追加内容

- `VideoTimingDiagnostics` を追加
- `VideoScanner` の単体テストを追加
  - visible → hblank 遷移
  - visible → vblank 遷移
  - 1フレーム周回
- `main.rs` に video timing 診断オーバーレイを追加
  - `F7` で ON/OFF

## 表示内容

- frame
- scanline
- scanline cycle
- cycle in frame
- hblank / vblank
- floating bus address / value

## 目的

- tick-driven video timing の確認
- floating bus の参照位置確認
- hblank / vblank の実機相当挙動の目視確認
