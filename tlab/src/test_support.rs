use std::{
    ffi::{OsStr, OsString},
    sync::{Mutex, OnceLock},
};

pub fn environment_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub struct EnvironmentVariable {
    name: OsString,
    previous: Option<OsString>,
}

impl EnvironmentVariable {
    pub fn set(name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        let name = name.as_ref().to_os_string();
        let previous = std::env::var_os(&name);
        unsafe {
            std::env::set_var(&name, value);
        }
        Self { name, previous }
    }

    pub fn remove(name: impl AsRef<OsStr>) -> Self {
        let name = name.as_ref().to_os_string();
        let previous = std::env::var_os(&name);
        unsafe {
            std::env::remove_var(&name);
        }
        Self { name, previous }
    }
}

impl Drop for EnvironmentVariable {
    fn drop(&mut self) {
        unsafe {
            if let Some(value) = &self.previous {
                std::env::set_var(&self.name, value);
            } else {
                std::env::remove_var(&self.name);
            }
        }
    }
}
