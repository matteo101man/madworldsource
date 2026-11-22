fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bin=madworld-native=/DYNAMICBASE:NO");
        println!("cargo:rustc-link-arg-bin=madworld-native=/BASE:0x1B0000000");
    }
}
