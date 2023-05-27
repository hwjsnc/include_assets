use include_assets::{AssetEnum, EnumArchive};

#[derive(AssetEnum)]
#[archive(base_path = ".", compression = "zstd", level = 5)]
enum Asset {
    #[asset(path = "build.rs")]
    BuildScript,
    #[asset(path = "src/main.rs")]
    Main,
    #[asset(path = "Cargo.toml")]
    Cargo, // There should be a warning: "variant `Cargo` is never constructed"
}

fn main() {
    let archive: EnumArchive<Asset> = EnumArchive::load();

    let main_size = archive[Asset::Main].len();
    println!("main is {} bytes large", main_size);

    // map all files to String for convenience
    let string_archive = archive.map(|data| std::str::from_utf8(data).unwrap().to_owned());

    print!("builds.rs for this example:\n{}", string_archive[Asset::BuildScript]);
}
