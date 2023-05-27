pub(crate) mod common;
pub(crate) mod enums;
pub(crate) mod named;
pub(crate) mod parse;

use include_assets_decode::codec::Codec;
use std::borrow::Borrow as _;

#[proc_macro]
pub fn include_dir(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::env::set_current_dir(manifest_dir).unwrap();

    let args = syn::parse_macro_input!(tokens as parse::IncludeDirArgs);
    let opts = parse::kv_args_to_hashmap(args.opts.into_iter(), ["compression", "level", "links"].into_iter().collect());

    //println!("current directory: {}", std::env::current_dir().unwrap().display());
    //println!("path: {}", args.path.value());

    let (codec, codec_tokens, _codec_type_tokens) = common::parse_codec(opts.get("compression").cloned(), opts.get("level").cloned());
    let symlink_rules = named::parse_symlink_rules(opts.get("links").cloned());

    let named::NamedArchive {
        compressed_data,
        uncompressed_data_size,
        compressed_names,
        uncompressed_names_size,
        compressed_sizes,
        checksums,
    } = named::prepare_named_archive(
        codec.borrow() as &dyn Codec<CompressionError = common::MyError, DecompressionError = common::MyError>,
        named::read_dir(args.path.value(), symlink_rules).unwrap(),
    )
    .unwrap();

    let data_token = syn::LitByteStr::new(&compressed_data, proc_macro2::Span::call_site());
    let names_token = syn::LitByteStr::new(&compressed_names, proc_macro2::Span::call_site());
    let checksums_token = common::checksums_tokens(checksums.into_iter());
    let sizes_token = syn::LitByteStr::new(&compressed_sizes, proc_macro2::Span::call_site());

    quote::quote! {
        ::include_assets::CompressedNamedArchive {
            codec: #codec_tokens,
            data: #data_token,
            uncompressed_data_size: #uncompressed_data_size,
            compressed_names: #names_token,
            uncompressed_names_size: #uncompressed_names_size,
            checksums: #checksums_token,
            compressed_sizes: #sizes_token
        }
    }
    .into()
}

#[proc_macro_derive(AssetEnum, attributes(archive, asset))]
pub fn derive_asset_enum(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::env::set_current_dir(manifest_dir).unwrap();

    let e = syn::parse_macro_input!(tokens as syn::ItemEnum);

    let enums::AssetEnumOptions {
        enum_name,
        base_path,
        compression_lit,
        level_lit,
        variant_paths,
    } = enums::check_enum_and_return_options(e);

    let (codec, codec_expr, codec_type) = common::parse_codec(compression_lit, level_lit);

    let file_data = enums::get_files(base_path, variant_paths);
    let checksums_token = common::checksums_tokens(file_data.iter());
    let enums::EnumArchive {
        compressed_data,
        data_end_offsets,
    } = enums::prepare_asset_archive(
        codec.borrow() as &dyn Codec<CompressionError = common::MyError, DecompressionError = common::MyError>,
        file_data,
    );
    let data_token = syn::LitByteStr::new(&compressed_data, proc_macro2::Span::call_site());

    quote::quote! {
        impl include_assets::AssetEnum for #enum_name {
            const DATA: &'static [u8] = #data_token;
            const DATA_END_OFFSETS: &'static [u32] = &[#(#data_end_offsets),*];
            const CHECKSUMS: &'static [include_assets::do_not_use_this_directly::Checksum] = #checksums_token;
            type C = #codec_type;
            const CODEC: Self::C = #codec_expr;
            fn index(self) -> usize {
                self as usize
            }
        }
    }
    .into()
}
