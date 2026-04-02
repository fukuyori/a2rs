//! WOZ ディスクイメージパーサー
//!
//! WOZ 1.0 および WOZ 2.0 フォーマットをパースし、
//! エミュレータが使用する NIB 形式（35トラック × 6656バイト）に変換する。
//!
//! 参考仕様:
//! - WOZ 1.0: https://applesaucefdc.com/woz/reference1/
//! - WOZ 2.0: https://applesaucefdc.com/woz/reference2/

use crate::disk::{WOZ_HALF_TRACKS, NIB_TRACK_SIZE, WOZ_NIB_SIZE};

/// WOZ 1.0 マジックバイト: "WOZ1" + FF 0A 0D 0A
pub const WOZ1_MAGIC: &[u8; 8] = b"WOZ1\xFF\x0A\x0D\x0A";
/// WOZ 2.0 マジックバイト: "WOZ2" + FF 0A 0D 0A
pub const WOZ2_MAGIC: &[u8; 8] = b"WOZ2\xFF\x0A\x0D\x0A";

/// WOZ ファイルかどうかを先頭マジックで判定
pub fn is_woz(data: &[u8]) -> bool {
    data.len() >= 8 && (data.starts_with(WOZ1_MAGIC) || data.starts_with(WOZ2_MAGIC))
}

/// WOZ パース結果: NIBデータ + ビットストリーム
pub struct WozParseResult {
    /// NIB形式データ（80ハーフトラック × 6656バイト、Safeモード用）
    pub nib_data: Vec<u8>,
    /// ハーフトラックごとの実ニブル数
    pub track_nibble_counts: Vec<usize>,
    /// ハーフトラックごとの生ビットストリーム（パック済み、MSB first）
    /// ビットレベルシーケンサエミュレーション用
    pub bitstreams: Vec<Vec<u8>>,
    /// ハーフトラックごとのビット数
    pub bit_counts: Vec<usize>,
}

/// WOZ ファイルを NIB 形式（80ハーフトラック × 6656バイト）に変換
///
/// # Errors
/// - マジックバイト不一致
/// - 必須チャンク（TMAP / TRKS）の欠如または破損
pub fn parse_woz(data: &[u8]) -> Result<WozParseResult, &'static str> {
    if data.len() < 12 {
        return Err("WOZ file too short");
    }
    if data.starts_with(WOZ1_MAGIC) {
        parse_woz1(data)
    } else if data.starts_with(WOZ2_MAGIC) {
        parse_woz2(data)
    } else {
        Err("Not a WOZ file")
    }
}

// ─── 内部ユーティリティ ─────────────────────────────────────────────────────

#[inline]
fn read_u16_le(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

#[inline]
fn read_u32_le(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

/// WOZ チャンクを検索してデータ部スライスを返す
///
/// チャンク構造: ID(4) + SIZE(4 LE) + DATA(SIZE バイト)
/// ファイル内の走査開始はオフセット 12（マジック8 + CRC32 4）から。
fn find_chunk<'a>(data: &'a [u8], chunk_id: &[u8; 4]) -> Option<&'a [u8]> {
    let mut pos: usize = 12;
    while pos + 8 <= data.len() {
        let size = read_u32_le(data, pos + 4) as usize;
        if &data[pos..pos + 4] == chunk_id {
            let start = pos + 8;
            let end = start.saturating_add(size);
            if end <= data.len() {
                return Some(&data[start..end]);
            }
        }
        // オーバーフロー対策
        pos = pos.saturating_add(8).saturating_add(size);
    }
    None
}

/// WOZ ビットストリームから Apple II ディスクバイト列（NIB 互換）を抽出して
/// NIB バッファに書き込む。
///
/// WOZ 形式はディスクヘッドが読む**生ビットストリーム**（MSB first）を格納する。
/// 自己同期バイト（sync byte）はビット列上では 10 ビット（`1111111100`）だが、
/// NIB 形式では 8 ビット（`0xFF`）として扱われる。
///
/// Apple II Disk II コントローラと同じアルゴリズムを使う:
/// シフトレジスタにビットを 1 本ずつ入れ、bit 7 が立った瞬間に
/// 1 バイトをラッチして出力する。これにより自己同期が自然に実現される。
/// ビットストリームからNIBバイト列を抽出してバッファに書き込み、
/// 抽出されたニブル数（= 1回転分のバイト数）を返す。
fn fill_nib_track(nib: &mut [u8], track: usize, track_bits: &[u8]) -> usize {
    if track_bits.is_empty() {
        return 0;
    }

    // ---- Phase 1: ビットストリーム → ディスクバイト列（NIB 互換）に変換 ----
    //
    // シフトレジスタが 0x80 以上になった瞬間 = bit 7 が 1 = 有効バイト検出。
    // ラッチしてレジスタをリセット。10 ビット自己同期バイトは自然に 0xFF として抽出される。
    let mut extracted: Vec<u8> = Vec::with_capacity(NIB_TRACK_SIZE);
    let mut shift_reg: u8 = 0;

    for &byte in track_bits {
        for bit_idx in (0..8).rev() {
            let bit = (byte >> bit_idx) & 1;
            shift_reg = (shift_reg << 1) | bit;
            if shift_reg >= 0x80 {
                extracted.push(shift_reg);
                shift_reg = 0;
            }
        }
    }

    if extracted.is_empty() {
        return 0;
    }

    // ---- Phase 2: 抽出バイトを循環させて NIB_TRACK_SIZE バイトを埋める ----
    let dst_base = track * NIB_TRACK_SIZE;
    let src_len = extracted.len();
    let mut dst_pos = 0usize;
    while dst_pos < NIB_TRACK_SIZE {
        let src_off = dst_pos % src_len;
        let copy_len = (src_len - src_off).min(NIB_TRACK_SIZE - dst_pos);
        nib[dst_base + dst_pos..dst_base + dst_pos + copy_len]
            .copy_from_slice(&extracted[src_off..src_off + copy_len]);
        dst_pos += copy_len;
    }

    src_len
}

// ─── WOZ 1.0 パーサー ─────────────────────────────────────────────────────

/// WOZ 1.0 TRKS チャンク内の 1 トラックエントリーサイズ（バイト）
const WOZ1_TRACK_ENTRY_SIZE: usize = 6656;
/// WOZ 1.0 ビットストリームフィールドのサイズ（バイト）
const WOZ1_BITS_SIZE: usize = 6646;
/// WOZ 1.0 BYTES_USED フィールドのエントリ内オフセット
const WOZ1_BYTES_USED_OFFSET: usize = WOZ1_BITS_SIZE; // 6646

fn parse_woz1(data: &[u8]) -> Result<WozParseResult, &'static str> {
    let tmap = find_chunk(data, b"TMAP").ok_or("TMAP chunk not found")?;
    if tmap.len() < 160 {
        return Err("TMAP chunk too small");
    }
    let trks = find_chunk(data, b"TRKS").ok_or("TRKS chunk not found")?;

    let mut nib = vec![0xFFu8; WOZ_NIB_SIZE];
    let mut track_nibble_counts = vec![NIB_TRACK_SIZE; WOZ_HALF_TRACKS];
    let mut bitstreams: Vec<Vec<u8>> = vec![Vec::new(); WOZ_HALF_TRACKS];
    let mut bit_counts: Vec<usize> = vec![0; WOZ_HALF_TRACKS];

    for phase in 0..WOZ_HALF_TRACKS {
        let qt = phase * 2;
        if qt >= 160 {
            break;
        }
        let trk_idx = tmap[qt] as usize;
        if trk_idx == 0xFF {
            continue;
        }

        let entry_off = trk_idx * WOZ1_TRACK_ENTRY_SIZE;
        if entry_off + WOZ1_TRACK_ENTRY_SIZE > trks.len() {
            continue;
        }

        let bytes_used = read_u16_le(trks, entry_off + WOZ1_BYTES_USED_OFFSET) as usize;
        let bytes_used = bytes_used.min(WOZ1_BITS_SIZE).max(1);

        // WOZ 1.0: BIT_COUNT は BYTES_USED の直後（offset 6648）の u16 LE
        let raw_bit_count = read_u16_le(trks, entry_off + WOZ1_BYTES_USED_OFFSET + 2) as usize;
        let track_bit_count = if raw_bit_count > 0 { raw_bit_count } else { bytes_used * 8 };

        let nibble_count = fill_nib_track(&mut nib, phase, &trks[entry_off..entry_off + bytes_used]);
        if nibble_count > 0 {
            track_nibble_counts[phase] = nibble_count;
        }

        // 生ビットストリームをコピー
        bitstreams[phase] = trks[entry_off..entry_off + bytes_used].to_vec();
        bit_counts[phase] = track_bit_count;
    }

    Ok(WozParseResult { nib_data: nib, track_nibble_counts, bitstreams, bit_counts })
}

// ─── WOZ 2.0 パーサー ─────────────────────────────────────────────────────

/// WOZ 2.0 TRK エントリーサイズ（バイト）
const WOZ2_TRK_ENTRY_SIZE: usize = 8;

fn parse_woz2(data: &[u8]) -> Result<WozParseResult, &'static str> {
    let tmap = find_chunk(data, b"TMAP").ok_or("TMAP chunk not found")?;
    if tmap.len() < 160 {
        return Err("TMAP chunk too small");
    }
    let trks_chunk = find_chunk(data, b"TRKS").ok_or("TRKS chunk not found")?;

    let mut nib = vec![0xFFu8; WOZ_NIB_SIZE];
    let mut track_nibble_counts = vec![NIB_TRACK_SIZE; WOZ_HALF_TRACKS];
    let mut bitstreams: Vec<Vec<u8>> = vec![Vec::new(); WOZ_HALF_TRACKS];
    let mut bit_counts_out: Vec<usize> = vec![0; WOZ_HALF_TRACKS];

    for phase in 0..WOZ_HALF_TRACKS {
        let qt = phase * 2;
        if qt >= 160 {
            break;
        }
        let trk_idx = tmap[qt] as usize;
        if trk_idx == 0xFF {
            continue;
        }

        let entry_off = trk_idx * WOZ2_TRK_ENTRY_SIZE;
        if entry_off + WOZ2_TRK_ENTRY_SIZE > trks_chunk.len() {
            continue;
        }

        let starting_block = read_u16_le(trks_chunk, entry_off) as usize;
        let block_count    = read_u16_le(trks_chunk, entry_off + 2) as usize;
        let bit_count      = read_u32_le(trks_chunk, entry_off + 4) as usize;

        if starting_block == 0 || block_count == 0 {
            continue;
        }

        let bits_start = starting_block * 512;
        let bits_end   = bits_start + block_count * 512;
        if bits_end > data.len() {
            continue;
        }

        let byte_count = ((bit_count + 7) / 8).min(block_count * 512).max(1);

        let nibble_count = fill_nib_track(&mut nib, phase, &data[bits_start..bits_start + byte_count]);
        if nibble_count > 0 {
            track_nibble_counts[phase] = nibble_count;
        }

        // 生ビットストリームをコピー
        bitstreams[phase] = data[bits_start..bits_start + byte_count].to_vec();
        bit_counts_out[phase] = bit_count;
    }

    Ok(WozParseResult { nib_data: nib, track_nibble_counts, bitstreams, bit_counts: bit_counts_out })
}
