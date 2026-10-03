//! Controlled short-lived process for exit-status and cancellation tests.
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let code: i32 = args[0].parse().unwrap();
    let delay: u64 = args[1].parse().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(delay));
    std::process::exit(code);
}
