fn main() {
    println!("cargo:rerun-if-changed=src/server.c");
    cc::Build::new()
        .file("src/server.c")
        .warnings(true)
        .extra_warnings(true)
        .compile("dmmt_server");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=ws2_32");
    }
}
