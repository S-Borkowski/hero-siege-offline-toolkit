fn main() {
    // `catalog::HUB_REPO` reads this through `option_env!`, which cargo does
    // not track as an input on its own. Without this line a cached build --
    // which is exactly what `swatinem/rust-cache` gives CI -- could hand back a
    // binary compiled against the previous value, so a fork's release would
    // quietly ship a hub pointing at whichever repository built it last.
    println!("cargo:rerun-if-env-changed=HUB_REPO");

    tauri_build::build()
}
