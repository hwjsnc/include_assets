use crate::checksum::{check, Checksum};
use crate::codec::Codec;
use crate::common::u32_to_usize;

/// Trait for assets that can be lookup up by enum.
///
/// This should _never_ be implemented manually, only derived.
pub trait AssetEnum: Sized {
    /// Compressed asset data
    const DATA: &'static [u8];

    /// Position of the end of the asset data for each enum within the uncompressed combined data.
    const DATA_END_OFFSETS: &'static [u32];

    /// Checksums for all assets
    const CHECKSUMS: &'static [Checksum];

    /// Type of compression codec
    type C: Codec;

    /// Compression codec with which to decompress the asset data
    const CODEC: Self::C;

    /// This method should map an enum variant to its discriminator (via `as` casting).
    ///
    /// The reason this exists is that the `Index` implementation for [`EnumArchive`] cannot perform this cast (because it doesn't know that implementers are enums)
    fn index(self) -> usize;

    /// Load (decompress) compressed data for this enum.
    fn load() -> EnumArchive<Self> {
        let mut data = vec![0u8; u32_to_usize(Self::DATA_END_OFFSETS.last().copied().unwrap_or(0))];
        Self::CODEC.decompress(Self::DATA, &mut data);
        let result = EnumArchive {
            data,
            _spooky: core::marker::PhantomData,
        };
        for i in 0..Self::CHECKSUMS.len() {
            check(result.lookup(i), &Self::CHECKSUMS[i]).expect("checksum should match");
        }

        result
    }
}

// Archive holding uncompressed data for an AssetEnum.
// User-facing documentation is in the include_assets crate.
pub struct EnumArchive<E> {
    data: std::vec::Vec<u8>,
    _spooky: core::marker::PhantomData<E>,
}

impl<E: AssetEnum> EnumArchive<E> {
    pub fn load() -> Self {
        E::load()
    }

    fn lookup(&self, i: usize) -> &[u8] {
        let end = u32_to_usize(E::DATA_END_OFFSETS[i]);
        let start = i.checked_sub(1).map(|j| E::DATA_END_OFFSETS[j]).map(u32_to_usize).unwrap_or(0);
        &self.data[start..end]
    }

    /// Apply the mapping function to the asset data.
    pub fn map<T, F: Fn(&[u8]) -> T>(&self, f: F) -> EnumMap<E, T> {
        EnumMap {
            data: (0..E::CHECKSUMS.len()).map(|i| self.lookup(i)).map(f).collect(),
            _spooky: core::marker::PhantomData,
        }
    }

    /// Apply a fallible mapping function to asset data and return an enum map if each invocation succeeds, or an `Err` otherwise.
    pub fn try_map<T, Err, F: Fn(&[u8]) -> Result<T, Err>>(&self, f: F) -> Result<EnumMap<E, T>, Err> {
        let data: Result<_, Err> = (0..E::CHECKSUMS.len()).map(|i| self.lookup(i)).map(f).collect();
        Ok(EnumMap {
            data: data?,
            _spooky: core::marker::PhantomData,
        })
    }
}

impl<E: AssetEnum> core::ops::Index<E> for EnumArchive<E> {
    type Output = [u8];

    /// Look up the asset data corresponding to the enum variant
    fn index(&self, e: E) -> &[u8] {
        self.lookup(e.index())
    }
}

/// A structure which holds a value of some type `T` for each variant of an [`AssetEnum`]
pub struct EnumMap<E: AssetEnum, T> {
    data: std::vec::Vec<T>,
    _spooky: core::marker::PhantomData<E>,
}

impl<E: AssetEnum, T> EnumMap<E, T> {
    /// Apply the mapping function to the asset data.
    pub fn map<U, F: Fn(&T) -> U>(&self, f: F) -> EnumMap<E, U> {
        EnumMap {
            data: (0..E::CHECKSUMS.len()).map(|i| f(&self.data[i])).collect(),
            _spooky: core::marker::PhantomData,
        }
    }

    /// Apply a fallible mapping function to asset data and return an enum map if each invocation succeeds, or an `Err` otherwise.
    pub fn try_map<U, Err, F: Fn(&T) -> Result<U, Err>>(&self, f: F) -> Result<EnumMap<E, U>, Err> {
        let data: Result<_, Err> = (0..E::CHECKSUMS.len()).map(|i| f(&self.data[i])).collect();
        Ok(EnumMap {
            data: data?,
            _spooky: core::marker::PhantomData,
        })
    }
}

impl<E: AssetEnum, T> core::ops::Index<E> for EnumMap<E, T> {
    type Output = T;

    /// Look up the value for the given enum variant
    fn index(&self, e: E) -> &T {
        &self.data[e.index()]
    }
}

impl<E: AssetEnum, T> core::ops::IndexMut<E> for EnumMap<E, T> {
    /// Provide an exclusive reference to the value for the given enum variant
    fn index_mut(&mut self, e: E) -> &mut T {
        &mut self.data[e.index()]
    }
}
