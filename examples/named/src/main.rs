use include_assets::{include_dir, NamedArchive};

fn main() {
    // include_dir! path is relative to the workspace directory
    let archive = NamedArchive::load(include_dir!("."));

    // alternative examples:
    //let archive = NamedArchive::load(include_dir!("examples/named/", compression = "uncompressed"));
    //let archive = NamedArchive::load(include_dir!("examples/named/", compression = "lz4", links = "follow"));
    //let archive = NamedArchive::load(include_dir!("examples/named/", compression = "zstd", level = 5));
    //let archive = NamedArchive::load(include_dir!("examples/named/", compression = "deflate", level = 9, links = "forbid"));

    println!("the following {} assets included in this executable:", archive.number_of_assets());
    for (name, data) in archive.assets() {
        println!("{}: {} bytes", name, data.len());
    }
    println!();

    println!("Source code of this executable:");
    let main_rs = std::str::from_utf8(&archive["src/main.rs"]).unwrap();
    print!("{}", main_rs);
}
