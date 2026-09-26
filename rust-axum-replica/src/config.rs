use std::env;

pub fn nettruyen_url() -> String {
    env::var("NETTRUYEN_URL").unwrap_or_else(|_| "https://nettruyenar.com".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nettruyen_url_uses_default_when_env_missing() {
        let previous = env::var("NETTRUYEN_URL").ok();
        unsafe {
            env::remove_var("NETTRUYEN_URL");
        }

        let url = nettruyen_url();

        if let Some(value) = previous {
            unsafe {
                env::set_var("NETTRUYEN_URL", value);
            }
        }

        assert_eq!(url, "https://nettruyenar.com");
    }

    #[test]
    fn nettruyen_url_uses_env_override_when_present() {
        let previous = env::var("NETTRUYEN_URL").ok();
        unsafe {
            env::set_var("NETTRUYEN_URL", "https://example.test");
        }

        let url = nettruyen_url();

        if let Some(value) = previous {
            unsafe {
                env::set_var("NETTRUYEN_URL", value);
            }
        } else {
            unsafe {
                env::remove_var("NETTRUYEN_URL");
            }
        }

        assert_eq!(url, "https://example.test");
    }
}
