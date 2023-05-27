fn main() {
    println!("cargo:rerun-if-changed=."); // this folder is the asset directory, so rebuild on any changes
}
