fn main() {
    // The pages are in the exe (tauri.conf.json's frontendDist), put there by
    // the macro in the app's own compilation. A change in them has to build
    // the exe again, or it keeps the pages it had: tauri_build::build
    // watches the config and the capabilities, but not the pages, and cargo
    // does not redo the macro when only they changed (the Rust side did not
    // move). What was run then was the pages of the last build that touched
    // the Rust side, and a change in a page seemed to do nothing.
    watch(&std::path::Path::new("../src"));
    tauri_build::build()
}

// Every file under a folder, one by one: cargo watches the path it is given,
// and a folder's own mtime does not move when a file in it is written.
fn watch(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            watch(&path);
        }
    }
}