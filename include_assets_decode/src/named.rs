use crate::checksum;
/// Named asset archives provide maps from path name to asset content.
/// This crate contains functionality specific to this kind of asset archives.
use crate::codec::Codec;

use crate::common::{decompress_names, decompress_ranges, u32_to_usize, u32_to_usize_range};

/// Compressed named archive
///
/// Contains the compressed asset data and all information required to uncompress it.
///
/// Users should only create these archives via the `include_dir!` macro and only read or access them via [`NamedArchive::load`].
#[derive(Clone, Copy)]
pub struct CompressedNamedArchive<C: Codec> {
    /// Compression codec with which the data was compressed
    pub codec: C,

    /// Raw compressed data
    pub data: &'static [u8],

    /// Size of the data after decompression.
    /// Limited to at most 4 GiB.
    pub uncompressed_data_size: u32,

    /// Names of the assets in some order, separated by null bytes (U+0000)
    ///
    /// The order needs to match the order of blobs in the uncompressed archive data.
    /// The final name is _not_ null-terminated.
    ///
    /// Names are currently sorted such that all files in a directory are sorted.
    /// This is for two reasons:
    /// - It likely leads to better compression if all names with the same (path) prefix are close together, and
    /// - It makes reproducible builds easier since we don't rely on file system iteration order.
    pub compressed_names: &'static [u8],

    /// Lengths of the uncompressed names (including separating null bytes)
    pub uncompressed_names_size: u32,

    /// List of asset checksums in the same order as [`CompressedNamedArchive::compressed_names`].
    pub checksums: &'static [checksum::Checksum],

    /// Compressed data sizes of the assets.
    ///
    /// Once uncompressed, these will be `u32`s (little endian) in the same order as [`CompressedNamedArchive::compressed_names`].
    pub compressed_sizes: &'static [u8],
}

/// Unpacked archive of named assets
///
/// Can be used to look up assets by name (i.e. path).
pub struct NamedArchive {
    data: std::vec::Vec<u8>,
    ranges: std::collections::HashMap<smartstring::SmartString<smartstring::LazyCompact>, std::ops::Range<u32>>,
}

impl NamedArchive {
    /// Load (decompress) compressed asset archive at runtime
    ///
    /// # Panics
    ///
    /// Panics if loading fails.
    /// This is only possible in the case of internal bugs, assuming that the compressed asset were created with the `include_dir!` macro.
    pub fn load<C: Codec>(compressed: CompressedNamedArchive<C>) -> Self {
        let CompressedNamedArchive {
            codec,
            data: compressed_data,
            uncompressed_data_size,
            compressed_names,
            uncompressed_names_size,
            checksums,
            compressed_sizes,
        } = compressed;

        // decompress data
        let data = codec.decompress_with_length(compressed_data, u32_to_usize(uncompressed_data_size));

        // decompress names and data ranges
        let names = decompress_names(&codec, compressed_names, uncompressed_names_size);
        let ranges = decompress_ranges(&codec, compressed_sizes, checksums.len());
        assert_eq!(names.len(), ranges.len(), "number of asset names should equal number of asset data ranges");

        // Data ranges were constructed in decompress_ranges.
        // We know that they are all non-overlapping, increasing, and don't leave any space.
        // We know the first range starts at 0.
        // The final range should end where the data ends.
        assert_eq!(ranges.last().map(|range| range.end).unwrap_or(0), uncompressed_data_size);

        let ranges: std::collections::HashMap<_, _> = names.into_iter().zip(ranges.into_iter()).collect();

        Self { data, ranges }
    }

    /// Get the content of the asset with the given `name`.
    ///
    /// Returns `None` if the archive does not contain an asset with this `name`.
    pub fn get<'a>(&'a self, name: &str) -> Option<&'a [u8]> {
        self.ranges.get(name).map(|range| &self.data[u32_to_usize_range(range)])
    }

    /// Returns the number of assets included in the archive.
    pub fn number_of_assets(&self) -> usize {
        self.ranges.len()
    }

    /// Returns an iterator of all asset names and contents in unspecified order.
    pub fn assets(&self) -> impl Iterator<Item = (&str, &[u8])> + ExactSizeIterator + '_ {
        self.ranges.iter().map(|(name, range)| (name.as_ref(), &self.data[u32_to_usize_range(range)]))
    }

    /// Returns true if an asset with the given `name` is included in the archive.
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Returns an iterator of all asset names in unspecified order.
    pub fn names(&self) -> impl Iterator<Item = &str> + ExactSizeIterator + '_ {
        self.ranges.keys().map(|s| s.as_ref())
    }
}

impl<S: AsRef<str>> core::ops::Index<S> for NamedArchive {
    type Output = [u8];

    /// Return the contents of the asset with the given name.
    /// Panics it the asset is not present.
    fn index(&self, s: S) -> &[u8] {
        match self.get(s.as_ref()) {
            Some(data) => data,
            None => panic!("asset '{}' not found", s.as_ref()),
        }
    }
}
