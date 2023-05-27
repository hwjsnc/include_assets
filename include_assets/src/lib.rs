/*! # `include_assets` in your executable

This crate provides convenient ways to include assets (arbitrary files) in a Rust executable.
It's like [`std::include_bytes!`] but works for multiple files.

Files are collected into archives, which are compressed at compile time and can be decompressed at runtime.
Archives are ["solid"](https://en.wikipedia.org/wiki/Solid_compression): Instead of compressing each asset independently, assets are first concatenated, then compressed as a whole.
As far as I'm aware, this crate is the only which does this!
Solid compression leads to smaller sizes since the compression algorithm can take advantage of redundancy between files.
However, all assets must be decompressed at once - if your assets cannot completely fit into main memory at the same time, or startup time is an issue, don't use this crate!

Potential use cases are:
- games shipping with fonts, sprites/textures, sounds, &c.,
- webservers serving static content (HTML templates, pictures, &c.),
- installers, or
- self-extracting archives.

# Include an asset directory and look up data by path name

Arguably the more straightforward approach.
Include an asset directory using the [`include_dir!`] macro.
Load (decompress) it at runtime using [`NamedArchive::load`].

Once loaded, use [`NamedArchive::get`] or `&archive["asset name"]` to look up asset data by name, or iterate through all assets with [`NamedArchive::assets`].

```
use include_assets::{NamedArchive, include_dir};

let archive = NamedArchive::load(include_dir!("assets"));
let hello_asset = archive.get("hello.txt").unwrap(); // Panics at runtime if the asset isn't present!
assert_eq!(hello_asset, b"Hello, world!");
println!("{} assets were included", archive.number_of_assets());
```

For more examples, see [`include_dir!`].


# Include assets and look up data by enum variant

A perhaps less intuitive approach with advantages and disadvantages to the previous one.
Declare an enumeration with one variant for each asset, and `#[derive(AssetEnum)]` with annotations for the (compile-time) path of the asset.
At runtime, load (decompress) the asset archive with the derived `load` method for your enum.
Look up asset data by enum variant using indexing.
Transform raw asset data using [`EnumArchive::map`] and [`EnumArchive::try_map`].

```
use include_assets::EnumArchive;

#[derive(include_assets::AssetEnum)]
#[archive(base_path = "assets")]
enum Asset {
    #[asset(path = "hello.txt")]
    Hello,
    #[asset(path = "unused.txt")]
    Unused, // Unused asset causes a compile-time warning!
}

let archive = EnumArchive::<Asset>::load();
let hello_asset = &archive[Asset::Hello]; // Presence of asset is ensured at compile time!
assert_eq!(hello_asset, b"Hello, world!");

let strings = archive.map(|data| std::str::from_utf8(data).unwrap().to_owned());
assert_eq!(&strings[Asset::Hello], "Hello, world!");
```

As indicated by the code comments, this method has the advantage that use of assets is checked at compile time.
Assets that are not present cannot be used, and unused assets cause compile-time warnings.

On the other hand, assets have to be declared manually, and cannot be iterated over.
(A `map` function is provided though.)

For more examples, see [`EnumArchive`].

# Build script

It is probably a good idea to tell Cargo to rebuild the executable whenever an asset changes.
This can be achieved with a `build.rs` such as:

```
fn main() {
    println!("cargo:rerun-if-changed=path/to/assets/");
    println!("cargo:rerun-if-changed=more/assets");
}
```

# Options

The macros that include assets have a few optional arguments.
These options must always be specified in the form of a `identifier = literal` assignment, where `identifier` is one of the following values:

- `compression`:
   Specifies the compression algorithm to be used.
   The default choice depends on crate features and is not bound by semver.
   It attempts to strike a balance between compression speed, decompression speed, and size reduction.
   The following values are potentially allowed:
   - `"zstd"` (requires feature `zstd`),
   - `"lz4"` (requires feature `lz4`),
   - `"deflate"` (requires feature `deflate`), and
   - `"uncompressed"`. This option should generally not be used except for assets which are already compressed (e.g. JPEG/PNG/FLAC).
- `level`:
  Compression level parameter.
  Meaning and allowed values depend on the chosen compression algorithm.
  Default values are unspecified and not bound by semver.
  - for `compression = "zstd"`:
    Smaller values are generally faster with worse compression quality.
    "Normal" compression levels are `1..=19`, "high" compression levels are `20..=22`, negative values signify "fast" compression levels.
  - for `compression = "lz4"`:
    This argument is not allowed.
  - for `compression = "deflate"`:
    Levels are in `1..=10`. Smaller values are generally faster with marginally worse compression quality.
  - for `compression = "uncompressed"`:
    This argument is not allowed.
- `links`:
  Specifies behaviour when a symbolic link is encountered.
  This option is only available for the [`include_dir!`] macro.
  Valid values are:
  - `links = "forbid"`:
    A compilation error is generated when a symbolic link is encountered.
    This is the default behaviour.
  - `links = "ignore"`:
    Symbolic links are ignored.
    If the link points (directly or indirectly) to a file, this file is not included via the link.
    If the link points to a directory, files in the directory are not included via the link.
  - `links = "follow"`:
    Symbolic links are treated as if they were the target directory or file.

# Limitations

At runtime, main memory needs to be big enough to hold all assets at the same time in compressed and uncompressed form.
At compile time, main memory needs to be big enough to hold all assets at the same time in compressed form and twice in uncompressed form.
(It would be possible to optimize compile time memory use, but if you can only barely compile it, users probably can't run it.)

The total size of each asset archive cannot exceed `u32::MAX` (4 GiB).
Each asset archive can contain at most `u32::MAX` (roughly 4e9) distinct assets.
If your use case exceeds these limits, reconsider if this is really the right approach.

`usize` is required to be at least 32 bits wide.
*/

pub use include_assets_decode::named::NamedArchive;

/// Include all files in a directory in compressed form.
/// At runtime, the files can be decompressed and their contents looked up by relative path name.
///
/// # Usage
///
/// The first argument must be a string literal specifying the path of the directory to be included.
/// This can be an absolute path or a path relative to the [`CARGO_MANIFEST_DIR`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
/// This path can be absolute (though this should be avoided) or relative to `cargo`'s working directory.
///
/// In addition, any of the options described in the [`crate`] level documentation may be used to specify compression options.
///
/// # Examples
///
/// Include the directory "assets":
///
/// ```
/// use include_assets::{NamedArchive, include_dir};
/// let archive = NamedArchive::load(include_dir!("assets"));
/// println!("{} assets were included", archive.number_of_assets());
/// ```
///
/// Include the directory "assets".
/// Assets will be compressed using zstd at level 5.
/// Symbolic links will be treated as if they were the file/directory pointed to:
///
/// ```
/// use include_assets::{NamedArchive, include_dir};
/// let archive = NamedArchive::load(include_dir!("assets", compression = "zstd", level = 5, links = "follow"));
/// println!("{} assets were included", archive.number_of_assets());
/// ```
///
/// Include the two directories "assets" (compressed with zstd level 22) and "other_assets" (lz4 compressed):
///
/// ```
/// use include_assets::{NamedArchive, include_dir};
/// let archive1 = NamedArchive::load(include_dir!("assets", compression = "zstd", level = 22));
/// let archive2 = NamedArchive::load(include_dir!("other_assets", compression = "lz4"));
/// println!("{} assets were included", archive1.number_of_assets() + archive2.number_of_assets());
/// ```
///
/// # Limitations
///
/// - The directory may only contains files, directories, or symbolic links which point (directly or indirectly) to a file or directory.
///   Special files are not allowed.
/// - Paths must be UTF-8
/// - Paths must not contain null bytes (U+0000)
pub use include_assets_encode::include_dir;

/// Derive the AssetEnum trait.
///
/// The trait should _never_ be implemented or used manually, _only_ with this derive macro.
/// Details (methods, associated types/constants) for the trait are not bound by semver!
///
/// This macro only works for enums.
/// Every enum variant must be unit (i.e. have no fields), and must not have an explicit discriminator.
/// There needs to be an outer attribute `#[archive(base_path = "path")]` on the enum specifying the base path of all assets.
/// This can be an absolute path or a path relative to the [`CARGO_MANIFEST_DIR`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
/// Every variant needs to have an attribute `#[asset(path = "relative path")]` specifying the (compile time) path of the asset relative to the base path.
///
/// Additionally, options described in the [`crate`] level documentation may be added to the outer enum attribute to specify compression options.
///
/// # Examples
///
/// Basic use.
/// Include `"assets/hello.txt"` and `"assets/unused.txt"` allowing lookup by `Asset::Hello` and `Asset::Unused`, respectively.
/// If the variant `Unused` is never used (as in this example), this will cause a compile-time warning.
///
/// ```
/// use include_assets::EnumArchive;
///
/// #[derive(include_assets::AssetEnum)]
/// #[archive(base_path = "assets")]
/// enum Asset {
///     #[asset(path = "hello.txt")]
///     Hello,
///     #[asset(path = "unused.txt")]
///     Unused,
/// }
///
/// let archive = EnumArchive::<Asset>::load();
/// assert_eq!(&archive[Asset::Hello], b"Hello, world!");
/// ```
///
/// For more information on how to use the decompressed assets, see [`EnumArchive`].
///
/// A more specific `derive` with the same assets.
/// Assets will be compressed using zstd at level 5.
/// The enum representation is explicitely chosen as `u8`:
///
/// ```
/// #[derive(include_assets::AssetEnum)]
/// #[archive(base_path = "assets", compression = "zstd", level = 5)]
/// #[repr(u8)]
/// enum Asset {
///     #[asset(path = "hello.txt")]
///     Hello,
///     #[asset(path = "unused.txt")]
///     Unused,
/// }
/// ```
///
/// Assets may not have fields or explicit discriminators:
///
/// ```compile_fail
/// #[derive(include_assets::AssetEnum)]
/// #[archive(base_path = "assets")]
/// enum Asset {
///     #[asset(path = "hello.txt")]
///     Hello(String), // field is not allowed
///     #[asset(path = "hello.txt")]
///     Hello2 { who: String }, // struct-like variant is not allowed
///     #[asset(path = "unused.txt")]
///     Unused = 42, // explicit discriminator is not allowed
/// }
/// ```
pub use include_assets_encode::AssetEnum;

#[doc(hidden)]
pub use include_assets_decode::enums::AssetEnum;

/// Archive holding uncompressed data for an [`AssetEnum`](derive@`AssetEnum`).
///
/// An `AssetEnum` is an `enum` type with unit variants.
/// Each variant corresponds to an asset.
/// An `EnumArchive` for a given `AssetEnum` allows looking up the enum data via indexing.
///
/// Iteration over all assets is not possible, but mapping the data is.
///
/// # Examples
///
/// Include assets and look up data by name:
///
/// ```
/// use include_assets::EnumArchive;
///
/// #[derive(include_assets::AssetEnum)]
/// #[archive(base_path = "assets")]
/// enum Asset {
///     #[asset(path = "hello.txt")]
///     Hello,
///     #[asset(path = "unused.txt")]
///     _Unused,
/// }
///
/// let archive = EnumArchive::<Asset>::load();
/// assert_eq!(&archive[Asset::Hello], b"Hello, world!");
/// ```
///
/// Include data and apply some transformation (here we convert the `&[u8]` data to `String` since all assets are plain text).
/// The transformed data can be looked up by enum variant:
///
/// ```
/// use include_assets::EnumArchive;
///
/// #[derive(include_assets::AssetEnum)]
/// #[archive(base_path = "assets")]
/// enum Asset {
///     #[asset(path = "hello.txt")]
///     Hello,
///     #[asset(path = "unused.txt")]
///     _Unused,
/// }
///
/// let archive = EnumArchive::<Asset>::load().map(|data| std::str::from_utf8(data).unwrap().to_owned());
/// assert_eq!(archive[Asset::Hello].as_str(), "Hello, world!");
/// ```
pub use include_assets_decode::enums::EnumArchive;

pub use include_assets_decode::enums::EnumMap;

#[doc(hidden)]
pub use include_assets_decode::named::CompressedNamedArchive;

#[doc(hidden)]
pub mod do_not_use_this_directly {
    pub use include_assets_decode::checksum::Checksum;
    pub use include_assets_decode::codec;
}
