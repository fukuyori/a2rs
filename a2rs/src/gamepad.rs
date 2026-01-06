//! ゲームパッド入力モジュール
//!
//! USB接続のゲームパッド（Tiger3deなど）をサポート
//!
//! ## 有効化方法:
//! 
//! ### Ubuntu/Debian:
//! ```bash
//! sudo apt-get install libudev-dev
//! cargo build --release --features gamepad
//! ```
//!
//! ### macOS/Windows:
//! ```bash
//! cargo build --release --features gamepad
//! ```

/// ゲームパッドの状態
#[derive(Debug, Clone, Default)]
pub struct GamepadState {
    /// 左スティックX軸 (-1.0 to 1.0)
    pub left_x: f32,
    /// 左スティックY軸 (-1.0 to 1.0)
    pub left_y: f32,
    /// 右スティックX軸 (-1.0 to 1.0)
    pub right_x: f32,
    /// 右スティックY軸 (-1.0 to 1.0)
    pub right_y: f32,
    /// Dパッド
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    /// ボタン
    pub button_a: bool,
    pub button_b: bool,
    pub button_x: bool,
    pub button_y: bool,
    pub button_lb: bool,
    pub button_rb: bool,
    pub button_start: bool,
    pub button_select: bool,
    /// 接続状態
    pub connected: bool,
}

// ============================================================
// gilrsが有効な場合の実装
// ============================================================

#[cfg(feature = "gamepad")]
use gilrs::{Gilrs, Button, Axis, Event, EventType};

#[cfg(feature = "gamepad")]
pub struct GamepadManager {
    gilrs: Gilrs,
    state: GamepadState,
    active_gamepad: Option<gilrs::GamepadId>,
}

#[cfg(feature = "gamepad")]
impl GamepadManager {
    pub fn new() -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|e| format!("Failed to initialize gamepad: {}", e))?;
        
        // 接続されているゲームパッドを検出
        let mut active_gamepad = None;
        for (id, gamepad) in gilrs.gamepads() {
            println!("Gamepad detected: {} ({:?})", gamepad.name(), id);
            if active_gamepad.is_none() {
                active_gamepad = Some(id);
            }
        }
        
        if active_gamepad.is_some() {
            println!("Using first detected gamepad");
        } else {
            println!("No gamepad detected (will auto-detect when connected)");
        }
        
        Ok(GamepadManager {
            gilrs,
            state: GamepadState::default(),
            active_gamepad,
        })
    }
    
    /// イベントを処理して状態を更新
    pub fn update(&mut self) {
        while let Some(Event { id, event, .. }) = self.gilrs.next_event() {
            // 新しいゲームパッドが接続された場合
            if self.active_gamepad.is_none() {
                if let EventType::Connected = event {
                    self.active_gamepad = Some(id);
                    self.state.connected = true;
                    println!("Gamepad connected: {:?}", id);
                }
            }
            
            // アクティブなゲームパッドのイベントのみ処理
            if Some(id) != self.active_gamepad {
                continue;
            }
            
            match event {
                EventType::ButtonPressed(button, _) => {
                    self.handle_button(button, true);
                }
                EventType::ButtonReleased(button, _) => {
                    self.handle_button(button, false);
                }
                EventType::AxisChanged(axis, value, _) => {
                    self.handle_axis(axis, value);
                }
                EventType::Connected => {
                    self.state.connected = true;
                }
                EventType::Disconnected => {
                    self.state.connected = false;
                    self.active_gamepad = None;
                    self.state = GamepadState::default();
                    println!("Gamepad disconnected");
                }
                _ => {}
            }
        }
    }
    
    fn handle_button(&mut self, button: Button, pressed: bool) {
        match button {
            Button::South => self.state.button_a = pressed,      // A / Cross
            Button::East => self.state.button_b = pressed,       // B / Circle
            Button::West => self.state.button_x = pressed,       // X / Square
            Button::North => self.state.button_y = pressed,      // Y / Triangle
            Button::LeftTrigger => self.state.button_lb = pressed,
            Button::RightTrigger => self.state.button_rb = pressed,
            Button::Start => self.state.button_start = pressed,
            Button::Select => self.state.button_select = pressed,
            Button::DPadLeft => self.state.dpad_left = pressed,
            Button::DPadRight => self.state.dpad_right = pressed,
            Button::DPadUp => self.state.dpad_up = pressed,
            Button::DPadDown => self.state.dpad_down = pressed,
            _ => {}
        }
    }
    
    fn handle_axis(&mut self, axis: Axis, value: f32) {
        // デッドゾーン処理
        let value = if value.abs() < 0.15 { 0.0 } else { value };
        
        match axis {
            Axis::LeftStickX => self.state.left_x = value,
            Axis::LeftStickY => self.state.left_y = -value, // Y軸は反転
            Axis::RightStickX => self.state.right_x = value,
            Axis::RightStickY => self.state.right_y = -value,
            Axis::DPadX => {
                self.state.dpad_left = value < -0.5;
                self.state.dpad_right = value > 0.5;
            }
            Axis::DPadY => {
                self.state.dpad_up = value < -0.5;
                self.state.dpad_down = value > 0.5;
            }
            _ => {}
        }
    }
    
    /// 現在の状態を取得
    pub fn state(&self) -> &GamepadState {
        &self.state
    }
    
    /// ゲームパッドが接続されているか
    pub fn is_connected(&self) -> bool {
        self.active_gamepad.is_some()
    }
}

// ============================================================
// スタブ実装（gilrsが無効な場合）
// ============================================================

#[cfg(not(feature = "gamepad"))]
pub struct GamepadManager {
    state: GamepadState,
}

#[cfg(not(feature = "gamepad"))]
impl GamepadManager {
    pub fn new() -> Result<Self, String> {
        Ok(GamepadManager {
            state: GamepadState::default(),
        })
    }
    
    pub fn update(&mut self) {
        // スタブ: 何もしない
    }
    
    pub fn state(&self) -> &GamepadState {
        &self.state
    }
    
    pub fn is_connected(&self) -> bool {
        false
    }
}
