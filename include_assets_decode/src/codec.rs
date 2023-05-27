/// Compression codec for the `include_assets` crate
pub trait Codec {
    /// Errors that might occur during compression
    type CompressionError: std::error::Error + Send + Sync + 'static; // Send + Sync + 'static is for use with the anyhow crate.
    /// Errors that might occur during decompression
    type DecompressionError: std::error::Error + Send + Sync + 'static;

    /// Compress data to a newly allocated vector.
    fn compress(&self, data: &[u8]) -> Result<std::vec::Vec<u8>, Self::CompressionError>;

    /// Decompress data in `src` to `dst`.
    ///
    /// Fails if the length of `dst` doesn't exactly match the length of the uncompressed data.
    ///
    /// If decompression fails for any reason, the contents of `dst` are unspecified.
    fn decompress_checked(&self, src: &[u8], dst: &mut [u8]) -> Result<(), Self::DecompressionError>;

    /// Like [`Codec::decompress_checked`], but panics on error.
    fn decompress(&self, src: &[u8], dst: &mut [u8]) {
        self.decompress_checked(src, dst).expect("decompression should succeed")
    }

    /// Decompresses data into a new vector with the given length.
    /// Panics on error.
    fn decompress_with_length(&self, src: &[u8], len: usize) -> std::vec::Vec<u8> {
        let mut dst = vec![0u8; len];
        self.decompress(src, &mut dst);
        dst
    }
}

/// No compression whatsoever
#[derive(Debug, Clone, Copy)]
pub struct Uncompressed {}

#[derive(Debug, Clone, Copy)]
pub struct UncompressedSizeMismatch {
    expected: usize,
    actual: usize,
}

impl core::fmt::Display for UncompressedSizeMismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "unexpected uncompressed size: expected {}, got {}", self.expected, self.actual)
    }
}

impl std::error::Error for UncompressedSizeMismatch {}

impl Codec for Uncompressed {
    type CompressionError = std::convert::Infallible;
    type DecompressionError = UncompressedSizeMismatch;

    fn compress(&self, data: &[u8]) -> Result<std::vec::Vec<u8>, Self::CompressionError> {
        Ok(data.to_vec())
    }

    /// Copy data from `src` to `dst`
    ///
    /// This can only fail if the lengths of `src` and `dst` do not match.
    fn decompress_checked(&self, src: &[u8], dst: &mut [u8]) -> Result<(), Self::DecompressionError> {
        if dst.len() == src.len() {
            dst.copy_from_slice(src);
            Ok(())
        } else {
            Err(UncompressedSizeMismatch {
                expected: dst.len(),
                actual: src.len(),
            })
        }
    }
}

#[cfg(feature = "lz4")]
/// lz4 block compression
#[derive(Debug, Clone, Copy)]
pub struct Lz4 {}

#[cfg(feature = "lz4")]
impl Codec for Lz4 {
    type CompressionError = std::convert::Infallible;
    type DecompressionError = lz4_flex::block::DecompressError;

    fn compress(&self, data: &[u8]) -> Result<std::vec::Vec<u8>, Self::CompressionError> {
        Ok(lz4_flex::block::compress(data))
    }

    fn decompress_checked(&self, src: &[u8], dst: &mut [u8]) -> Result<(), Self::DecompressionError> {
        let uncompressed_size = lz4_flex::block::decompress_into(src, dst)?;
        if uncompressed_size != dst.len() {
            Err(lz4_flex::block::DecompressError::UncompressedSizeDiffers {
                expected: dst.len(),
                actual: uncompressed_size,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "zstd")]
/// zstd compression
#[derive(Debug, Clone, Copy)]
pub struct Zstd {
    /// Zstd compression level.
    ///
    /// Higher is better compression with slower speed.
    /// Level 5 is recommended.
    pub level: i32,
}

#[cfg(feature = "zstd")]
impl Codec for Zstd {
    type CompressionError = std::io::Error;
    type DecompressionError = std::io::Error;

    fn compress(&self, data: &[u8]) -> Result<std::vec::Vec<u8>, Self::CompressionError> {
        zstd::bulk::compress(data, self.level)
    }

    fn decompress_checked(&self, src: &[u8], dst: &mut [u8]) -> Result<(), Self::DecompressionError> {
        let uncompressed_size = zstd::bulk::decompress_to_buffer(src, dst)?;
        if uncompressed_size != dst.len() {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                UncompressedSizeMismatch {
                    expected: dst.len(),
                    actual: uncompressed_size,
                },
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "deflate")]
/// raw DEFLATE compression (no wrapper format)
#[derive(Debug, Clone, Copy)]
pub struct Deflate {
    /// Compression level
    ///
    /// Higher is better compression with slower speed.
    /// 0 is uncompressed, 10 is the maximum.
    pub level: u8,
}

#[cfg(feature = "deflate")]
/// yazi::Error doesn't implement std::error::Error, so we wrap it and implement it ourselves
pub struct YaziError(yazi::Error);

#[cfg(feature = "deflate")]
impl core::fmt::Debug for YaziError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        <yazi::Error as core::fmt::Debug>::fmt(&self.0, f)
    }
}

#[cfg(feature = "deflate")]
impl core::fmt::Display for YaziError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match &self.0 {
            yazi::Error::Underflow => write!(f, "yazi error: not enough input was provided"),
            yazi::Error::InvalidBitstream => write!(f, "yazi error: invalid bitstream"),
            yazi::Error::Overflow => write!(f, "yazi error: output buffer was too small"),
            yazi::Error::Finished => write!(f, "yazi error: attempted to write into a finished stream"),
            yazi::Error::Io(err) => write!(f, "yazi io error: {}", err),
        }
    }
}

#[cfg(feature = "deflate")]
impl std::error::Error for YaziError {}

#[cfg(feature = "deflate")]
impl Codec for Deflate {
    type CompressionError = YaziError;
    type DecompressionError = YaziError;

    fn compress(&self, data: &[u8]) -> Result<std::vec::Vec<u8>, Self::CompressionError> {
        yazi::compress(data, yazi::Format::Raw, yazi::CompressionLevel::Specific(self.level)).map_err(YaziError)
    }

    fn decompress_checked(&self, src: &[u8], dst: &mut [u8]) -> Result<(), Self::DecompressionError> {
        let mut decoder = yazi::Decoder::new();
        let mut stream = decoder.stream_into_buf(dst);
        // Write compressed bytes into the decoder stream.
        // This will finish successfully once all bytes are decoded.
        // It will finish with an error if the destination buffer is too short.
        let compressed_written = std::io::copy(&mut std::io::Cursor::new(src), &mut stream)
            .map_err(yazi::Error::Io)
            .map_err(YaziError)?;
        assert_eq!(
            usize::try_from(compressed_written),
            Ok(src.len()),
            "number of bytes decompressed should equal compressed size"
        );
        // Flush remaining uncompressed output
        let (uncompressed_size, checksum) = stream.finish().map_err(YaziError)?;
        assert!(checksum.is_none(), "checksum should not be present for raw stream");
        // Check if the output buffer has been fully overwritten
        if usize::try_from(uncompressed_size) != Ok(dst.len()) {
            Err(YaziError(yazi::Error::Underflow))
        } else {
            Ok(())
        }
    }
}
