# A2RS - Apple II Emulator in Rust 🍎

**A2RS** (Apple II in Rust) は、Rust で書かれた高精度な Apple II エミュレータです。

## 特徴

- 🎮 **フル機能エミュレーション** - Apple II, II+, IIe, IIe Enhanced 対応
- ⚡ **高速** - リリースビルドで実機の200倍以上の速度
- 🔧 **SafeFast** - コピープロテクト対応の安全な高速ディスクアクセス
- 📊 **プロファイラ内蔵** - パフォーマンス計測とブート時間分析
- 🐛 **デバッガUI** - リアルタイムCPU/メモリ/ディスク監視

## バージョン

現在: **v0.1.0**

## 完成した機能

### CPU エミュレーション
- **MOS 6502** - Apple II, Apple II+ 用
- **WDC 65C02** - Apple IIe Enhanced 用
- 全公式オペコードをサポート
- サイクル精度のエミュレーション
- **Klaus2m5 6502 functional test 合格**

### メモリシステム
- 64KB メインRAM + 64KB 補助RAM（Apple IIe）
- ランゲージカード（16KB追加RAM）
- ソフトスイッチによるメモリバンク切り替え

### ビデオモード
- **40x24 テキストモード**
- **Lo-Res グラフィックス** (40x48、16色)
- **Hi-Res グラフィックス** (280x192、6色)
- **Mixed モード**

### ディスクドライブ
- **Disk II エミュレーション**（スロット6）
- DSK/DO/PO/NIB形式対応
- **SafeFast高速化** - AppleWin互換

## ビルド方法

```bash
# 依存関係（Debian/Ubuntu）
sudo apt-get install libxkbcommon-dev libwayland-dev

# ビルド
cargo build --release

# フル機能ビルド
cargo build --release --features full

# 実行
./target/release/a2rs -1 dos33.dsk
```

## 使用方法

```bash
a2rs -1 dos33.dsk              # 基本起動
a2rs -f -1 dos33.dsk           # 高速ブート
a2rs --profile -1 dos33.dsk    # プロファイラ有効
```

### キーボード

| キー | 機能 |
|------|------|
| `Tab` | デバッガパネル |
| `F1` | 速度切り替え |
| `F5/F9` | セーブ/ロード |
| `F11` | フルスクリーン |
| `F12` | リセット |

## ライセンス

MIT License
