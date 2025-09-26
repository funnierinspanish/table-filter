pub fn exit_with_error(msg: &str) -> ! {
    eprintln!("Error: {}", msg);
    std::process::exit(1);
}
