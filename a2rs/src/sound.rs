//! Apple II サウンドエミュレーション (1bit Speaker)
//!
//! Apple IIのスピーカーは$C030をアクセスするとトグルする単純な仕組み。
//! 波形は変えず、耳に刺さる成分だけを時間方向で丸める。

use std::collections::VecDeque;

#[cfg(feature = "audio")]
use std::sync::atomic::{AtomicUsize, Ordering};

/// サンプルレート (Hz)
pub const SAMPLE_RATE: u32 = 44100;

/// 1フレームあたりのサンプル数 (44100 / 60)
const SAMPLES_PER_FRAME: usize = 735;

/// リングバッファサイズ（約0.2秒分）
#[cfg(feature = "audio")]
const RING_BUFFER_SIZE: usize = 8192;

/// 1-pole IIR ローパスフィルタ（シンプル・高速・十分）
struct LowPass {
    alpha: f32,
    z: f32,
}

impl LowPass {
    fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);
        Self { alpha, z: 0.0 }
    }

    fn process(&mut self, input: f32) -> f32 {
        self.z += self.alpha * (input - self.z);
        self.z
    }
}

/// ソフトサチュレーション（tanh系・安全・軽量）
fn soft_saturate(x: f32) -> f32 {
    (x * 1.5).tanh()
}

/// スピーカー慣性（紙コーンの慣性を再現）
fn speaker_inertia(prev: f32, current: f32) -> f32 {
    prev + 0.2 * (current - prev)
}

/// Apple IIスピーカーエミュレータ (1bit方式)
pub struct Speaker {
    /// クリックイベントキュー (サイクル数)
    click_queue: VecDeque<u64>,
    /// サウンドが有効か
    enabled: bool,
    /// ボリューム（0.0 - 1.0）
    volume: f32,
    /// 現在のスピーカー状態 (true = HIGH, false = LOW)
    speaker_state: bool,
    /// 前回のスピーカー出力（慣性用）
    prev_speaker_output: f32,
    /// サンプル生成用バッファ（再利用）
    sample_buffer: Vec<f32>,
    /// ローパスフィルタ
    lpf: LowPass,
    /// 最後に処理したサイクル
    last_processed_cycle: u64,
    /// 最後のクリックからの経過フレーム
    silent_frames: u32,
    /// フェードアウト中のゲイン
    fade_gain: f32,
}

impl Speaker {
    pub fn new() -> Self {
        Speaker {
            click_queue: VecDeque::with_capacity(4096),
            enabled: true,
            volume: 0.25,
            speaker_state: false,
            prev_speaker_output: 0.0,
            sample_buffer: vec![0.0; SAMPLES_PER_FRAME],
            lpf: LowPass::new(4000.0, SAMPLE_RATE as f32),
            last_processed_cycle: 0,
            silent_frames: 100,
            fade_gain: 0.0,
        }
    }

    /// スピーカーをクリック（$C030アクセス時に呼ばれる）
    pub fn click(&mut self, cycle: u64) {
        self.click_queue.push_back(cycle);
        self.silent_frames = 0;
        if self.click_queue.len() > 8192 {
            self.click_queue.pop_front();
        }
    }

    /// サウンドの有効/無効を設定
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// ボリュームを設定（0.0 - 1.0）
    #[allow(dead_code)]
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// オーディオサンプルを生成
    pub fn generate_samples(&mut self, base_cycle: u64, cycles_per_frame: u64) -> Option<&[f32]> {
        if !self.enabled || cycles_per_frame == 0 {
            return None;
        }

        let end_cycle = base_cycle + cycles_per_frame;
        
        // このフレームでクリックがあるか確認
        let has_clicks = self.click_queue.iter().any(|&c| c < end_cycle);
        
        if !has_clicks {
            self.silent_frames = self.silent_frames.saturating_add(1);
            
            // 音がフェードアウト中でない、または完全にフェードアウトした場合
            if self.fade_gain < 0.001 {
                return None;
            }
            
            // フェードアウト処理
            for sample in self.sample_buffer.iter_mut() {
                self.fade_gain *= 0.995;
                let s = self.prev_speaker_output * self.fade_gain;
                *sample = self.lpf.process(s) * self.volume;
            }
            
            return Some(&self.sample_buffer);
        }
        
        // クリックがある場合はフェードインして処理
        self.silent_frames = 0;
        self.fade_gain = 1.0;
        
        let cycles_per_sample = cycles_per_frame as f32 / SAMPLES_PER_FRAME as f32;
        
        // 各サンプルを生成
        for i in 0..SAMPLES_PER_FRAME {
            let sample_cycle = base_cycle + (i as f32 * cycles_per_sample) as u64;
            
            // このサンプル時点までのクリックを処理
            while let Some(&click_cycle) = self.click_queue.front() {
                if click_cycle <= sample_cycle {
                    self.click_queue.pop_front();
                    self.speaker_state = !self.speaker_state;
                } else {
                    break;
                }
            }
            
            // 1bit → PCM化（-1.0 〜 +1.0）
            let raw_pcm = if self.speaker_state { 1.0 } else { -1.0 };
            
            // スピーカー慣性（紙コーンの動き）
            let with_inertia = speaker_inertia(self.prev_speaker_output, raw_pcm);
            self.prev_speaker_output = with_inertia;
            
            // ローパスフィルタ
            let filtered = self.lpf.process(with_inertia);
            
            // ソフトサチュレーション
            let saturated = soft_saturate(filtered);
            
            // 音量適用
            self.sample_buffer[i] = saturated * self.volume;
        }

        // キューに残った古いイベントをクリーンアップ
        while let Some(&cycle) = self.click_queue.front() {
            if cycle < end_cycle {
                self.click_queue.pop_front();
                self.speaker_state = !self.speaker_state;
            } else {
                break;
            }
        }
        
        self.last_processed_cycle = end_cycle;

        Some(&self.sample_buffer)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.click_queue.clear();
        self.speaker_state = false;
        self.prev_speaker_output = 0.0;
        self.lpf.z = 0.0;
        self.silent_frames = 100;
        self.fade_gain = 0.0;
    }
}

impl Default for Speaker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// オーディオ出力（rodioが有効な場合のみ）
// ============================================================

#[cfg(feature = "audio")]
use rodio::{OutputStream, Sink, Source};

#[cfg(feature = "audio")]
pub struct AudioOutput {
    _stream: OutputStream,
    sink: Sink,
    ring_buffer: std::sync::Arc<RingBuffer>,
}

#[cfg(feature = "audio")]
struct RingBuffer {
    data: Box<[f32; RING_BUFFER_SIZE]>,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
}

#[cfg(feature = "audio")]
impl RingBuffer {
    fn new() -> Self {
        RingBuffer {
            data: Box::new([0.0; RING_BUFFER_SIZE]),
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
        }
    }
    
    fn write(&self, samples: &[f32]) {
        let mut write_pos = self.write_pos.load(Ordering::Relaxed);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        
        for &sample in samples {
            let next_pos = (write_pos + 1) % RING_BUFFER_SIZE;
            if next_pos == read_pos {
                break;
            }
            unsafe {
                let ptr = self.data.as_ptr() as *mut f32;
                *ptr.add(write_pos) = sample;
            }
            write_pos = next_pos;
        }
        self.write_pos.store(write_pos, Ordering::Release);
    }
    
    fn read(&self) -> f32 {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        
        if read_pos == write_pos {
            return 0.0;
        }
        
        let sample = unsafe {
            let ptr = self.data.as_ptr();
            *ptr.add(read_pos)
        };
        
        let next_pos = (read_pos + 1) % RING_BUFFER_SIZE;
        self.read_pos.store(next_pos, Ordering::Release);
        sample
    }
    
    fn available(&self) -> usize {
        let write_pos = self.write_pos.load(Ordering::Relaxed);
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        
        if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            RING_BUFFER_SIZE - read_pos + write_pos
        }
    }
}

#[cfg(feature = "audio")]
impl AudioOutput {
    pub fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to create audio output: {}", e))?;
        
        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;
        
        let ring_buffer = std::sync::Arc::new(RingBuffer::new());
        
        let source = RingBufferSource {
            buffer: std::sync::Arc::clone(&ring_buffer),
            sample_rate: SAMPLE_RATE,
            last_sample: 0.0,
        };
        
        sink.append(source);
        
        Ok(AudioOutput {
            _stream: stream,
            sink,
            ring_buffer,
        })
    }

    pub fn play_samples(&mut self, samples: Option<&[f32]>) {
        if let Some(samples) = samples {
            if self.ring_buffer.available() < RING_BUFFER_SIZE - samples.len() - 100 {
                self.ring_buffer.write(samples);
            }
        }
    }
    
    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        !self.sink.is_paused()
    }
}

#[cfg(feature = "audio")]
struct RingBufferSource {
    buffer: std::sync::Arc<RingBuffer>,
    sample_rate: u32,
    last_sample: f32,
}

#[cfg(feature = "audio")]
impl Iterator for RingBufferSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.buffer.read();
        
        if sample == 0.0 && self.last_sample.abs() > 0.001 {
            self.last_sample *= 0.95;
            return Some(self.last_sample);
        }
        
        self.last_sample = sample;
        Some(sample)
    }
}

#[cfg(feature = "audio")]
impl Source for RingBufferSource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<std::time::Duration> { None }
}

// ============================================================
// スタブ実装（rodioが無効な場合）
// ============================================================

#[cfg(not(feature = "audio"))]
pub struct AudioOutput { _dummy: () }

#[cfg(not(feature = "audio"))]
impl AudioOutput {
    pub fn new() -> Result<Self, String> { Ok(AudioOutput { _dummy: () }) }
    pub fn play_samples(&mut self, _samples: Option<&[f32]>) {}
}
