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

use crate::config::{GamepadButtonInput, GamepadConfig};

#[cfg(feature = "gamepad")]
use std::thread;
#[cfg(feature = "gamepad")]
use std::time::{Duration, Instant};

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
    /// 接続状態
    pub connected: bool,
}

#[cfg(feature = "gamepad")]
use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs};
#[cfg(feature = "gamepad")]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "gamepad")]
#[derive(Debug, Clone)]
struct TrackedGamepad {
    id: GamepadId,
    name: String,
    profile_name: Option<String>,
    config: GamepadConfig,
    state: GamepadState,
    last_event: String,
    raw_axes: BTreeMap<u32, f32>,
    raw_buttons_down: BTreeSet<u32>,
    activity_tick: u64,
}

#[cfg(feature = "gamepad")]
pub struct GamepadManager {
    gilrs: Gilrs,
    base_config: GamepadConfig,
    devices: Vec<TrackedGamepad>,
    merged_state: GamepadState,
    primary_gamepad: Option<GamepadId>,
    activity_clock: u64,
}

#[cfg(feature = "gamepad")]
impl GamepadManager {
    pub fn new(config: GamepadConfig) -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|e| format!("Failed to initialize gamepad: {}", e))?;

        let mut manager = GamepadManager {
            gilrs,
            base_config: config,
            devices: Vec::new(),
            merged_state: GamepadState::default(),
            primary_gamepad: None,
            activity_clock: 1,
        };

        let detected: Vec<GamepadId> = manager.gilrs.gamepads().map(|(id, _)| id).collect();
        for id in detected {
            manager.register_device(id, true);
        }

        if manager.devices.is_empty() {
            println!("No gamepad detected (will auto-detect when connected)");
        }

        manager.refresh_all_snapshots();
        manager.recompute_merged_state();
        Ok(manager)
    }

    /// イベントを処理して状態を更新
    pub fn update(&mut self) {
        while let Some(Event { id, event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::Connected => {
                    self.register_device(id, false);
                    println!("Gamepad connected: {} ({:?})", self.gilrs.gamepad(id).name(), id);
                }
                EventType::Disconnected => {
                    self.unregister_device(id);
                    println!("Gamepad disconnected: {:?}", id);
                    continue;
                }
                EventType::ButtonPressed(button, code) => {
                    self.mark_activity(id);
                    self.update_last_event(id, describe_event(&EventType::ButtonPressed(button, code)));
                    self.handle_raw_button(id, button, true, &code);
                }
                EventType::ButtonReleased(button, code) => {
                    self.mark_activity(id);
                    self.update_last_event(id, describe_event(&EventType::ButtonReleased(button, code)));
                    self.handle_raw_button(id, button, false, &code);
                }
                EventType::ButtonChanged(button, value, code) => {
                    if value.abs() >= 0.5 {
                        self.mark_activity(id);
                    }
                    self.update_last_event(id, describe_event(&EventType::ButtonChanged(button, value, code)));
                    self.handle_raw_button(id, button, value >= 0.5, &code);
                }
                EventType::AxisChanged(axis, value, code) => {
                    if value.abs() >= 0.1 {
                        self.mark_activity(id);
                    }
                    self.update_last_event(id, describe_event(&EventType::AxisChanged(axis, value, code)));
                    self.handle_raw_axis(id, axis, value, &code);
                }
                other => {
                    self.update_last_event(id, describe_event(&other));
                }
            }
        }

        self.refresh_all_snapshots();
        self.recompute_merged_state();
    }

    pub fn state(&self) -> &GamepadState {
        &self.merged_state
    }

    pub fn is_connected(&self) -> bool {
        self.devices.iter().any(|d| d.state.connected)
    }

    pub fn active_gamepad_name(&self) -> Option<&str> {
        self.primary_device().map(|d| d.name.as_str())
    }

    pub fn active_profile_name(&self) -> Option<&str> {
        self.primary_device().and_then(|d| d.profile_name.as_deref())
    }

    pub fn debug_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "GP: {} ({})",
            if self.is_connected() { "connected" } else { "disconnected" },
            self.devices.len()
        ));
        lines.push(format!(
            "LX:{:+.2}  LY:{:+.2}",
            self.merged_state.left_x, self.merged_state.left_y
        ));
        lines.push(format!(
            "RX:{:+.2}  RY:{:+.2}",
            self.merged_state.right_x, self.merged_state.right_y
        ));
        lines.push(format!(
            "DPAD L:{} R:{} U:{} D:{}",
            b(self.merged_state.dpad_left),
            b(self.merged_state.dpad_right),
            b(self.merged_state.dpad_up),
            b(self.merged_state.dpad_down)
        ));
        lines.push(format!(
            "BTN  A:{} B:{}",
            b(self.merged_state.button_a),
            b(self.merged_state.button_b)
        ));

        for device in &self.devices {
            let primary_mark = if Some(device.id) == self.primary_gamepad { "*" } else { " " };
            lines.push(format!(
                "{}DEV {} [{}] {}",
                primary_mark,
                device.id,
                device.profile_name.as_deref().unwrap_or("default"),
                device.name
            ));
            lines.push(format!(
                "{}BTN A:{} B:{}",
                primary_mark,
                b(device.state.button_a),
                b(device.state.button_b)
            ));
            lines.push(format!(
                "{}AXIS LX:{:+.2} LY:{:+.2} RX:{:+.2} RY:{:+.2}",
                primary_mark,
                device.state.left_x,
                device.state.left_y,
                device.state.right_x,
                device.state.right_y
            ));
            lines.push(format!("{}LAST {}", primary_mark, device.last_event));
            if device.raw_buttons_down.is_empty() {
                lines.push(format!("{}RAW KEYS (none pressed)", primary_mark));
            } else {
                let mut items = Vec::new();
                for k in &device.raw_buttons_down {
                    items.push(k.to_string());
                }
                lines.push(format!("{}RAW KEYS {}", primary_mark, items.join(",")));
            }
        }

        lines
    }

    fn register_device(&mut self, id: GamepadId, announce: bool) {
        let gamepad = self.gilrs.gamepad(id);
        if !gamepad.is_connected() {
            return;
        }

        let name = gamepad.name().to_string();
        let (resolved_config, profile_name) = self.base_config.resolved_for_device_name(Some(&name));

        if let Some(device) = self.devices.iter_mut().find(|d| d.id == id) {
            device.name = name.clone();
            device.profile_name = profile_name.clone();
            device.config = resolved_config;
            device.state.connected = true;
        } else {
            self.devices.push(TrackedGamepad {
                id,
                name: name.clone(),
                profile_name: profile_name.clone(),
                config: resolved_config,
                state: GamepadState::default(),
                last_event: "(none)".to_string(),
                raw_axes: BTreeMap::new(),
                raw_buttons_down: BTreeSet::new(),
                activity_tick: self.activity_clock,
            });
        }

        if self.primary_gamepad.is_none() {
            self.primary_gamepad = Some(id);
        }

        if announce {
            println!("Gamepad detected: {} ({:?})", name, id);
            match profile_name {
                Some(profile) => println!("Applied gamepad profile for {:?}: {}", id, profile),
                None => println!("Applied gamepad profile for {:?}: (default)", id),
            }
        }
    }

    fn unregister_device(&mut self, id: GamepadId) {
        self.devices.retain(|d| d.id != id);
        if self.primary_gamepad == Some(id) {
            self.primary_gamepad = self.devices.first().map(|d| d.id);
        }
    }

    fn primary_device(&self) -> Option<&TrackedGamepad> {
        self.primary_gamepad
            .and_then(|id| self.devices.iter().find(|d| d.id == id))
            .or_else(|| self.devices.first())
    }

    fn mark_activity(&mut self, id: GamepadId) {
        self.activity_clock = self.activity_clock.saturating_add(1);
        if let Some(device) = self.devices.iter_mut().find(|d| d.id == id) {
            device.activity_tick = self.activity_clock;
        }
        self.primary_gamepad = Some(id);
    }

    fn update_last_event(&mut self, id: GamepadId, event: String) {
        if let Some(device) = self.devices.iter_mut().find(|d| d.id == id) {
            device.last_event = event;
        }
    }

    fn handle_raw_button<T: std::fmt::Debug>(&mut self, id: GamepadId, button: Button, pressed: bool, code: &T) {
        if !matches!(button, Button::Unknown) {
            return;
        }
        let Some(raw_code) = extract_raw_code(code) else {
            return;
        };
        if let Some(device) = self.devices.iter_mut().find(|d| d.id == id) {
            if pressed {
                device.raw_buttons_down.insert(raw_code);
            } else {
                device.raw_buttons_down.remove(&raw_code);
            }
        }
    }

    fn handle_raw_axis<T: std::fmt::Debug>(&mut self, id: GamepadId, axis: Axis, value: f32, code: &T) {
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
        if let Some(device) = self.devices.iter_mut().find(|d| d.id == id) {
            device.raw_axes.insert(raw_code, value);
        }
    }

    fn refresh_all_snapshots(&mut self) {
        for idx in 0..self.devices.len() {
            self.refresh_device_snapshot(idx);
        }
    }

    fn refresh_device_snapshot(&mut self, idx: usize) {
        let id = self.devices[idx].id;
        let gamepad = self.gilrs.gamepad(id);
        if !gamepad.is_connected() {
            self.devices[idx].state = GamepadState::default();
            return;
        }

        let device = &mut self.devices[idx];
        device.state.connected = true;

        let lx = axis_value(&gamepad, &[Axis::LeftStickX, Axis::RightStickX], device.config.deadzone);
        let ly = axis_value(&gamepad, &[Axis::LeftStickY, Axis::RightStickY], device.config.deadzone);
        let rx = axis_value(&gamepad, &[Axis::RightStickX, Axis::LeftStickX], device.config.deadzone);
        let ry = axis_value(&gamepad, &[Axis::RightStickY, Axis::LeftStickY], device.config.deadzone);
        device.state.left_x = lx;
        device.state.left_y = -ly;
        device.state.right_x = rx;
        device.state.right_y = -ry;

        let dpad_x = axis_value(&gamepad, &[Axis::DPadX, Axis::LeftZ, Axis::RightZ], device.config.deadzone);
        let dpad_y = axis_value(&gamepad, &[Axis::DPadY], device.config.deadzone);
        device.state.dpad_left = gamepad.is_pressed(Button::DPadLeft) || dpad_x < -0.5;
        device.state.dpad_right = gamepad.is_pressed(Button::DPadRight) || dpad_x > 0.5;
        device.state.dpad_up = gamepad.is_pressed(Button::DPadUp) || dpad_y < -0.5;
        device.state.dpad_down = gamepad.is_pressed(Button::DPadDown) || dpad_y > 0.5;

        apply_linux_raw_axis_mapping(&mut device.state, &device.raw_axes, &device.config);
        apply_configured_button_mapping(&gamepad, device);
    }

    fn recompute_merged_state(&mut self) {
        let mut merged = GamepadState {
            connected: !self.devices.is_empty(),
            ..GamepadState::default()
        };

        for device in &self.devices {
            merged.dpad_left |= device.state.dpad_left;
            merged.dpad_right |= device.state.dpad_right;
            merged.dpad_up |= device.state.dpad_up;
            merged.dpad_down |= device.state.dpad_down;
            merged.button_a |= device.state.button_a;
            merged.button_b |= device.state.button_b;
        }

        if let Some(axis_device) = self
            .devices
            .iter()
            .max_by_key(|d| d.activity_tick)
            .or_else(|| self.devices.first())
        {
            merged.left_x = axis_device.state.left_x;
            merged.left_y = axis_device.state.left_y;
            merged.right_x = axis_device.state.right_x;
            merged.right_y = axis_device.state.right_y;
        }

        self.merged_state = merged;
    }
}

#[cfg(feature = "gamepad")]
fn apply_linux_raw_axis_mapping(state: &mut GamepadState, raw_axes: &BTreeMap<u32, f32>, config: &GamepadConfig) {
    if let Some(v) = raw_axes.get(&config.axis_x_code).copied() {
        state.left_x = apply_deadzone(v, config.deadzone);
    }
    if let Some(v) = raw_axes.get(&config.axis_y_code).copied() {
        state.left_y = -apply_deadzone(v, config.deadzone);
    }
    if let Some(v) = raw_axes.get(&config.hat_x_code).copied() {
        state.dpad_left = v < -0.5;
        state.dpad_right = v > 0.5;
    }
    if let Some(v) = raw_axes.get(&config.hat_y_code).copied() {
        state.dpad_up = v < -0.5;
        state.dpad_down = v > 0.5;
    }
}

#[cfg(feature = "gamepad")]
fn apply_configured_button_mapping(gamepad: &gilrs::Gamepad<'_>, device: &mut TrackedGamepad) {
    device.state.button_a = any_pressed_input(gamepad, &device.raw_buttons_down, &device.config.button_0_inputs_resolved());
    device.state.button_b = any_pressed_input(gamepad, &device.raw_buttons_down, &device.config.button_1_inputs_resolved());
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
fn any_pressed_input(
    gamepad: &gilrs::Gamepad<'_>,
    raw_buttons_down: &BTreeSet<u32>,
    inputs: &[GamepadButtonInput],
) -> bool {
    inputs.iter().any(|input| match input {
        GamepadButtonInput::Name(name) => button_from_name(name)
            .map(|button| gamepad.is_pressed(button))
            .unwrap_or(false),
        GamepadButtonInput::RawCode(code) => raw_buttons_down.contains(code),
    })
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

    pub fn active_gamepad_name(&self) -> Option<&str> {
        None
    }

    pub fn active_profile_name(&self) -> Option<&str> {
        None
    }

    pub fn debug_lines(&self) -> Vec<String> {
        vec!["GP: disabled".to_string()]
    }
}

#[cfg(feature = "gamepad")]
pub fn run_input_probe() -> Result<(), String> {
    let mut gilrs = Gilrs::new().map_err(|e| format!("Failed to initialize gamepad: {}", e))?;

    println!("Gamepad probe started.");
    println!("Press controller buttons to see gilrs names and raw input codes.");
    println!("Press Ctrl+C to exit.");
    println!();

    let mut found_any = false;
    for (id, gamepad) in gilrs.gamepads() {
        found_any = true;
        println!("Detected: {} ({:?})", gamepad.name(), id);
    }
    if !found_any {
        println!("No gamepad detected yet. Connect one and press a button.");
    }

    let mut last_snapshot = Instant::now();

    loop {
        while let Some(Event { id, event, time, .. }) = gilrs.next_event() {
            match event {
                EventType::Connected => {
                    println!("[{:?}] connected at {:?}", id, time);
                }
                EventType::Disconnected => {
                    println!("[{:?}] disconnected at {:?}", id, time);
                }
                EventType::ButtonPressed(button, code) => {
                    println!(
                        "[{:?}] pressed  {:<12} raw={:?} role={}",
                        id,
                        format!("{:?}", button),
                        code,
                        button_role(button)
                    );
                }
                EventType::ButtonReleased(button, code) => {
                    println!(
                        "[{:?}] released {:<12} raw={:?} role={}",
                        id,
                        format!("{:?}", button),
                        code,
                        button_role(button)
                    );
                }
                EventType::ButtonChanged(button, value, code) => {
                    if value.abs() >= 0.5 {
                        println!(
                            "[{:?}] changed  {:<12} value={:+.2} raw={:?} role={}",
                            id,
                            format!("{:?}", button),
                            value,
                            code,
                            button_role(button)
                        );
                    }
                }
                EventType::AxisChanged(axis, value, code) => {
                    if is_interesting_axis(axis) && value.abs() >= 0.5 {
                        println!(
                            "[{:?}] axis     {:<12} value={:+.2} raw={:?}",
                            id,
                            format!("{:?}", axis),
                            value,
                            code
                        );
                    }
                }
                _ => {}
            }
        }

        if last_snapshot.elapsed() >= Duration::from_secs(2) {
            last_snapshot = Instant::now();
            for (id, gamepad) in gilrs.gamepads() {
                if !gamepad.is_connected() {
                    continue;
                }
                let mut held = Vec::new();
                for button in [
                    Button::South,
                    Button::East,
                    Button::West,
                    Button::North,
                    Button::C,
                    Button::Z,
                    Button::LeftTrigger,
                    Button::LeftTrigger2,
                    Button::RightTrigger,
                    Button::RightTrigger2,
                    Button::Select,
                    Button::Start,
                    Button::Mode,
                    Button::DPadLeft,
                    Button::DPadRight,
                    Button::DPadUp,
                    Button::DPadDown,
                ] {
                    if gamepad.is_pressed(button) {
                        held.push(format!("{:?}", button));
                    }
                }
                if !held.is_empty() {
                    println!("[{:?}] held     {}", id, held.join(", "));
                }
            }
        }

        thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(not(feature = "gamepad"))]
pub fn run_input_probe() -> Result<(), String> {
    Err("gamepad feature is disabled. Rebuild with `--features gamepad`.".to_string())
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
fn button_role(button: Button) -> &'static str {
    match button {
        Button::South => "button_0 default",
        Button::East => "button_1 default",
        Button::West => "not used by default",
        Button::North => "not used by default",
        _ => "-",
    }
}

#[cfg(feature = "gamepad")]
fn is_interesting_axis(axis: Axis) -> bool {
    matches!(
        axis,
        Axis::LeftStickX
            | Axis::LeftStickY
            | Axis::RightStickX
            | Axis::RightStickY
            | Axis::DPadX
            | Axis::DPadY
            | Axis::LeftZ
            | Axis::RightZ
    )
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
