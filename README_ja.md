# A2RS — Rust 製 Apple II エミュレータ

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.4.2-green.svg)](https://github.com/user/a2rs/releases)

[English README](README.md)

**A2RS** は Rust で書かれた高精度な Apple II エミュレータです。既存エミュレータのコードをコピーするのではなく、仕様書をもとにした実装を重視し、Apple II のハードウェアアーキテクチャを深く理解することに焦点を当てています。

<p align="center">
  <img src="docs/screenshot.png" alt="A2RS スクリーンショット" width="640">
</p>

## 主な機能

| 機能 | 説明 |
|------|------|
| **複数モデル対応** | Apple II, II+, IIe, IIe Enhanced |
| **高性能** | リリースビルドで 200 MHz 以上の等価速度 |
| **サイクル精度** | Klaus2m5 6502 機能テストに合格 |
| **Disk II エミュレーション** | DSK / DO / PO / NIB / WOZ 形式、高速ディスク対応 |
| **正確なビデオ出力** | テキスト、Lo-Res、Hi-Res、Double Hi-Res モード |
| **ゲームパッド対応** | パドルエミュレーション用ジョイスティック入力、JSON 設定対応 |
| **オーディオエミュレーション** | 1bit スピーカークリック音、音量調整 |
| **内蔵プロファイラ** | パフォーマンス分析とブート時間測定 |
| **デバッガ UI** | リアルタイム CPU / メモリ / ディスク監視 |
| **セーブステート** | 10 スロットのクイックセーブ / ロード |
| **柔軟な設定** | JSON 設定ファイル、OS 別デフォルトパス対応 |

---

## クイックスタート

```bash
# クローンとビルド
git clone https://github.com/user/a2rs.git
cd a2rs
cargo build --release

# ディスクイメージで起動
./target/release/a2rs -r roms/apple2e.rom -1 disks/dos33.dsk
```

---

## 動作要件

### Rust

Rust 1.70 以上。

### システム依存ライブラリ

<details>
<summary><b>Linux (Debian / Ubuntu)</b></summary>

```bash
# 必須
sudo apt-get install libxkbcommon-dev libwayland-dev

# クリップボード対応に必須
sudo apt-get install libxcb-xfixes0-dev

# オプション: オーディオ
sudo apt-get install libasound2-dev

# オプション: ゲームパッド
sudo apt-get install libudev-dev
```
</details>

<details>
<summary><b>Linux (Fedora)</b></summary>

```bash
# 必須
sudo dnf install libxkbcommon-devel wayland-devel

# オプション: オーディオ
sudo dnf install alsa-lib-devel

# オプション: ゲームパッド
sudo dnf install systemd-devel
```
</details>

<details>
<summary><b>macOS / Windows</b></summary>

追加のシステムライブラリは不要です。
</details>

---

## ビルド

```bash
# 標準ビルド（オーディオ・ゲームパッドはデフォルトで有効）
cargo build --release

# オプション機能なしでビルド
cargo build --release --no-default-features

# 全機能を有効にしてビルド
cargo build --release --features full

# ヘルプ表示
./target/release/a2rs --help
```

---

## 使い方

### 基本的な例

```bash
# Apple IIe ROM で DOS 3.3 を起動
a2rs -r roms/apple2e.rom -1 disks/dos33.dsk

# ROM サイズからモデルを自動判定
a2rs -r roms/apple2plus.rom -1 disks/game.dsk

# モデルを明示指定
a2rs -m iie -r roms/apple2e.rom -1 disks/prodos.dsk

# 2 ドライブ使用
a2rs -r roms/apple2e.rom -1 disk1.dsk -2 disk2.dsk

# 設定ファイルを指定
a2rs --config /path/to/config.json -1 game.dsk

# ホームディレクトリを指定
a2rs --home ~/Apple2 -1 dos33.dsk
```

### コマンドラインオプション

| オプション | 説明 |
|------------|------|
| `-1, --disk1 <FILE>` | ドライブ 1 のディスクイメージ |
| `-2, --disk2 <FILE>` | ドライブ 2 のディスクイメージ |
| `-r, --rom <FILE>` | Apple II ROM ファイル |
| `-m, --model <MODEL>` | `auto` \| `ii` \| `ii+` \| `iie` \| `iie-enhanced`（デフォルト: `auto`） |
| `--disk-rom <FILE>` | Disk II Boot ROM（256 バイト） |
| `--speed <N>` | 速度倍率: `1` = 通常速度、`0` = 最大速度（デフォルト: `1`） |
| `--size <WxH>` | ウィンドウサイズ（デフォルト: `640x480`） |
| `-c, --config <FILE>` | 設定ファイルのパス |
| `--home <PATH>` | A2RS ホームディレクトリ（相対パスの基準） |
| `--headless` | GUI なしで実行 |
| `--cycles <N>` | ヘッドレスモードで実行するサイクル数 |
| `--profile` | 内蔵プロファイラを有効化 |
| `--disk-log <LEVEL>` | ディスクログ詳細度: `none` \| `flow` \| `state` \| `decide` \| `all` |

---

## キーボード操作

### エミュレータ操作

| キー | 機能 |
|:---:|------|
| `F1` | 設定メニュー |
| `F2` | 速度切り替え（×1 → ×2 → ×5 → ×10 → MAX） |
| `F3` | レンダリング品質レベル切り替え |
| `F4` | 自動品質調整 ON / OFF |
| `F5` | クイックセーブ |
| `F6` | サウンド ON / OFF |
| `F8` | セーブスロット切り替え（0〜9） |
| `F9` | クイックロード |
| `F10` | スクリーンショット |
| `F11` | デバッガパネル表示 / 非表示 |
| `F12` | リセット |
| `ESC` | メニューを閉じる（メニュー未表示時は Apple II に送信） |
| `Ctrl+0〜9` | セーブスロットを直接選択 |

### デバッガ操作（パネル表示中）

| キー | 機能 |
|:---:|------|
| `Tab` | タブ切り替え |
| `F6` | ステップ実行 |
| `F7` | 継続 |
| `F8` | ブレーク / 一時停止 |
| `↑` `↓` | メモリビュースクロール |
| `PageUp/Down` | メモリビュー高速スクロール |

### ツールバーボタン

| ボタン | 機能 |
|:------:|------|
| ▶ / ⏸ | 再生 / 一時停止 |
| ⟳ | リセット |
| 💾1 | ドライブ 1 ディスクメニュー |
| 💾2 | ドライブ 2 ディスクメニュー |
| ⇄ | ドライブ間でディスクを入れ替え |
| 💾 | ステート保存 |
| 📂 | ステート読み込み |
| 📷 | スクリーンショット |

ツールバー右側に音量スライダーがあります。

---

## 対応モデル

| モデル | CPU | RAM | ROM サイズ | 備考 |
|--------|:---:|:---:|:----------:|------|
| Apple II | 6502 | 48 KB | — | オリジナル Apple II |
| Apple II+ | 6502 | 64 KB | 20 KB | Autostart ROM |
| Apple IIe | 6502 | 128 KB | 32 KB | 拡張 80 桁カード |
| Apple IIe Enhanced | 65C02 | 128 KB | 32 KB | MouseText 対応 |

---

## 対応ディスク形式

| 形式 | 拡張子 | サイズ | 説明 |
|------|:------:|:------:|------|
| DSK | `.dsk` `.do` | 140 KB | 標準ディスクイメージ（DOS 3.3 セクター順） |
| PO | `.po` | 140 KB | ProDOS セクター順ディスクイメージ |
| NIB | `.nib` | 227 KB | ニブライズドディスクイメージ（生ビットストリーム） |
| WOZ 1.0 | `.woz` | 可変 | Applesauce WOZ 形式 v1（読み取り専用） |
| WOZ 2.0 | `.woz` | 可変 | Applesauce WOZ 形式 v2（読み取り専用） |

> WOZ イメージはマジックバイトで自動検出され、ロード時に内部 NIB 形式に変換されます。WOZ への書き戻しは非対応です。

---

## ROM ファイルについて

> **著作権の関係上、ROM ファイルは含まれていません。ご自身でご用意ください。**

| ROM | サイズ | 備考 |
|-----|:------:|------|
| Apple II Plus ROM | 20,480 バイト（20 KB） | |
| Apple IIe ROM | 32,768 バイト（32 KB） | |
| Disk II Boot ROM | 256 バイト | オプション、ファイル名 `disk2.rom` |

`roms/` ディレクトリに配置するか、`--rom` / `--disk-rom` で指定してください。

---

## ディレクトリ構成

A2RS はプラットフォームごとのデフォルトディレクトリにファイルを保存します。

| OS | デフォルトホーム |
|----|----------------|
| Windows | `%LOCALAPPDATA%\a2rs\` |
| macOS | `~/Library/Application Support/a2rs/` |
| Linux | `~/.local/share/a2rs/` |

```
a2rs_home/
├── roms/               # ROM ファイル
│   ├── apple2e.rom
│   └── disk2.rom
├── disks/              # ディスクイメージ
│   ├── dos33.dsk
│   └── games/
├── saves/              # セーブステート（quicksave.json、save_slot_1.json…）
└── screenshots/        # PNG スクリーンショット
```

設定ファイルは別の場所に置かれます。

| OS | 設定ファイルパス |
|----|----------------|
| Windows | `%APPDATA%\a2rs\config.json` |
| macOS | `~/Library/Application Support/a2rs/config.json` |
| Linux | `~/.config/a2rs/config.json` |

---

## 設定ファイル

A2RS は起動時に `config.json` を読み込みます。**GUI での変更は設定ファイルに書き戻されません**（セッション中のみ有効）。

### 完全なサンプル

```json
{
  "a2rs_home": "",
  "rom_dir": "roms",
  "disk_dir": "disks",
  "screenshot_dir": "screenshots",
  "save_dir": "saves",
  "speed": 1,
  "fast_disk": true,
  "sound_enabled": true,
  "volume": 0.5,
  "quality_level": 4,
  "auto_quality": true,
  "window_width": 560,
  "window_height": 384,
  "current_slot": 0,
  "gamepad": {
    "enabled": true,
    "deadzone": 0.15,
    "show_debug_overlay": false,
    "button_0_inputs": ["South", 288],
    "button_1_inputs": ["East", 289],
    "profiles": [
      {
        "name_contains": "USB,4-Axis,12-Button with POV",
        "settings": {
          "button_0_inputs": [288],
          "button_1_inputs": [289]
        }
      }
    ],
    "axis_x_code": 0,
    "axis_y_code": 1,
    "hat_x_code": 16,
    "hat_y_code": 17
  },
  "experimental": {
    "disk_sequencer_mode": "safe",
    "weak_bits": false,
    "write_splice": false,
    "disk_debug_logging": false
  }
}
```

すべてのキーは省略可能です。省略した場合は上記のデフォルト値が使われます。

---

### パス設定

#### `a2rs_home`
**型:** 文字列 — **デフォルト:** `""` (OS デフォルトパス)

他のすべての相対パス（`rom_dir`、`disk_dir` など）の基準となるディレクトリです。

| 値 | 動作 |
|----|------|
| `""` (空文字) | OS のデフォルトホームディレクトリを使用（上記テーブル参照） |
| 絶対パス | そのパスをそのまま使用 |
| 相対パス | 設定ファイルのディレクトリからの相対パスとして解決 |
| `~` から始まるパス | ユーザーのホームディレクトリに展開 |

```json
// カスタムパスを使用
{ "a2rs_home": "/Users/alice/retro/apple2" }

// Windows の例
{ "a2rs_home": "D:/Games/Apple2" }

// チルダ展開
{ "a2rs_home": "~/retro/apple2" }
```

コマンドラインから `--home <PATH>` でも上書きできます。

---

#### `rom_dir`
**型:** 文字列 — **デフォルト:** `"roms"`

ROM ファイルを検索するディレクトリです。相対パスは `a2rs_home` からの相対パスとして解決されます。

```json
{ "rom_dir": "roms" }              // → <a2rs_home>/roms/
{ "rom_dir": "/opt/apple2/roms" }  // 絶対パス
```

---

#### `disk_dir`
**型:** 文字列 — **デフォルト:** `"disks"`

ディスクメニューに表示するディスクイメージを検索するディレクトリです。このディレクトリを最大 3 階層まで再帰的にスキャンし、`.dsk`、`.do`、`.po`、`.nib`、`.woz` ファイルを列挙します。

```json
{ "disk_dir": "disks" }            // → <a2rs_home>/disks/
{ "disk_dir": "/mnt/nas/apple2" }   // 絶対パス
```

---

#### `screenshot_dir`
**型:** 文字列 — **デフォルト:** `""` (→ `<a2rs_home>/screenshots/`)

スクリーンショット（PNG）の保存先ディレクトリです。空の場合は `<a2rs_home>/screenshots/` が使われます。

```json
{ "screenshot_dir": "screenshots" }
{ "screenshot_dir": "~/Desktop/apple2-screenshots" }
```

---

#### `save_dir`
**型:** 文字列 — **デフォルト:** `"saves"`

セーブステートファイルの保存先ディレクトリです。スロット 0 は `quicksave.json`、スロット 1〜9 は `save_slot_1.json`〜`save_slot_9.json` という名前で保存されます。

```json
{ "save_dir": "saves" }  // → <a2rs_home>/saves/
```

---

### エミュレーション設定

#### `speed`
**型:** 整数 — **デフォルト:** `1`

CPU 速度の倍率です。

| 値 | 説明 |
|:--:|------|
| `1` | 通常速度（実機 Apple II の約 1 MHz） |
| `2` | 2 倍速 |
| `5` | 5 倍速 |
| `10` | 10 倍速 |
| `0` | 最大速度（スロットルなし）— ブートブーストも有効になります |

実行中に `F2` で切り替えられます。

---

#### `fast_disk`
**型:** 真偽値 — **デフォルト:** `true`

SafeFast ディスク高速化を有効にします。有効時は、DOS 3.3 の標準 RWTS ルーチンを検出して高速読み取り最適化を適用し、非標準のアクセスパターンを検出すると自動的にニブル単位の正確エミュレーションに戻ります。

無効にするとディスクアクセスが実機速度になりますが、最高精度で動作します。

---

#### `sound_enabled`
**型:** 真偽値 — **デフォルト:** `true`

オーディオ出力を有効または無効にします。`F6` で実行中に切り替えられます。

---

#### `volume`
**型:** 浮動小数点数（0.0〜1.0） — **デフォルト:** `0.5`

マスター音量です。`0.0` が無音、`1.0` が最大音量。ツールバーのスライダーで調整できます。

---

#### `quality_level`
**型:** 整数（0〜4） — **デフォルト:** `4`

レンダリング品質 / フレームレートの目標値です。

| 値 | 説明 |
|:--:|------|
| `0` | 最低品質（最高速） |
| `1〜3` | 中間レベル |
| `4` | フル品質（Apple II の実際のフレームレートに合わせる） |

`F3` で切り替えます。`auto_quality` が ON のときは自動調整されます。

---

#### `auto_quality`
**型:** 真偽値 — **デフォルト:** `true`

有効時は、FPS が落ちると品質レベルを素早く下げ、安定した回復後にゆっくりと品質を戻します。低スペックの環境でエミュレーションをスムーズに保つための機能です。`F4` で切り替えます。

---

#### `window_width` / `window_height`
**型:** 整数 — **デフォルト:** `560` × `384`

ウィンドウの初期サイズ（ピクセル）です。Apple II の表示は 280×192 で、デフォルトの 560×384 はツールバーとステータスバーを含む 2 倍サイズです。

---

#### `current_slot`
**型:** 整数（0〜9） — **デフォルト:** `0`

クイックセーブ（`F5`）・クイックロード（`F9`）で使用するスロット番号です。

| 値 | ファイル名 |
|:--:|-----------|
| `0` | `quicksave.json` |
| `1`〜`9` | `save_slot_1.json`〜`save_slot_9.json` |

`F8` で順番に切り替え、`Ctrl+0`〜`Ctrl+9` で直接選択できます。

---

### `gamepad` セクション

Apple II のパドルエミュレーション用ゲームパッド / ジョイスティック入力の設定です。

#### `config.json` の書き方

共通設定は `gamepad` オブジェクト直下に書きます。
特定のコントローラーだけ別設定にしたい場合は `profiles` に追加し、違う項目だけを `settings` で上書きします。

基本的な流れ:

1. 共通の割り当てを `gamepad.button_0_inputs` と `gamepad.button_1_inputs` に書く
2. コントローラーを接続して起動ログに出る名前を確認する
3. その名前の一部を `name_contains` に書いた `profiles` を追加する
4. その機種だけ変えたい項目だけを `settings` に書く

最小例:

```json
{
  "gamepad": {
    "button_0_inputs": ["South", 288],
    "button_1_inputs": ["East", 289],
    "profiles": [
      {
        "name_contains": "USB,4-Axis,12-Button with POV",
        "settings": {
          "button_0_inputs": [288],
          "button_1_inputs": [289]
        }
      }
    ]
  }
}
```

各 `button_*_inputs` 配列では:
- `"South"` のような文字列は `gilrs` の論理ボタン名です
- `288` のような整数は raw ボタンコードです
- 同じ配列に混在して書けます

複数のゲームパッドが同時接続されている場合も、A2RS はデバイス名ごとに `profiles` を解決するため、種類ごとに別設定を適用できます。
Apple II 自体はボタンを 2 つしか使わないため、A2RS でも `button_0_inputs` と `button_1_inputs` だけを公開します。

#### `enabled`
**型:** 真偽値 — **デフォルト:** `true`

ゲームパッド入力を有効または無効にします。

---

#### `deadzone`
**型:** 浮動小数点数（0.0〜1.0） — **デフォルト:** `0.15`

アナログスティックのデッドゾーンです。この値より小さいスティック入力は中央（ゼロ）として扱われます。スティックを離しているのにカーソルがずれる場合は値を大きくしてください。

---

#### `show_debug_overlay`
**型:** 真偽値 — **デフォルト:** `false`

ウィンドウ左上にゲームパッドのデバッグオーバーレイを表示します（Linux 専用）。軸の値、D パッドの状態、ボタンの状態、最後の gilrs イベント名などをリアルタイムで表示します。コントローラーのボタンマッピングを確認したいときに役立ちます。

---

#### `button_0_inputs` / `button_1_inputs`
**型:** 文字列または整数の配列 — **デフォルト:** `["South",288]` / `["East",289]`

Apple II の**ボタン 0**と**ボタン 1**に対応する入力です。各要素には `gilrs` が報告するボタン名文字列か、生の evdev ボタンコード整数を書けます。
デフォルトの論理割り当ては Windows / macOS / Linux で共通です: `South -> button_0`, `East -> button_1`。

よく使われる gilrs のボタン名: `South`、`East`、`North`、`West`、`C`、`Z`、`LeftTrigger`、`RightTrigger`、`Start`、`Select`、`Mode`。

---

#### `profiles`
**型:** 配列 — **デフォルト:** `[]`

コントローラーごとの上書き設定です。接続されたゲームパッド名に `name_contains` が含まれていると、その `settings` に書かれた項目だけがデフォルトの `gamepad` 設定に上書き適用されます。

例:

```json
{
  "name_contains": "USB,4-Axis,12-Button with POV",
  "settings": {
    "button_0_inputs": [288],
    "button_1_inputs": [289]
  }
}
```

A2RS 起動時点でゲームパッドが接続されている場合は、起動ログにゲームパッド名と適用したプロファイル名を表示します。

---

#### `axis_x_code` / `axis_y_code`
**型:** 整数 — **デフォルト:** `0` / `1`

左アナログスティックの evdev 軸コードです（Linux フォールバック）。

---

#### `hat_x_code` / `hat_y_code`
**型:** 整数 — **デフォルト:** `16` / `17`

D パッド（ハット）の evdev 軸コードです（Linux フォールバック）。

---

### `experimental` セクション

高度な設定項目です。すべてデフォルトは最も安全・互換性の高い値に設定されています。**内容を理解している場合のみ変更してください。**

#### `disk_sequencer_mode`
**型:** 文字列 — **デフォルト:** `"safe"`

Disk II の低レベルシーケンサーの実装モードです。

| 値 | 説明 |
|:--:|------|
| `"safe"` | 標準エミュレーション。最大のソフトウェア互換性 |
| `"transitional"` | テスト用の中間モード |
| `"strict"` | ハードウェア精度優先のシーケンサー。一部のソフトウェアが動作しない可能性あり |

---

#### `weak_bits`
**型:** 真偽値 — **デフォルト:** `false`

コピープロテクトディスクに見られる「不安定ビット（ウィークビット）」現象のシミュレーションを有効にします。現時点では未実装です。

---

#### `write_splice`
**型:** 真偽値 — **デフォルト:** `false`

書き込みスプライス（1 回の書き込みの終端と次の書き込みの始端の境界）シミュレーションを有効にします。現時点では未実装です。

---

#### `disk_debug_logging`
**型:** 真偽値 — **デフォルト:** `false`

低レベルのディスクシーケンサーの詳細ログを標準出力に出力します。非常に大量のログが出ます。コマンドラインの `--disk-log all` と組み合わせると最大の詳細度になります。

---

## プロジェクト構成

```
a2rs/
├── src/
│   ├── main.rs          # エントリポイント、GUI、メインループ
│   ├── lib.rs           # ライブラリエクスポート
│   ├── apple2.rs        # エミュレータ統合
│   ├── cpu/
│   │   ├── mod.rs       # 6502/65C02 CPU コア
│   │   ├── addressing.rs
│   │   ├── opcodes.rs
│   │   └── opcodes2.rs  # 65C02 拡張命令
│   ├── memory.rs        # メモリマップ、ソフトスイッチ
│   ├── video.rs         # ビデオレンダリング
│   ├── disk.rs          # Disk II コントローラ
│   ├── disk_log.rs      # ディスクアクティビティログ
│   ├── woz.rs           # WOZ 1.0 / 2.0 パーサー
│   ├── sound.rs         # オーディオ出力
│   ├── gamepad.rs       # ゲームパッド / ジョイスティック対応
│   ├── gui.rs           # UI オーバーレイとメニュー
│   ├── profiler.rs      # パフォーマンスプロファイラ
│   ├── config.rs        # 設定管理
│   └── savestate.rs     # セーブステートのシリアライズ
├── Cargo.toml
├── README.md            # 英語 README
└── README_ja.md         # 本ファイル（日本語）
```

---

## テスト

```bash
# Klaus2m5 6502 機能テストを実行
cargo run --bin cpu_test

# デバッグログ付きで実行
RUST_LOG=debug cargo run -- -r roms/apple2e.rom -1 dos33.dsk

# ディスクアクティビティログ付きで実行
cargo run -- --disk-log flow -1 dos33.dsk
```

---

## インストーラのビルド

### Windows MSI

```bash
cargo install cargo-wix
cargo wix
# 出力: target/wix/a2rs-0.4.2-x86_64.msi
```

### Linux DEB

```bash
cargo install cargo-deb
cargo deb
```

### Linux RPM

```bash
cargo install cargo-generate-rpm
cargo generate-rpm
```

---

## Linux 固有の注意事項

- ゲームパッド入力は `gilrs` を使って毎フレーム状態を再読み込みするため、汎用 USB パッドでも反応しやすくなっています。
- ウィンドウのドラッグ移動は X11 / XWayland 向けです。純 Wayland 環境ではウィンドウマネージャの制約により動作しないことがあります。

---

## 変更履歴

### Version 0.4.2

- WOZ 1.0 / 2.0 ディスクイメージ対応を追加
- Mac でオーディオが鳴らない問題を修正（`audio` feature をデフォルト有効化）

### Version 0.4.0

- Phase 7 までの安定版ベースライン
- `experimental.disk_sequencer_mode` で `safe` / `transitional` / `strict` を指定可能に

### Version 0.3.0

- ツールバーに音量スライダー追加
- ホームディレクトリ指定オプション（`--home`）追加
- 設定ファイルパス指定オプション（`--config`）追加
- テキスト入力でのクリップボード貼り付け（Ctrl+V）対応
- ディスクメニューで最大 60 文字表示、ファイル名アルファベット順ソート
- 高速ディスクモードを常に ON に変更
- 設定メニューを `F1` に変更（旧 `ESC`）
- デバッガパネルを `F11` に変更（旧 `Tab`）

### Version 0.1.0

- 初回リリース
- Apple II / II+ / IIe / IIe Enhanced 対応
- Disk II エミュレーション
- セーブステート
- ゲームパッド対応

---

## ライセンス

MIT ライセンス — 詳細は [LICENSE](LICENSE) を参照してください。

---

## 参考資料

- [Beneath Apple DOS](https://archive.org/details/Beneath_Apple_DOS) — Disk II 必携ドキュメント
- [Understanding the Apple II](https://archive.org/details/understanding_the_apple_ii) — ハードウェアリファレンス
- [Klaus2m5 6502 Test Suite](https://github.com/Klaus2m5/6502_65C02_functional_tests) — CPU 検証テスト
- [AppleWin](https://github.com/AppleWin/AppleWin) — 参考実装
- [Applesauce WOZ Specification](https://applesaucefdc.com/woz/) — WOZ フォーマット仕様

---

<p align="center">Rust で作られています</p>
