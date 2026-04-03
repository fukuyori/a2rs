fn main() {
    if let Err(err) = a2rs::gamepad::run_input_probe() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
