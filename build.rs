fn main() {
    println!("cargo:rerun-if-env-changed=LIFESHOT_HOST");
    println!("cargo:rerun-if-env-changed=LIFESHOT_PORT");
    println!("cargo:rerun-if-env-changed=LIFESHOT_ADDR");
}
