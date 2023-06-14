fn main() {
    cgo::Build::new()
        .trimpath(true)
        .ldflags("-s -w")
        .package("metayaml/main.go")
        .build("metayaml");

    println!("cargo:rerun-if-changed=pkg");
    println!("cargo:rerun-if-changed=go.mod");

    let target = std::env::var("TARGET").unwrap();
    println!("cargo:rustc-env=TARGET={target}");
    println!("cargo:rerun-if-changed-env=TARGET");
}
