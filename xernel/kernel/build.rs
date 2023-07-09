fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rustc-link-search={}/../../FlyingBalls/", dir);
    println!("cargo:rustc-link-lib=flyingballs");
}