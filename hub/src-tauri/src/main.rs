// Release builds get no console window; debug builds keep one so `println!` and
// a panic backtrace have somewhere to go.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    hero_siege_toolkit_hub_lib::run()
}
