// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // ── Panic hook: write crash info to a crash log ──
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {}", info);
        if let Some(dir) = dirs_next::data_dir() {
            let crash_dir = dir.join("MentorFish");
            let _ = std::fs::create_dir_all(&crash_dir);
            let crash_path = crash_dir.join("crash.log");
            let msg = format!(
                "=== MentorFish Crash Report ===\nTimestamp: {}\nPanic: {}\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                info
            );
            if let Err(e) = std::fs::write(&crash_path, &msg) {
                eprintln!("Failed to write crash log: {}", e);
            }
        }
    }));

    app_lib::run();
}
