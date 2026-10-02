//! Controlled native test receiver: records argv beside its own temporary exe.
#![windows_subsystem = "windows"]
fn main() {
    let output = std::env::current_exe().unwrap().with_extension("args");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let staged = output.with_extension("args.tmp");
    std::fs::write(&staged, args.join("\n")).unwrap();
    std::fs::rename(staged, output).unwrap();
}
