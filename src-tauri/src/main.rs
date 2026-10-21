#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(code) = music_generator::engine::process::supervise() {
        std::process::exit(code);
    }
    music_generator::run();
}
