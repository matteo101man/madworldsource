fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-cdylib=/DYNAMICBASE:NO");
        println!("cargo:rustc-link-arg-cdylib=/BASE:0x1B0000000");
    }
}
