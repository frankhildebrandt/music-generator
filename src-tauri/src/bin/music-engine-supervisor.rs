fn main() {
    std::process::exit(music_generator::engine::process::supervise().unwrap_or(2));
}
