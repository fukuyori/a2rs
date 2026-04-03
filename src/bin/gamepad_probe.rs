#[cfg(feature = "gamepad")]
use gilrs::{Axis, Button, Event, EventType, Gilrs};

#[cfg(feature = "gamepad")]
use std::thread;
#[cfg(feature = "gamepad")]
use std::time::{Duration, Instant};

#[cfg(feature = "gamepad")]
fn main() {
    let mut gilrs = match Gilrs::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to initialize gamepad: {}", e);
            std::process::exit(1);
        }
    };

    println!("Gamepad probe started.");
    println!("Press controller buttons to see gilrs names such as South/East/West/North.");
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

#[cfg(feature = "gamepad")]
fn button_role(button: Button) -> &'static str {
    match button {
        Button::South => "button_a default",
        Button::East => "button_b default",
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

#[cfg(not(feature = "gamepad"))]
fn main() {
    eprintln!("gamepad feature is disabled. Rebuild with `--features gamepad`.");
    std::process::exit(1);
}
