fn main() {
    embuild::espidf::sysenv::output();
    // esp-dl's prebuilt model loader needs libstdc++ symbols that are linked before it.
    println!("cargo:rustc-link-arg=-lstdc++");
}
