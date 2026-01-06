//! A2RS Profiler
//!
//! パフォーマンス計測とデバッグ情報の収集

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// プロファイラ設定
pub const PROFILER_ENABLED: bool = true;
pub const SAMPLE_INTERVAL_MS: u64 = 1000; // 1秒ごとにサンプリング

/// プロファイリングカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileCategory {
    /// CPU実行
    CpuExecution,
    /// ディスクI/O
    DiskIO,
    /// ディスクニブル読み取り
    DiskNibbleRead,
    /// ディスクセクタ検索
    DiskSectorSearch,
    /// メモリアクセス
    MemoryAccess,
    /// ビデオレンダリング
    VideoRender,
    /// オーディオ処理
    AudioProcess,
    /// GUI描画
    GuiRender,
    /// フレーム全体
    FrameTotal,
    /// SafeFast判定
    SafeFastCheck,
}

impl ProfileCategory {
    pub fn name(&self) -> &'static str {
        match self {
            ProfileCategory::CpuExecution => "CPU Exec",
            ProfileCategory::DiskIO => "Disk I/O",
            ProfileCategory::DiskNibbleRead => "Nibble Read",
            ProfileCategory::DiskSectorSearch => "Sector Search",
            ProfileCategory::MemoryAccess => "Memory",
            ProfileCategory::VideoRender => "Video",
            ProfileCategory::AudioProcess => "Audio",
            ProfileCategory::GuiRender => "GUI",
            ProfileCategory::FrameTotal => "Frame Total",
            ProfileCategory::SafeFastCheck => "SafeFast",
        }
    }
}

/// プロファイリング統計
#[derive(Debug, Clone, Default)]
pub struct ProfileStats {
    /// 累積時間
    pub total_time: Duration,
    /// 呼び出し回数
    pub call_count: u64,
    /// 最小時間
    pub min_time: Option<Duration>,
    /// 最大時間
    pub max_time: Option<Duration>,
}

impl ProfileStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record(&mut self, duration: Duration) {
        self.total_time += duration;
        self.call_count += 1;
        
        match self.min_time {
            None => self.min_time = Some(duration),
            Some(min) if duration < min => self.min_time = Some(duration),
            _ => {}
        }
        
        match self.max_time {
            None => self.max_time = Some(duration),
            Some(max) if duration > max => self.max_time = Some(duration),
            _ => {}
        }
    }
    
    pub fn average(&self) -> Duration {
        if self.call_count == 0 {
            Duration::ZERO
        } else {
            self.total_time / self.call_count as u32
        }
    }
    
    pub fn reset(&mut self) {
        self.total_time = Duration::ZERO;
        self.call_count = 0;
        self.min_time = None;
        self.max_time = None;
    }
}

/// ブート段階
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BootStage {
    /// 初期化
    Init,
    /// ブートROM実行中
    BootRom,
    /// セクタ0読み込み中
    Sector0,
    /// DOS読み込み中
    DosLoading,
    /// DOS初期化中
    DosInit,
    /// BASICプロンプト表示
    BasicPrompt,
    /// 完了
    Complete,
    /// エラー
    Error(&'static str),
}

impl BootStage {
    pub fn name(&self) -> &'static str {
        match self {
            BootStage::Init => "Init",
            BootStage::BootRom => "Boot ROM",
            BootStage::Sector0 => "Sector 0",
            BootStage::DosLoading => "DOS Loading",
            BootStage::DosInit => "DOS Init",
            BootStage::BasicPrompt => "BASIC Prompt",
            BootStage::Complete => "Complete",
            BootStage::Error(msg) => msg,
        }
    }
}

/// ディスクアクセス情報
#[derive(Debug, Clone)]
pub struct DiskAccessInfo {
    /// トラックアクセス回数
    pub track_accesses: [u32; 35],
    /// セクタ読み取り成功回数
    pub sectors_read: u32,
    /// セクタ読み取り失敗回数
    pub sectors_failed: u32,
    /// ニブル読み取り回数
    pub nibbles_read: u64,
    /// アドレスマーク検出回数
    pub address_marks_found: u32,
    /// データマーク検出回数
    pub data_marks_found: u32,
    /// 現在のトラック
    pub current_track: usize,
    /// 現在のセクタ
    pub current_sector: Option<usize>,
    /// 最後のI/Oアドレス
    pub last_io_address: u16,
    /// 最後のI/O値
    pub last_io_value: u8,
}

impl Default for DiskAccessInfo {
    fn default() -> Self {
        DiskAccessInfo {
            track_accesses: [0; 35],
            sectors_read: 0,
            sectors_failed: 0,
            nibbles_read: 0,
            address_marks_found: 0,
            data_marks_found: 0,
            current_track: 0,
            current_sector: None,
            last_io_address: 0,
            last_io_value: 0,
        }
    }
}

/// CPUプロファイル情報
#[derive(Debug, Clone)]
pub struct CpuProfileInfo {
    /// 実行命令数
    pub instructions_executed: u64,
    /// 総サイクル数
    pub total_cycles: u64,
    /// 命令別カウント（オプコードごと）
    pub opcode_counts: Box<[u32; 256]>,
    /// 最後のPC
    pub last_pc: u16,
    /// ループ検出用: PC履歴
    pub pc_history: Vec<u16>,
    /// 検出されたループ
    pub detected_loops: Vec<(u16, u32)>, // (PC, count)
}

impl Default for CpuProfileInfo {
    fn default() -> Self {
        CpuProfileInfo {
            instructions_executed: 0,
            total_cycles: 0,
            opcode_counts: Box::new([0; 256]),
            last_pc: 0,
            pc_history: Vec::new(),
            detected_loops: Vec::new(),
        }
    }
}

/// メインプロファイラ
pub struct Profiler {
    /// カテゴリ別統計
    stats: HashMap<ProfileCategory, ProfileStats>,
    /// 現在の計測開始時刻
    current_start: Option<(ProfileCategory, Instant)>,
    /// 有効フラグ
    pub enabled: bool,
    /// 最後のサンプル時刻
    last_sample: Instant,
    /// ブート段階
    pub boot_stage: BootStage,
    /// ブート開始時刻
    boot_start: Option<Instant>,
    /// 各ブート段階の所要時間
    pub boot_timings: HashMap<BootStage, Duration>,
    /// ディスクアクセス情報
    pub disk_info: DiskAccessInfo,
    /// CPU情報
    pub cpu_info: CpuProfileInfo,
    /// サンプル履歴（過去10サンプル）
    sample_history: Vec<HashMap<ProfileCategory, ProfileStats>>,
    /// フレームカウント
    frame_count: u64,
    /// 秒間フレーム数
    pub fps: f64,
    /// CPU速度（MHz相当）
    pub cpu_mhz: f64,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    pub fn new() -> Self {
        Profiler {
            stats: HashMap::new(),
            current_start: None,
            enabled: PROFILER_ENABLED,
            last_sample: Instant::now(),
            boot_stage: BootStage::Init,
            boot_start: None,
            boot_timings: HashMap::new(),
            disk_info: DiskAccessInfo::default(),
            cpu_info: CpuProfileInfo::default(),
            sample_history: Vec::new(),
            frame_count: 0,
            fps: 0.0,
            cpu_mhz: 0.0,
        }
    }
    
    /// 計測開始
    #[inline]
    pub fn start(&mut self, category: ProfileCategory) {
        if self.enabled {
            self.current_start = Some((category, Instant::now()));
        }
    }
    
    /// 計測終了
    #[inline]
    pub fn end(&mut self, category: ProfileCategory) {
        if let Some((cat, start)) = self.current_start.take() {
            if cat == category {
                let duration = start.elapsed();
                self.stats
                    .entry(category)
                    .or_insert_with(ProfileStats::new)
                    .record(duration);
            }
        }
    }
    
    /// 直接計測を記録
    #[inline]
    pub fn record(&mut self, category: ProfileCategory, duration: Duration) {
        if self.enabled {
            self.stats
                .entry(category)
                .or_insert_with(ProfileStats::new)
                .record(duration);
        }
    }
    
    /// カウントのみインクリメント（時間計測なし）
    #[inline]
    pub fn count(&mut self, category: ProfileCategory) {
        if self.enabled {
            self.stats
                .entry(category)
                .or_insert_with(ProfileStats::new)
                .call_count += 1;
        }
    }
    
    /// ブート開始
    pub fn start_boot(&mut self) {
        self.boot_start = Some(Instant::now());
        self.boot_stage = BootStage::BootRom;
        self.boot_timings.clear();
    }
    
    /// ブート段階を更新
    pub fn set_boot_stage(&mut self, stage: BootStage) {
        if let Some(start) = self.boot_start {
            let elapsed = start.elapsed();
            self.boot_timings.insert(self.boot_stage, elapsed);
        }
        self.boot_stage = stage;
    }
    
    /// ブート完了時間を取得
    pub fn boot_elapsed(&self) -> Option<Duration> {
        self.boot_start.map(|s| s.elapsed())
    }
    
    /// ディスクトラックアクセスを記録
    #[inline]
    pub fn record_track_access(&mut self, track: usize) {
        if self.enabled && track < 35 {
            self.disk_info.track_accesses[track] += 1;
            self.disk_info.current_track = track;
        }
    }
    
    /// ニブル読み取りを記録
    #[inline]
    pub fn record_nibble_read(&mut self) {
        if self.enabled {
            self.disk_info.nibbles_read += 1;
        }
    }
    
    /// セクタ読み取りを記録
    #[inline]
    pub fn record_sector_read(&mut self, success: bool) {
        if self.enabled {
            if success {
                self.disk_info.sectors_read += 1;
            } else {
                self.disk_info.sectors_failed += 1;
            }
        }
    }
    
    /// CPU命令実行を記録
    #[inline]
    pub fn record_instruction(&mut self, opcode: u8, pc: u16, cycles: u32) {
        if self.enabled {
            self.cpu_info.instructions_executed += 1;
            self.cpu_info.total_cycles += cycles as u64;
            self.cpu_info.opcode_counts[opcode as usize] += 1;
            self.cpu_info.last_pc = pc;
            
            // ループ検出（簡易版）
            if self.cpu_info.pc_history.len() >= 100 {
                self.cpu_info.pc_history.remove(0);
            }
            self.cpu_info.pc_history.push(pc);
        }
    }
    
    /// フレーム終了時に呼ぶ
    pub fn end_frame(&mut self) {
        self.frame_count += 1;
        
        // 1秒ごとにサンプリング
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample);
        if elapsed.as_millis() >= SAMPLE_INTERVAL_MS as u128 {
            // FPS計算
            self.fps = self.frame_count as f64 / elapsed.as_secs_f64();
            
            // CPU速度計算（1.023MHz基準）
            let expected_cycles = 1_023_000.0 * elapsed.as_secs_f64();
            self.cpu_mhz = (self.cpu_info.total_cycles as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
            
            // サンプル履歴に追加
            if self.sample_history.len() >= 10 {
                self.sample_history.remove(0);
            }
            self.sample_history.push(self.stats.clone());
            
            // リセット
            for stat in self.stats.values_mut() {
                stat.reset();
            }
            self.cpu_info.total_cycles = 0;
            self.frame_count = 0;
            self.last_sample = now;
        }
    }
    
    /// 統計を取得
    pub fn get_stats(&self, category: ProfileCategory) -> Option<&ProfileStats> {
        self.stats.get(&category)
    }
    
    /// 全統計を取得
    pub fn all_stats(&self) -> &HashMap<ProfileCategory, ProfileStats> {
        &self.stats
    }
    
    /// プロファイル情報を文字列で取得
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        
        lines.push(format!("=== Profiler Summary ==="));
        lines.push(format!("FPS: {:.1}  CPU: {:.2} MHz", self.fps, self.cpu_mhz));
        lines.push(format!("Boot Stage: {:?}", self.boot_stage));
        
        if let Some(elapsed) = self.boot_elapsed() {
            lines.push(format!("Boot Time: {:.2}s", elapsed.as_secs_f64()));
        }
        
        lines.push(format!("\n--- Timing ---"));
        for (cat, stat) in &self.stats {
            if stat.call_count > 0 {
                lines.push(format!(
                    "{}: {:.2}ms total, {} calls, {:.2}us avg",
                    cat.name(),
                    stat.total_time.as_secs_f64() * 1000.0,
                    stat.call_count,
                    stat.average().as_secs_f64() * 1_000_000.0
                ));
            }
        }
        
        lines.push(format!("\n--- Disk ---"));
        lines.push(format!("Nibbles Read: {}", self.disk_info.nibbles_read));
        lines.push(format!("Sectors Read: {} (Failed: {})", 
            self.disk_info.sectors_read, self.disk_info.sectors_failed));
        lines.push(format!("Current Track: {}", self.disk_info.current_track));
        
        lines.push(format!("\n--- CPU ---"));
        lines.push(format!("Instructions: {}", self.cpu_info.instructions_executed));
        lines.push(format!("Last PC: ${:04X}", self.cpu_info.last_pc));
        
        lines.join("\n")
    }
    
    /// 詳細なプロファイルレポートを生成
    pub fn detailed_report(&self) -> String {
        let mut lines = Vec::new();
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        
        lines.push(format!("================================================================================"));
        lines.push(format!("A2RS Profile Report"));
        lines.push(format!("Generated: {}", timestamp));
        lines.push(format!("================================================================================"));
        lines.push(String::new());
        
        // 基本情報
        lines.push(format!("## Performance"));
        lines.push(format!("FPS: {:.2}", self.fps));
        lines.push(format!("CPU Speed: {:.3} MHz (target: 1.023 MHz)", self.cpu_mhz));
        lines.push(format!("Speed Ratio: {:.1}x", self.cpu_mhz / 1.023));
        lines.push(String::new());
        
        // ブート情報
        lines.push(format!("## Boot Status"));
        lines.push(format!("Current Stage: {:?}", self.boot_stage));
        if let Some(elapsed) = self.boot_elapsed() {
            lines.push(format!("Total Boot Time: {:.3}s", elapsed.as_secs_f64()));
        }
        for (stage, duration) in &self.boot_timings {
            lines.push(format!("  {:?}: {:.3}s", stage, duration.as_secs_f64()));
        }
        lines.push(String::new());
        
        // タイミング統計
        lines.push(format!("## Timing Statistics"));
        lines.push(format!("{:<20} {:>12} {:>12} {:>12} {:>12}", 
            "Category", "Total(ms)", "Calls", "Avg(us)", "Max(us)"));
        lines.push(format!("{:-<70}", ""));
        
        let mut sorted_stats: Vec<_> = self.stats.iter().collect();
        sorted_stats.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
        
        for (cat, stat) in sorted_stats {
            if stat.call_count > 0 {
                let max_us = stat.max_time.map(|d| d.as_secs_f64() * 1_000_000.0).unwrap_or(0.0);
                lines.push(format!("{:<20} {:>12.2} {:>12} {:>12.2} {:>12.2}",
                    cat.name(),
                    stat.total_time.as_secs_f64() * 1000.0,
                    stat.call_count,
                    stat.average().as_secs_f64() * 1_000_000.0,
                    max_us
                ));
            }
        }
        lines.push(String::new());
        
        // ディスク統計
        lines.push(format!("## Disk I/O Statistics"));
        lines.push(format!("Total Nibbles Read: {}", self.disk_info.nibbles_read));
        lines.push(format!("Sectors Read: {} (Success) / {} (Failed)", 
            self.disk_info.sectors_read, self.disk_info.sectors_failed));
        lines.push(format!("Address Marks Found: {}", self.disk_info.address_marks_found));
        lines.push(format!("Data Marks Found: {}", self.disk_info.data_marks_found));
        lines.push(format!("Current Track: {}", self.disk_info.current_track));
        lines.push(format!("Last I/O: ${:04X} = ${:02X}", 
            self.disk_info.last_io_address, self.disk_info.last_io_value));
        lines.push(String::new());
        
        // トラックアクセスヒートマップ
        lines.push(format!("## Track Access Heatmap"));
        lines.push(format!("Track: {}", (0..35).map(|t| format!("{:>5}", t)).collect::<Vec<_>>().join("")));
        lines.push(format!("Count: {}", self.disk_info.track_accesses.iter()
            .map(|&c| format!("{:>5}", c)).collect::<Vec<_>>().join("")));
        lines.push(String::new());
        
        // CPU統計
        lines.push(format!("## CPU Statistics"));
        lines.push(format!("Total Instructions: {}", self.cpu_info.instructions_executed));
        lines.push(format!("Last PC: ${:04X}", self.cpu_info.last_pc));
        lines.push(String::new());
        
        // ホットオプコード
        lines.push(format!("## Hot Opcodes (Top 20)"));
        lines.push(format!("{:<10} {:<8} {:>12} {:>10}", "Opcode", "Name", "Count", "Percent"));
        lines.push(format!("{:-<45}", ""));
        
        let hot = self.hot_opcodes(20);
        let total_ops = self.cpu_info.instructions_executed.max(1) as f64;
        for (opcode, count) in hot {
            let percent = (count as f64 / total_ops) * 100.0;
            lines.push(format!("${:02X}       {:<8} {:>12} {:>9.2}%", 
                opcode, opcode_name(opcode), count, percent));
        }
        lines.push(String::new());
        
        lines.push(format!("================================================================================"));
        
        lines.join("\n")
    }
    
    /// プロファイルデータをファイルに出力
    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let report = self.detailed_report();
        let mut file = std::fs::File::create(path)?;
        file.write_all(report.as_bytes())?;
        Ok(())
    }
    
    /// プロファイルデータをCSV形式で出力
    pub fn write_csv(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;
        
        // ヘッダー
        writeln!(file, "timestamp,fps,cpu_mhz,boot_stage,boot_time_s,nibbles_read,sectors_read,sectors_failed,instructions,last_pc")?;
        
        // データ
        let boot_time = self.boot_elapsed().map(|d| d.as_secs_f64()).unwrap_or(0.0);
        writeln!(file, "{},{:.2},{:.3},{:?},{:.3},{},{},{},{},{:#06X}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            self.fps,
            self.cpu_mhz,
            self.boot_stage,
            boot_time,
            self.disk_info.nibbles_read,
            self.disk_info.sectors_read,
            self.disk_info.sectors_failed,
            self.cpu_info.instructions_executed,
            self.cpu_info.last_pc
        )?;
        
        Ok(())
    }
    
    /// プロファイルデータをJSON形式で出力
    pub fn to_json(&self) -> String {
        let boot_time = self.boot_elapsed().map(|d| d.as_secs_f64()).unwrap_or(0.0);
        
        format!(r#"{{
  "timestamp": "{}",
  "performance": {{
    "fps": {:.2},
    "cpu_mhz": {:.3},
    "speed_ratio": {:.2}
  }},
  "boot": {{
    "stage": "{:?}",
    "time_seconds": {:.3}
  }},
  "disk": {{
    "nibbles_read": {},
    "sectors_read": {},
    "sectors_failed": {},
    "current_track": {},
    "track_accesses": {:?}
  }},
  "cpu": {{
    "instructions": {},
    "last_pc": "{:#06X}"
  }}
}}"#,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            self.fps,
            self.cpu_mhz,
            self.cpu_mhz / 1.023,
            self.boot_stage,
            boot_time,
            self.disk_info.nibbles_read,
            self.disk_info.sectors_read,
            self.disk_info.sectors_failed,
            self.disk_info.current_track,
            &self.disk_info.track_accesses[..],
            self.cpu_info.instructions_executed,
            self.cpu_info.last_pc
        )
    }
    
    /// JSON形式でファイルに出力
    pub fn write_json(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let json = self.to_json();
        let mut file = std::fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
    
    /// デバッグ用: ホットスポットオプコードを取得
    pub fn hot_opcodes(&self, top_n: usize) -> Vec<(u8, u32)> {
        let mut opcodes: Vec<(u8, u32)> = self.cpu_info.opcode_counts
            .iter()
            .enumerate()
            .map(|(i, &count)| (i as u8, count))
            .filter(|(_, count)| *count > 0)
            .collect();
        
        opcodes.sort_by(|a, b| b.1.cmp(&a.1));
        opcodes.truncate(top_n);
        opcodes
    }
    
    /// リセット
    pub fn reset(&mut self) {
        self.stats.clear();
        self.current_start = None;
        self.boot_stage = BootStage::Init;
        self.boot_start = None;
        self.boot_timings.clear();
        self.disk_info = DiskAccessInfo::default();
        self.cpu_info = CpuProfileInfo::default();
        self.sample_history.clear();
        self.frame_count = 0;
        self.fps = 0.0;
        self.cpu_mhz = 0.0;
        self.last_sample = Instant::now();
    }
}

/// グローバルプロファイラ（スレッドローカル）
thread_local! {
    pub static PROFILER: std::cell::RefCell<Profiler> = std::cell::RefCell::new(Profiler::new());
}

/// プロファイリングマクロ
#[macro_export]
macro_rules! profile_scope {
    ($category:expr) => {
        let _guard = $crate::profiler::ProfileGuard::new($category);
    };
}

/// RAIIスタイルのプロファイルガード
pub struct ProfileGuard {
    category: ProfileCategory,
    start: Instant,
}

impl ProfileGuard {
    pub fn new(category: ProfileCategory) -> Self {
        ProfileGuard {
            category,
            start: Instant::now(),
        }
    }
}

impl Drop for ProfileGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        PROFILER.with(|p| {
            p.borrow_mut().record(self.category, duration);
        });
    }
}

/// デバッガ状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebuggerState {
    /// 通常実行
    Running,
    /// 一時停止
    Paused,
    /// ステップ実行
    Stepping,
    /// ブレークポイントでヒット
    BreakpointHit,
}

/// ブレークポイント
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// ID
    pub id: u32,
    /// アドレス
    pub address: u16,
    /// 有効フラグ
    pub enabled: bool,
    /// ヒット回数
    pub hit_count: u32,
    /// 条件（オプション）
    pub condition: Option<BreakCondition>,
}

/// ブレークポイント条件
#[derive(Debug, Clone)]
pub enum BreakCondition {
    /// Aレジスタが特定値
    AEquals(u8),
    /// Xレジスタが特定値
    XEquals(u8),
    /// Yレジスタが特定値
    YEquals(u8),
    /// メモリが特定値
    MemEquals(u16, u8),
    /// ヒット回数が特定値以上
    HitCount(u32),
}

/// デバッガ
pub struct Debugger {
    /// 状態
    pub state: DebuggerState,
    /// ブレークポイント
    breakpoints: Vec<Breakpoint>,
    /// 次のブレークポイントID
    next_bp_id: u32,
    /// ステップオーバーの戻りアドレス
    step_over_return: Option<u16>,
    /// トレースログ有効
    pub trace_enabled: bool,
    /// トレースログバッファ
    trace_buffer: Vec<String>,
    /// トレースバッファサイズ上限
    trace_buffer_limit: usize,
    /// ウォッチポイント（メモリアドレス）
    watchpoints: Vec<(u16, u8)>, // (address, last_value)
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            state: DebuggerState::Running,
            breakpoints: Vec::new(),
            next_bp_id: 1,
            step_over_return: None,
            trace_enabled: false,
            trace_buffer: Vec::new(),
            trace_buffer_limit: 10000,
            watchpoints: Vec::new(),
        }
    }
    
    /// ブレークポイントを追加
    pub fn add_breakpoint(&mut self, address: u16) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        
        self.breakpoints.push(Breakpoint {
            id,
            address,
            enabled: true,
            hit_count: 0,
            condition: None,
        });
        
        id
    }
    
    /// 条件付きブレークポイントを追加
    pub fn add_conditional_breakpoint(&mut self, address: u16, condition: BreakCondition) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        
        self.breakpoints.push(Breakpoint {
            id,
            address,
            enabled: true,
            hit_count: 0,
            condition: Some(condition),
        });
        
        id
    }
    
    /// ブレークポイントを削除
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(pos) = self.breakpoints.iter().position(|bp| bp.id == id) {
            self.breakpoints.remove(pos);
            true
        } else {
            false
        }
    }
    
    /// ブレークポイントの有効/無効を切り替え
    pub fn toggle_breakpoint(&mut self, id: u32) -> bool {
        if let Some(bp) = self.breakpoints.iter_mut().find(|bp| bp.id == id) {
            bp.enabled = !bp.enabled;
            true
        } else {
            false
        }
    }
    
    /// アドレスにブレークポイントがあるかチェック
    pub fn check_breakpoint(&mut self, address: u16, a: u8, x: u8, y: u8, memory: &[u8]) -> bool {
        for bp in &mut self.breakpoints {
            if bp.enabled && bp.address == address {
                // 条件チェック
                let condition_met = match &bp.condition {
                    None => true,
                    Some(BreakCondition::AEquals(v)) => a == *v,
                    Some(BreakCondition::XEquals(v)) => x == *v,
                    Some(BreakCondition::YEquals(v)) => y == *v,
                    Some(BreakCondition::MemEquals(addr, v)) => {
                        memory.get(*addr as usize).copied() == Some(*v)
                    }
                    Some(BreakCondition::HitCount(n)) => bp.hit_count >= *n,
                };
                
                if condition_met {
                    bp.hit_count += 1;
                    return true;
                }
            }
        }
        false
    }
    
    /// 全ブレークポイントを取得
    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }
    
    /// トレースログを追加
    pub fn add_trace(&mut self, entry: String) {
        if self.trace_enabled {
            self.trace_buffer.push(entry);
            if self.trace_buffer.len() > self.trace_buffer_limit {
                self.trace_buffer.remove(0);
            }
        }
    }
    
    /// トレースログを取得
    pub fn get_trace(&self, last_n: usize) -> &[String] {
        let start = self.trace_buffer.len().saturating_sub(last_n);
        &self.trace_buffer[start..]
    }
    
    /// トレースログをクリア
    pub fn clear_trace(&mut self) {
        self.trace_buffer.clear();
    }
    
    /// ウォッチポイントを追加
    pub fn add_watchpoint(&mut self, address: u16, initial_value: u8) {
        self.watchpoints.push((address, initial_value));
    }
    
    /// ウォッチポイントをチェック（値が変わったらtrue）
    pub fn check_watchpoints(&mut self, memory: &[u8]) -> Option<(u16, u8, u8)> {
        for (addr, last_val) in &mut self.watchpoints {
            if let Some(&current) = memory.get(*addr as usize) {
                if current != *last_val {
                    let old = *last_val;
                    *last_val = current;
                    return Some((*addr, old, current));
                }
            }
        }
        None
    }
    
    /// 一時停止
    pub fn pause(&mut self) {
        self.state = DebuggerState::Paused;
    }
    
    /// 再開
    pub fn resume(&mut self) {
        self.state = DebuggerState::Running;
    }
    
    /// ステップ実行
    pub fn step(&mut self) {
        self.state = DebuggerState::Stepping;
    }
    
    /// ステップ完了後に停止
    pub fn step_complete(&mut self) {
        self.state = DebuggerState::Paused;
    }
    
    /// リセット
    pub fn reset(&mut self) {
        self.state = DebuggerState::Running;
        self.step_over_return = None;
        self.trace_buffer.clear();
        for bp in &mut self.breakpoints {
            bp.hit_count = 0;
        }
    }
}

/// オプコード名を取得（デバッグ用）
pub fn opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "BRK", 0x01 => "ORA", 0x05 => "ORA", 0x06 => "ASL",
        0x08 => "PHP", 0x09 => "ORA", 0x0A => "ASL", 0x0D => "ORA",
        0x0E => "ASL", 0x10 => "BPL", 0x11 => "ORA", 0x15 => "ORA",
        0x16 => "ASL", 0x18 => "CLC", 0x19 => "ORA", 0x1D => "ORA",
        0x1E => "ASL", 0x20 => "JSR", 0x21 => "AND", 0x24 => "BIT",
        0x25 => "AND", 0x26 => "ROL", 0x28 => "PLP", 0x29 => "AND",
        0x2A => "ROL", 0x2C => "BIT", 0x2D => "AND", 0x2E => "ROL",
        0x30 => "BMI", 0x31 => "AND", 0x35 => "AND", 0x36 => "ROL",
        0x38 => "SEC", 0x39 => "AND", 0x3D => "AND", 0x3E => "ROL",
        0x40 => "RTI", 0x41 => "EOR", 0x45 => "EOR", 0x46 => "LSR",
        0x48 => "PHA", 0x49 => "EOR", 0x4A => "LSR", 0x4C => "JMP",
        0x4D => "EOR", 0x4E => "LSR", 0x50 => "BVC", 0x51 => "EOR",
        0x55 => "EOR", 0x56 => "LSR", 0x58 => "CLI", 0x59 => "EOR",
        0x5D => "EOR", 0x5E => "LSR", 0x60 => "RTS", 0x61 => "ADC",
        0x65 => "ADC", 0x66 => "ROR", 0x68 => "PLA", 0x69 => "ADC",
        0x6A => "ROR", 0x6C => "JMP", 0x6D => "ADC", 0x6E => "ROR",
        0x70 => "BVS", 0x71 => "ADC", 0x75 => "ADC", 0x76 => "ROR",
        0x78 => "SEI", 0x79 => "ADC", 0x7D => "ADC", 0x7E => "ROR",
        0x81 => "STA", 0x84 => "STY", 0x85 => "STA", 0x86 => "STX",
        0x88 => "DEY", 0x8A => "TXA", 0x8C => "STY", 0x8D => "STA",
        0x8E => "STX", 0x90 => "BCC", 0x91 => "STA", 0x94 => "STY",
        0x95 => "STA", 0x96 => "STX", 0x98 => "TYA", 0x99 => "STA",
        0x9A => "TXS", 0x9D => "STA", 0xA0 => "LDY", 0xA1 => "LDA",
        0xA2 => "LDX", 0xA4 => "LDY", 0xA5 => "LDA", 0xA6 => "LDX",
        0xA8 => "TAY", 0xA9 => "LDA", 0xAA => "TAX", 0xAC => "LDY",
        0xAD => "LDA", 0xAE => "LDX", 0xB0 => "BCS", 0xB1 => "LDA",
        0xB4 => "LDY", 0xB5 => "LDA", 0xB6 => "LDX", 0xB8 => "CLV",
        0xB9 => "LDA", 0xBA => "TSX", 0xBC => "LDY", 0xBD => "LDA",
        0xBE => "LDX", 0xC0 => "CPY", 0xC1 => "CMP", 0xC4 => "CPY",
        0xC5 => "CMP", 0xC6 => "DEC", 0xC8 => "INY", 0xC9 => "CMP",
        0xCA => "DEX", 0xCC => "CPY", 0xCD => "CMP", 0xCE => "DEC",
        0xD0 => "BNE", 0xD1 => "CMP", 0xD5 => "CMP", 0xD6 => "DEC",
        0xD8 => "CLD", 0xD9 => "CMP", 0xDD => "CMP", 0xDE => "DEC",
        0xE0 => "CPX", 0xE1 => "SBC", 0xE4 => "CPX", 0xE5 => "SBC",
        0xE6 => "INC", 0xE8 => "INX", 0xE9 => "SBC", 0xEA => "NOP",
        0xEC => "CPX", 0xED => "SBC", 0xEE => "INC", 0xF0 => "BEQ",
        0xF1 => "SBC", 0xF5 => "SBC", 0xF6 => "INC", 0xF8 => "SED",
        0xF9 => "SBC", 0xFD => "SBC", 0xFE => "INC",
        _ => "???",
    }
}
