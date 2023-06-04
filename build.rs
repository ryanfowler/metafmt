use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    Command::new("go")
        .args([
            "build",
            "-buildmode",
            "c-archive",
            "-ldflags",
            "-s -w",
            "-o",
            &format!("{}/libyaml.a", out_dir),
            "pkg/yaml.go",
        ])
        .status()
        .expect("building Go yaml library failed");

    println!("cargo:rerun-if-changed=pkg");
    println!("cargo:rerun-if-changed=go.mod");
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=yaml");
}
