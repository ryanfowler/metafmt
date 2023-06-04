use std::process::Command;

fn main() {
    Command::new("go")
        .args([
            "build",
            "-buildmode",
            "c-archive",
            "-ldflags",
            "-s -w",
            "-o",
            "bin/libyaml.a",
            "pkg/yaml.go",
        ])
        .status()
        .expect("building Go yaml library failed");

    println!("cargo:rerun-if-changed=pkg");
    println!("cargo:rerun-if-changed=go.mod");
    println!("cargo:rustc-link-search=native=bin");
    println!("cargo:rustc-link-lib=static=yaml");
}
