use anyhow::Context as _;

use crate::common::{compress_names, compress_sizes};
use include_assets_decode::checksum::{compute_checksum, Checksum};
use include_assets_decode::codec::Codec;

pub struct NamedArchive {
    /// Compressed data
    ///
    /// All assets are concatenated
    /// The order of asset data must match the order of assets in `compressed_names`.
    pub compressed_data: std::vec::Vec<u8>,
    /// Size of the data after decompression
    pub uncompressed_data_size: u32,
    /// Compressed names of the assets in ascending order, with separating null bytes
    pub compressed_names: std::vec::Vec<u8>,
    /// Size of the uncompressed names (including separating null bytes)
    pub uncompressed_names_size: u32,
    /// Sizes of asset data, in the same order as `compressed_names`.
    pub compressed_sizes: std::vec::Vec<u8>,
    /// Asset checksums, in the same order as `compressed_names`.
    pub checksums: std::vec::Vec<Checksum>,
}

pub fn prepare_named_archive<C: Codec + ?Sized>(
    codec: &C,
    assets: std::vec::Vec<(smartstring::SmartString<smartstring::LazyCompact>, std::vec::Vec<u8>)>,
) -> anyhow::Result<NamedArchive> {
    // ensure that names are unique
    {
        let mut names = std::collections::HashSet::new();
        for (name, _) in assets.iter() {
            let is_new = names.insert(name);
            if !is_new {
                panic!("duplicate asset name: {name}")
            }
        }
    }

    // compress asset names, sizes, and compute checksums
    let (compressed_names, uncompressed_names_size) = compress_names(codec, assets.iter().map(|(name, _)| name)).context("couldn't compress asset names")?;
    let compressed_sizes = compress_sizes(codec, assets.iter().map(|(name, data)| (name, data.len()))).context("couldn't compress asset sizes")?;
    let checksums: std::vec::Vec<Checksum> = assets.iter().map(|(_, data)| compute_checksum(data.as_ref())).collect();

    // compress data
    let mut uncompressed_data = vec![];
    for (_, asset_data) in assets.iter() {
        uncompressed_data.extend_from_slice(asset_data.as_slice());
    }
    let compressed_data = codec.compress(uncompressed_data.as_slice()).context("couldn't compress asset data")?;

    // ensure that the uncompressed data isn't too big
    let uncompressed_data_size: u32 = uncompressed_data
        .len()
        .try_into()
        .map_err(|_| anyhow::Error::msg(format!("too much data ({} bytes)", uncompressed_data.len())))?;

    Ok(NamedArchive {
        compressed_data,
        uncompressed_data_size,
        compressed_names,
        uncompressed_names_size,
        compressed_sizes,
        checksums,
    })
}

#[derive(Clone, Copy)]
pub enum SymlinkRules {
    Forbid,
    Ignore,
    Follow,
}

pub fn parse_symlink_rules(lit: Option<syn::Lit>) -> SymlinkRules {
    match lit {
        None => SymlinkRules::Forbid,
        Some(syn::Lit::Str(s)) => match &s.value()[..] {
            "forbid" => SymlinkRules::Forbid,
            "ignore" => SymlinkRules::Ignore,
            "follow" => SymlinkRules::Follow,
            _ => panic!("invalid/unsupported rule for symbolic links (supported rules are: forbid, ignore, follow)"),
        },
        Some(_) => panic!("invalid/unsupported rule for symbolic links (supported rules are : forbid, ignore, follow)"),
    }
}

pub fn read_dir<P: AsRef<std::path::Path>>(
    base: P,
    symlink_rules: SymlinkRules,
) -> anyhow::Result<std::vec::Vec<(smartstring::SmartString<smartstring::LazyCompact>, std::vec::Vec<u8>)>> {
    let (follow_symlinks, ignore_symlinks) = match symlink_rules {
        SymlinkRules::Forbid => (false, false),
        SymlinkRules::Ignore => (false, true),
        SymlinkRules::Follow => (true, false),
    };
    let mut assets = vec![];
    for dirent in walkdir::WalkDir::new(base.as_ref()).sort_by_file_name().follow_links(follow_symlinks) {
        // Note: sorting by file name is important to ensure the same compressed data independent of the creation/modification order of assets
        let ent = dirent?;
        if ent.file_type().is_dir() {
            continue; // ignore
        } else if ent.file_type().is_file() {
            let filename = ent
                .path()
                .strip_prefix(base.as_ref())
                .expect("child path should have parent as prefix")
                .to_str()
                .with_context(|| format!("Non-UTF-8 file name: '{}'", ent.path().display()))?;
            let data = std::fs::read(ent.path()).with_context(|| format!("Couldn't read file '{}'", ent.path().display()))?;
            assets.push((filename.into(), data))
        } else if ent.file_type().is_symlink() {
            if ignore_symlinks {
                continue; // ignore
            } else {
                return Err(anyhow::Error::msg(format!("Encountered a symbolic link: {}", ent.path().display())));
            }
        } else {
            panic!("File {} is neither directory, file, nor symbolic link.", ent.path().display());
        }
    }
    Ok(assets)
}
