pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn restore_env(name: &str, value: Option<String>) {
    unsafe {
        if let Some(value) = value {
            std::env::set_var(name, value);
        } else {
            std::env::remove_var(name);
        }
    }
}
