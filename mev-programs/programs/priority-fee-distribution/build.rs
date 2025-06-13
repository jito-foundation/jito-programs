fn main() {
    println!("cargo:rerun-if-env-changed=GIT_SHA");
    println!(
        "cargo:rustc-env=GIT_SHA={}",
        option_env!("GIT_SHA").unwrap_or("GIT_SHA_MISSING")
    );

    println!("cargo:rerun-if-env-changed=GIT_REF_NAME");
    println!(
        "cargo:rustc-env=GIT_REF_NAME={}",
        option_env!("GIT_REF_NAME").unwrap_or("GIT_REF_NAME_MISSING")
    );
}
