# `include_assets` in your executable

`include_assets` provides convenient ways to include assets (arbitrary files) in a Rust binary.
Assets are compressed and can either be looked up by file name or by variants of an enumeration.


## assets by name

This is probably the most straightforward approach.
Include all files in a directory with the `include_dir!()` macro, load (decompress) the assets at runtime using `NamedArchive::load`.
Once they are loaded, use the `NamedArchive` more or less as you would a `HashMap<&str, &[u8]>`.

For examples, see the [docs](https://docs.rs/crate/include_assets/latest/include_assets/macro.include_dir.html) and [`examples/named/src/main.rs`](examples/named/src/main.rs).


## assets by enum variant

This approach might be a little unusual.
Declare an enum with one unit variant per asset, and derive the trait `EnumAsset` using the derive macro that comes with this crate.
Load the uncompressed assets using `EnumArchive::<MyEnum>::load()` (replacing `MyEnum` with whatever name you chose for your enum).
Then look up the asset data via indexing (`&archive[MyAsset::SomeVariant]`) - this is infallible!

This approach has two distinct advantages:

- You cannot accidentally use any asset that's not included in the executable: If you try, that's a compile-time error.
- If you include an asset in the binary but never use it (i.e. never construct the corresponding enum variant) that causes a compile-time warning.

A disadvantage is that you cannot iterate over assets.
Additionally, their names are erased at runtime.

Despite the lack of iteration, asset data can be mapped using `AssetEnum::map`.
You can then look up assets by enum variant in the resulting `EnumMap`.
This may be useful for homogenous assets, you could for example parse templates, decode sound/image files, &c.
Note that you can have multiple `EnumAsset`s in the same program; mapping will be more useful if you have different enums for different types of asset.

For examples, see the [docs](https://docs.rs/crate/include_assets/latest/include_assets/struct.EnumArchive.html) and [`examples/enums/src/main.rs`](examples/enums/src/main.rs).

## Build script

If you want to rebuild the executable whenever one of the assets changes, you should use a [`build.rs`](https://doc.rust-lang.org/cargo/reference/build-scripts.html) like this:

```
fn main() {
    println!("cargo:rerun-if-changed=path/to/assets");
    println!("cargo:rerun-if-changed=more/assets");
}
```


## Licence

This crate is (probably going to be but NOT CURRENTLY) licensed under the LGPL v3.


## Compression

Currently supported: zstd, lz4, deflate, no compression.


## Checksums

At compile time, a checksum is computed for each asset.
These checksums are included in the binary.
When loading/decompressing assets, the checksum of decompressed assets is compared against the compile-time checksum as a measure against data corruption and (more importantly) bugs.

Currently, blake2b is used for this, but this may change in the future.


## Limitations

At runtime, main memory needs to be big enough to hold all assets at the same time in compressed and uncompressed form.
At compile time, main memory needs to be big enough to hold all assets at the same time in compressed form and twice in uncompressed form.

The total size of each asset archive cannot exceed `u32::MAX` (4 GiB).
Each asset archive can contain at most `u32::MAX` (roughly 4e9) distinct assets.
If your use case exceeds these limits, reconsider if this is really the right approach.

`usize` is required to be at least 32 bits wide.


## Related work

Rust `core` includes the `include_bytes!` macro which allows including a single file (uncompressed).

There are several crates which allow including compressed files, and even directories.

As far as I know, this crate is the only one which compresses included files as a whole rather than seperately.
This approach has a significant disadvantage: To decompress a single file, all files have to be decompressed.
However, it leads to better compression because the compression algorithm can take advantage of similarities between files in addition to similarities within each file.


## Future work

- Compile times suck.
  I'm not sure how the `include_bytes!` macro works, but it _probably_ doesn't just dump a huge bytestring into the AST.
  I'd love to write the data to `OUT_DIR` and then include that blob with `include_bytes!`, but that's [not available in proc macros](https://github.com/rust-lang/cargo/issues/9084).
  As a workaround it may be (more?) useful to generate code and a compressed blob from `build.rs`, then `include!` it from the main code.
- If the assets are big, decompression can be rather slow.
  It may be worth investigating zstandard (and lz4) dictionary compression.
  At compile time, a dictionary can be created by analyzing each asset.
  Then, each asset can be compressed independently using this dictionary.
  Hopefully, this wouldn't result in significantly bigger files than compressing them together.
  The advantage of this approach is that runtime compression could be done in parallel using multiple threads.
  Alternatively, each file could be decompressed as needed, but this is not currently a goal of this crate.
- It may be useful to deduplicate assets based on contents before compression.
  Compression can obviously reduce size but only if the redundant files are not too far apart.
  This can be implemented with a layer of indirection: asset name maps to a blob id, blob id maps to a data range.
  Currently deduplication is best achieved in application code: `archive.get(override_asset).unwrap_or_else(|| archive[fallback_asset])`
- It may be useful to provide options to use other checksum algorithms.
  Possible options: CRC for smaller hashes, SHA256 for (possibly) faster hashing (special CPU instructions!), `[u8; 0]` to effectively disable checking.
  The choice is probably best handled through feature flags.
- Error handling of the macros could use some work, but this is blocked on stable Rust allowing proc macro diagnostics.


## Contributing

[Bug reports](https://github.com/hwjsnc/include_assets/issues) are very welcome.
Feature requests are also welcome, but no promises.
I do not plan to accept patches at this time.
