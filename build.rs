fn main() {
    cgo::Build::new()
        .trimpath(true)
        .ldflags("-s -w")
        .package("metayaml/main.go")
        .build("metayaml");

    println!("cargo:rerun-if-changed=pkg");
    println!("cargo:rerun-if-changed=go.mod");
}
