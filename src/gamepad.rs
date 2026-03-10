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

use crate::config::GamepadConfig;

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

#[cfg(feature = "gamepad")]
use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs};
#[cfg(feature = "gamepad")]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "gamepad")]
pub struct GamepadManager {
    gilrs: Gilrs,
    state: GamepadState,
    active_gamepad: Option<GamepadId>,
    last_event: String,
    raw_axes: BTreeMap<u32, f32>,
    raw_buttons_down: BTreeSet<u32>,
    config: GamepadConfig,
}

#[cfg(feature = "gamepad")]
impl GamepadManager {
    pub fn new(config: GamepadConfig) -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|e| format!("Failed to initialize gamepad: {}", e))?;

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

        let mut manager = GamepadManager {
            gilrs,
            state: GamepadState::default(),
            active_gamepad,
            last_event: "(none)".to_string(),
            raw_axes: BTreeMap::new(),
            raw_buttons_down: BTreeSet::new(),
            config,
        };
        manager.refresh_snapshot();
        Ok(manager)
    }

    /// イベントを処理して状態を更新
    pub fn update(&mut self) {
        while let Some(Event { id, event, .. }) = self.gilrs.next_event() {
            if self.active_gamepad.is_none() {
                if let EventType::Connected = event {
                    self.active_gamepad = Some(id);
                    self.state.connected = true;
                    println!("Gamepad connected: {:?}", id);
                }
            }

            if Some(id) != self.active_gamepad {
                continue;
            }

            self.last_event = describe_event(&event);

            match event {
                EventType::ButtonPressed(button, code) => {
                    self.handle_button(button, true);
                    self.handle_raw_button(button, true, &code);
                }
                EventType::ButtonReleased(button, code) => {
                    self.handle_button(button, false);
                    self.handle_raw_button(button, false, &code);
                }
                EventType::ButtonChanged(button, value, code) => {
                    self.handle_button(button, value >= 0.5);
                    self.handle_raw_button(button, value >= 0.5, &code);
                }
                EventType::AxisChanged(axis, value, code) => {
                    self.handle_axis(axis, value);
                    self.handle_raw_axis(axis, value, &code);
                }
                EventType::Connected => self.state.connected = true,
                EventType::Disconnected => {
                    self.state.connected = false;
                    self.active_gamepad = None;
                    self.state = GamepadState::default();
                    self.raw_axes.clear();
                    self.raw_buttons_down.clear();
                    println!("Gamepad disconnected");
                }
                _ => {}
            }
        }

        // Linux の汎用USBパッドではイベントマッピングが不完全な場合があるため、
        // 毎フレームのスナップショットからも状態を再構築する。
        self.refresh_snapshot();
    }

    fn handle_button(&mut self, button: Button, pressed: bool) {
        match button {
            Button::South => self.state.button_a = pressed,
            Button::East => self.state.button_b = pressed,
            Button::West => self.state.button_x = pressed,
            Button::North => self.state.button_y = pressed,
            Button::LeftTrigger | Button::LeftTrigger2 => self.state.button_lb = pressed,
            Button::RightTrigger | Button::RightTrigger2 => self.state.button_rb = pressed,
            Button::Start | Button::Mode => self.state.button_start = pressed,
            Button::Select => self.state.button_select = pressed,
            Button::DPadLeft => self.state.dpad_left = pressed,
            Button::DPadRight => self.state.dpad_right = pressed,
            Button::DPadUp => self.state.dpad_up = pressed,
            Button::DPadDown => self.state.dpad_down = pressed,
            _ => {}
        }
    }

    fn handle_axis(&mut self, axis: Axis, value: f32) {
        let value = apply_deadzone(value, self.config.deadzone);
        match axis {
            Axis::LeftStickX => self.state.left_x = value,
            Axis::LeftStickY => self.state.left_y = -value,
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

    fn handle_raw_button<T: std::fmt::Debug>(&mut self, button: Button, pressed: bool, code: &T) {
        if !matches!(button, Button::Unknown) {
            return;
        }
        let Some(raw_code) = extract_raw_code(code) else {
            return;
        };
        if pressed {
            self.raw_buttons_down.insert(raw_code);
        } else {
            self.raw_buttons_down.remove(&raw_code);
        }
        self.apply_linux_raw_button_mapping();
    }

    fn handle_raw_axis<T: std::fmt::Debug>(&mut self, axis: Axis, value: f32, code: &T) {
        let raw_code = extract_raw_code(code).or_else(|| match axis {
            Axis::Unknown => None,
            Axis::LeftStickX => Some(0),
            Axis::LeftStickY => Some(1),
            Axis::RightStickX => Some(3),
            Axis::RightStickY => Some(4),
            Axis::DPadX => Some(16),
            Axis::DPadY => Some(17),
            _ => None,
        });
        let Some(raw_code) = raw_code else {
            return;
        };
        self.raw_axes.insert(raw_code, value);
        self.apply_linux_raw_axis_mapping();
    }

    fn apply_linux_raw_button_mapping(&mut self) {
        self.state.button_a = self.state.button_a
            || self
                .config
                .raw_button_a_codes
                .iter()
                .any(|code| self.raw_buttons_down.contains(code));
        self.state.button_b = self.state.button_b
            || self
                .config
                .raw_button_b_codes
                .iter()
                .any(|code| self.raw_buttons_down.contains(code));
    }

    fn apply_linux_raw_axis_mapping(&mut self) {
        if let Some(v) = self.raw_axes.get(&self.config.raw_axis_x_code).copied() {
            self.state.left_x = apply_deadzone(v, self.config.deadzone);
        }
        if let Some(v) = self.raw_axes.get(&self.config.raw_axis_y_code).copied() {
            self.state.left_y = -apply_deadzone(v, self.config.deadzone);
        }
        if let Some(v) = self.raw_axes.get(&self.config.raw_hat_x_code).copied() {
            self.state.dpad_left = v < -0.5;
            self.state.dpad_right = v > 0.5;
        }
        if let Some(v) = self.raw_axes.get(&self.config.raw_hat_y_code).copied() {
            self.state.dpad_up = v < -0.5;
            self.state.dpad_down = v > 0.5;
        }
    }

    fn refresh_snapshot(&mut self) {
        let Some(id) = self.active_gamepad else {
            self.state.connected = false;
            return;
        };

        let gamepad = self.gilrs.gamepad(id);
        if !gamepad.is_connected() {
            self.state = GamepadState::default();
            self.active_gamepad = None;
            return;
        }

        self.state.connected = true;

        let lx = axis_value(&gamepad, &[Axis::LeftStickX, Axis::RightStickX], self.config.deadzone);
        let ly = axis_value(&gamepad, &[Axis::LeftStickY, Axis::RightStickY], self.config.deadzone);
        let rx = axis_value(&gamepad, &[Axis::RightStickX, Axis::LeftStickX], self.config.deadzone);
        let ry = axis_value(&gamepad, &[Axis::RightStickY, Axis::LeftStickY], self.config.deadzone);
        self.state.left_x = lx;
        self.state.left_y = -ly;
        self.state.right_x = rx;
        self.state.right_y = -ry;

        let dpad_x = axis_value(&gamepad, &[Axis::DPadX, Axis::LeftZ, Axis::RightZ], self.config.deadzone);
        let dpad_y = axis_value(&gamepad, &[Axis::DPadY], self.config.deadzone);
        self.state.dpad_left = gamepad.is_pressed(Button::DPadLeft) || dpad_x < -0.5;
        self.state.dpad_right = gamepad.is_pressed(Button::DPadRight) || dpad_x > 0.5;
        self.state.dpad_up = gamepad.is_pressed(Button::DPadUp) || dpad_y < -0.5;
        self.state.dpad_down = gamepad.is_pressed(Button::DPadDown) || dpad_y > 0.5;

        self.state.button_a = any_pressed_named(&gamepad, &self.config.button_a_names);
        self.state.button_b = any_pressed_named(&gamepad, &self.config.button_b_names);
        self.state.button_x = any_pressed_named(&gamepad, &self.config.button_x_names);
        self.state.button_y = any_pressed_named(&gamepad, &self.config.button_y_names);
        self.state.button_lb = any_pressed_named(&gamepad, &self.config.button_lb_names);
        self.state.button_rb = any_pressed_named(&gamepad, &self.config.button_rb_names);
        self.state.button_start = any_pressed_named(&gamepad, &self.config.button_start_names);
        self.state.button_select = any_pressed_named(&gamepad, &self.config.button_select_names);

        self.apply_linux_raw_axis_mapping();
        self.apply_linux_raw_button_mapping();
    }

    pub fn state(&self) -> &GamepadState {
        &self.state
    }

    pub fn is_connected(&self) -> bool {
        self.state.connected && self.active_gamepad.is_some()
    }

    pub fn debug_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("GP: {}", if self.is_connected() { "connected" } else { "disconnected" }));
        lines.push(format!("LX:{:+.2}  LY:{:+.2}", self.state.left_x, self.state.left_y));
        lines.push(format!("RX:{:+.2}  RY:{:+.2}", self.state.right_x, self.state.right_y));
        lines.push(format!("DPAD L:{} R:{} U:{} D:{}", b(self.state.dpad_left), b(self.state.dpad_right), b(self.state.dpad_up), b(self.state.dpad_down)));
        lines.push(format!("BTN  A:{} B:{} X:{} Y:{}", b(self.state.button_a), b(self.state.button_b), b(self.state.button_x), b(self.state.button_y)));
        lines.push(format!("SHLD LB:{} RB:{} ST:{} SE:{}", b(self.state.button_lb), b(self.state.button_rb), b(self.state.button_start), b(self.state.button_select)));
        lines.push(format!("LAST {}", self.last_event));

        if let Some(id) = self.active_gamepad {
            let gamepad = self.gilrs.gamepad(id);
            let mut axes = Vec::new();
            for axis in [Axis::LeftStickX, Axis::LeftStickY, Axis::RightStickX, Axis::RightStickY, Axis::DPadX, Axis::DPadY, Axis::LeftZ, Axis::RightZ] {
                let v = gamepad.value(axis);
                if v.abs() >= 0.01 {
                    axes.push(format!("{:?}:{:+.2}", axis, v));
                }
            }
            lines.push(if axes.is_empty() {
                "AXES (no movement)".to_string()
            } else {
                format!("AXES {}", axes.join(" "))
            });

            let mut buttons = Vec::new();
            for button in [Button::South, Button::East, Button::West, Button::North, Button::LeftTrigger, Button::LeftTrigger2, Button::RightTrigger, Button::RightTrigger2, Button::Select, Button::Start, Button::Mode, Button::DPadLeft, Button::DPadRight, Button::DPadUp, Button::DPadDown, Button::C, Button::Z] {
                if gamepad.is_pressed(button) {
                    buttons.push(format!("{:?}", button));
                }
            }
            lines.push(if buttons.is_empty() {
                "BTNS (none pressed)".to_string()
            } else {
                format!("BTNS {}", buttons.join(","))
            });
        }

        if self.raw_axes.is_empty() {
            lines.push("RAW ABS (none)".to_string());
        } else {
            let mut items = Vec::new();
            for (k, v) in &self.raw_axes {
                items.push(format!("{}:{:+.2}", k, v));
            }
            lines.push(format!("RAW ABS {}", items.join(" ")));
        }

        if self.raw_buttons_down.is_empty() {
            lines.push("RAW KEYS (none pressed)".to_string());
        } else {
            let mut items = Vec::new();
            for k in &self.raw_buttons_down {
                items.push(k.to_string());
            }
            lines.push(format!("RAW KEYS {}", items.join(",")));
        }

        lines
    }
}

#[cfg(feature = "gamepad")]
fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
    if value.abs() < deadzone { 0.0 } else { value }
}

#[cfg(feature = "gamepad")]
fn axis_value(gamepad: &gilrs::Gamepad<'_>, axes: &[Axis], deadzone: f32) -> f32 {
    for axis in axes {
        let value = apply_deadzone(gamepad.value(*axis), deadzone);
        if value.abs() >= 0.01 {
            return value;
        }
    }
    0.0
}

#[cfg(feature = "gamepad")]
fn any_pressed_named(gamepad: &gilrs::Gamepad<'_>, names: &[String]) -> bool {
    names.iter()
        .filter_map(|name| button_from_name(name))
        .any(|button| gamepad.is_pressed(button))
}

#[cfg(feature = "gamepad")]
fn button_from_name(name: &str) -> Option<Button> {
    match name.trim().to_ascii_lowercase().as_str() {
        "south" => Some(Button::South),
        "east" => Some(Button::East),
        "west" => Some(Button::West),
        "north" => Some(Button::North),
        "c" => Some(Button::C),
        "z" => Some(Button::Z),
        "lefttrigger" | "left_trigger" | "lt" => Some(Button::LeftTrigger),
        "lefttrigger2" | "left_trigger2" | "lt2" => Some(Button::LeftTrigger2),
        "righttrigger" | "right_trigger" | "rt" => Some(Button::RightTrigger),
        "righttrigger2" | "right_trigger2" | "rt2" => Some(Button::RightTrigger2),
        "start" => Some(Button::Start),
        "mode" => Some(Button::Mode),
        "select" => Some(Button::Select),
        "dpadleft" | "dpad_left" => Some(Button::DPadLeft),
        "dpadright" | "dpad_right" => Some(Button::DPadRight),
        "dpadup" | "dpad_up" => Some(Button::DPadUp),
        "dpaddown" | "dpad_down" => Some(Button::DPadDown),
        _ => None,
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
    pub fn new(_config: GamepadConfig) -> Result<Self, String> {
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

    pub fn debug_lines(&self) -> Vec<String> {
        vec!["GP: disabled".to_string()]
    }
}

#[cfg(feature = "gamepad")]
fn extract_raw_code<T: std::fmt::Debug>(code: &T) -> Option<u32> {
    let s = format!("{:?}", code);
    if let Some(i) = s.find("code: ") {
        let digits: String = s[i + 6..].chars().take_while(|c| c.is_ascii_digit()).collect();
        return digits.parse().ok();
    }
    None
}

#[cfg(feature = "gamepad")]
fn b(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

#[cfg(feature = "gamepad")]
fn describe_event(event: &EventType) -> String {
    match event {
        EventType::ButtonPressed(button, _) => format!("ButtonPressed({:?})", button),
        EventType::ButtonReleased(button, _) => format!("ButtonReleased({:?})", button),
        EventType::AxisChanged(axis, value, _) => format!("AxisChanged({:?}={:+.2})", axis, value),
        EventType::ButtonChanged(button, value, code) => format!("ButtonChanged({:?},{:+.2},raw={:?})", button, value, code),
        EventType::Connected => "Connected".to_string(),
        EventType::Disconnected => "Disconnected".to_string(),
        other => format!("{:?}", other),
    }
}
