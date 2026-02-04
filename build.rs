fn main() {
    let target = std::env::var("TARGET").unwrap();
    println!("cargo:rustc-env=TARGET={target}");
    println!("cargo:rerun-if-changed-env=TARGET");
}
