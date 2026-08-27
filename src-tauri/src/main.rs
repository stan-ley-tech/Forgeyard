// Prevents an additional console window from spawning on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    forgeyard_lib::run();
}
