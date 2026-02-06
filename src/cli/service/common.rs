use std::process::Command;

pub(super) const SERVICE_PATH: &str = "/etc/systemd/system/edgelord.service";

pub(super) fn run_systemctl(args: &[&str]) -> bool {
    Command::new("systemctl")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(super) fn is_root() -> bool {
    // On Unix, check if effective UID is 0
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
