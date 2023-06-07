fn main() {
    cgo::Build::new()
        .trimpath(true)
        .ldflags("-s -w")
        .package("pkg/yaml.go")
        .build("metayaml");

    println!("cargo:rerun-if-changed=pkg");
    println!("cargo:rerun-if-changed=go.mod");
}
