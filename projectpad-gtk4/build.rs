use std::process::Command;

// can probably be cleaned up: https://gtk-rs.org/gtk4-rs/stable/latest/book/composite_templates.html#resources
fn main() {
    // println!("cargo:rerun-if-changed=src/projectpad.gresource.xml.in");

    let status = Command::new("glib-compile-resources")
        .arg("src/projectpad.gresource.xml.in")
        .arg("--target=src/resources.bin")
        .spawn()
        .expect("Failed running glib-compile-resources")
        .wait()
        .unwrap();
    assert!(status.success());
}
