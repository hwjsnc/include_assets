use anyhow::Context as _;

use include_assets_decode::checksum::compute_checksum;
use include_assets_decode::codec::Codec;

pub fn compress_sizes<C: Codec + ?Sized, S: AsRef<str>, I: Iterator<Item = (S, usize)>>(codec: &C, sizes: I) -> anyhow::Result<std::vec::Vec<u8>> {
    let mut sizes_vec = vec![];
    for (name, size) in sizes {
        let size: u32 = size
            .try_into()
            .with_context(|| format!("asset {} is too big ({} bytes)", name.as_ref(), size))?;
        sizes_vec.extend_from_slice(&size.to_le_bytes());
    }
    // ensure that the uncompressed lengths aren't longer than 4 GiB (i.e. the length fits in a u32)
    if u32::try_from(sizes_vec.len()).is_err() {
        return Err(anyhow::Error::msg(format!(
            "too many assets: size of uncompressed asset sizes is too big ({} bytes)",
            sizes_vec.len()
        )));
    }
    codec.compress(sizes_vec.as_slice()).context("couldn't compress asset data sizes")
}

pub fn compress_names<C: Codec + ?Sized, S: AsRef<str>, I: Iterator<Item = S>>(codec: &C, mut names: I) -> anyhow::Result<(std::vec::Vec<u8>, u32)> {
    let mut uncompressed_names = vec![];
    if let Some(first) = names.next() {
        assert!(!first.as_ref().as_bytes().contains(&0));
        uncompressed_names.extend_from_slice(first.as_ref().as_bytes());
        for name in names {
            assert!(!name.as_ref().as_bytes().contains(&0));
            uncompressed_names.extend_from_slice(&[0u8]);
            uncompressed_names.extend_from_slice(name.as_ref().as_bytes());
        }
    };
    let uncompressed_size = u32::try_from(uncompressed_names.len())
        .map_err(|_| anyhow::Error::msg(format!("uncompressed names are too long ({} bytes)", uncompressed_names.len())))?;
    let compressed_names = codec.compress(uncompressed_names.as_slice()).context("couldn't compress asset names")?;
    Ok((compressed_names, uncompressed_size))
}

/// Wrapper for `anyhow::Error`, required because `anyhow::Error` doesn't `impl std::error::Error`.
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct MyError(anyhow::Error);

pub struct DynCodec<C> {
    codec: C,
}

impl<C> DynCodec<C> {
    pub fn new(codec: C) -> Self {
        Self { codec }
    }
}

impl<C: Codec> Codec for DynCodec<C> {
    type CompressionError = MyError;
    type DecompressionError = MyError;

    fn compress(&self, data: &[u8]) -> Result<std::vec::Vec<u8>, MyError> {
        self.codec.compress(data).map_err(anyhow::Error::msg).map_err(MyError)
    }

    fn decompress_checked(&self, src: &[u8], dst: &mut [u8]) -> Result<(), MyError> {
        self.codec.decompress_checked(src, dst).map_err(anyhow::Error::msg).map_err(MyError)
    }
}

pub fn parse_codec(
    compression: Option<syn::Lit>,
    level: Option<syn::Lit>,
) -> (
    Box<dyn Codec<CompressionError = MyError, DecompressionError = MyError>>,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    let compression_string = if let Some(lit) = compression {
        if let syn::Lit::Str(s) = lit {
            s.value()
        } else {
            panic!("invalid compression option (expected a string literal)");
        }
    } else {
        let available = [
            #[cfg(feature = "zstd")]
            "zstd",
            #[cfg(feature = "lz4")]
            "lz4",
            #[cfg(feature = "deflate")]
            "deflate",
            "uncompressed",
        ];
        available[0].to_owned()
    };

    match &compression_string[..] {
        "uncompressed" => {
            if level.is_some() {
                panic!("compression 'uncompressed' does not have levels");
            } else {
                let codec = DynCodec::new(include_assets_decode::codec::Uncompressed {});
                let expr = quote::quote! { ::include_assets::do_not_use_this_directly::codec::Uncompressed{} };
                let type_expr = quote::quote! { ::include_assets::do_not_use_this_directly::codec::Uncompressed };
                let boxed_codec: Box<dyn Codec<CompressionError = MyError, DecompressionError = MyError>> = Box::new(codec);
                (boxed_codec, expr, type_expr)
            }
        }
        #[cfg(feature = "lz4")]
        "lz4" => {
            if level.is_some() {
                panic!("compression 'lz4' does not (currently) support levels");
            } else {
                let codec = DynCodec::new(include_assets_decode::codec::Lz4 {});
                let expr = quote::quote! {::include_assets::do_not_use_this_directly::codec::Lz4{} };
                let type_expr = quote::quote! { ::include_assets::do_not_use_this_directly::codec::Lz4 };
                let boxed_codec: Box<dyn Codec<CompressionError = MyError, DecompressionError = MyError>> = Box::new(codec);
                (boxed_codec, expr, type_expr)
            }
        }
        #[cfg(feature = "deflate")]
        "deflate" => {
            let level: u8 = match level {
                None => 2,
                Some(syn::Lit::Int(int)) => {
                    if let Ok(n) = int.base10_parse() {
                        n
                    } else {
                        panic!("Invalid compression level {}", int);
                    }
                }
                _ => panic!("Invalid compression level"),
            };
            let codec = DynCodec::new(include_assets_decode::codec::Deflate { level });
            let expr = quote::quote! {::include_assets::do_not_use_this_directly::codec::Deflate{ level: #level } };
            let type_expr = quote::quote! { ::include_assets::do_not_use_this_directly::codec::Deflate };
            let boxed_codec: Box<dyn Codec<CompressionError = MyError, DecompressionError = MyError>> = Box::new(codec);
            (boxed_codec, expr, type_expr)
        }
        #[cfg(feature = "zstd")]
        "zstd" => {
            let level: i32 = match level {
                None => 5,
                Some(syn::Lit::Int(int)) => {
                    if let Ok(n) = int.base10_parse() {
                        n
                    } else {
                        panic!("Invalid compression level {}", int);
                    }
                }
                _ => panic!("Invalid compression level"),
            };
            let codec = DynCodec::new(include_assets_decode::codec::Zstd { level });
            let expr = quote::quote_spanned! {proc_macro2::Span::mixed_site()=> ::include_assets::do_not_use_this_directly::codec::Zstd{ level: #level } };
            let type_expr = quote::quote! { ::include_assets::do_not_use_this_directly::codec::Zstd };
            let boxed_codec: Box<dyn Codec<CompressionError = MyError, DecompressionError = MyError>> = Box::new(codec);
            (boxed_codec, expr, type_expr)
        }
        s => panic!("invalid/unsupported compression '{s}'"),
    }
}

pub fn checksums_tokens<T: AsRef<[u8]>, I: Iterator<Item = T>>(asset_data: I) -> proc_macro2::TokenStream {
    let checksums: std::vec::Vec<_> = asset_data
        .map(|data| compute_checksum(data.as_ref()))
        .map(|checksum| quote::quote! { [#(#checksum),*] })
        .collect();
    quote::quote! {&[#(#checksums),*]}
}
