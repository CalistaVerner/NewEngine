pub fn init() {
    // Можно настроить через RUST_LOG=info,newengine_core=debug и т.п.
    let _ = env_logger::builder()
        .format_timestamp_millis()
        .try_init();
}