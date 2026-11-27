#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    aureon_lib::win_glass::enable_per_monitor_dpi();

    aureon_lib::run();
}
