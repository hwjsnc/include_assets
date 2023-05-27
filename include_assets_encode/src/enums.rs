use include_assets_decode::codec::Codec;

pub struct AssetEnumOptions {
    pub enum_name: syn::Ident,
    pub base_path: syn::LitStr,
    pub compression_lit: Option<syn::Lit>,
    pub level_lit: Option<syn::Lit>,
    pub variant_paths: std::vec::Vec<syn::LitStr>,
}

pub fn check_enum_and_return_options(e: syn::ItemEnum) -> AssetEnumOptions {
    // check outer attributes of the enum
    let mut opts = std::collections::HashMap::new();
    for attr in e.attrs.iter() {
        match &attr.meta {
            syn::Meta::Path(path) => {
                if path.is_ident("archive") || path.is_ident("asset") {
                    panic!("path style attribute is not supported");
                }
                // otherwise ignore
            }
            syn::Meta::List(list) => {
                if list.path.is_ident("archive") {
                    let kv_opts: crate::parse::KVList = syn::parse2(list.tokens.clone()).unwrap();
                    for (k, v) in crate::parse::kv_args_to_hashmap(kv_opts.kvs.into_iter(), ["base_path", "compression", "level"].into_iter().collect()) {
                        opts.insert(k, v);
                    }
                } else if list.path.is_ident("asset") {
                    panic!("invalid attribute 'asset' for AssetEnum");
                } else {
                    // ignore
                }
            }
            syn::Meta::NameValue(namevalue) => {
                let path = &namevalue.path;
                match path.get_ident() {
                    Some(s) if s == "archive" || s == "asset" => {
                        panic!("{s} = value style attribute is not supported");
                    }
                    _ => {} // ignore
                }
            }
        }
    }

    let base_path = match opts.remove("base_path") {
        None => panic!("attribute base_path is missing"),
        Some(lit) => match lit {
            syn::Lit::Str(s) => s,
            _ => panic!("unexpected value for attribute base_path, expected a string literal"),
        },
    };

    // collect relative path of all variants.
    // while we're at it, ensure that all variants are unit and (most importantly) have no explicit discriminator.
    // we need enums to have discriminators 0..N!
    let mut variant_paths = vec![];
    for var in e.variants {
        let name = var.ident.to_string();
        if !matches!(var.fields, syn::Fields::Unit) {
            panic!("{name} is not a unit variant");
        }
        if var.discriminant.is_some() {
            panic!("variant {name} has an explicit discriminant, which is not allowed");
        }
        match &var.attrs[..] {
            [] => panic!("variant {name} is missing attribute"),
            [attr] => match &attr.meta {
                syn::Meta::Path(_) => panic!("invalid attribute for variant {name}"),
                syn::Meta::NameValue(_) => panic!("invalid attribute for variant {name}"),
                syn::Meta::List(list) => {
                    if !list.path.is_ident("asset") {
                        panic!("invalid attribute for variant {name}, expected 'asset'");
                    }
                    let kv_opts: crate::parse::KVList = syn::parse2(list.tokens.clone()).unwrap();
                    let mut opts = crate::parse::kv_args_to_hashmap(kv_opts.kvs.into_iter(), ["path"].into_iter().collect());
                    match opts.remove("path") {
                        None => panic!("variant {name} is missing attribute 'path'"),
                        Some(syn::Lit::Str(s)) => {
                            variant_paths.push(s);
                        }
                        Some(_) => panic!("invalid attribute for variant {name}"),
                    }
                }
            },
            _ => panic!("variant {name} has more than one attribute"),
        }
    }

    AssetEnumOptions {
        enum_name: e.ident,
        base_path,
        compression_lit: opts.remove("compression"),
        level_lit: opts.remove("level"),
        variant_paths,
    }
}

pub fn get_files(base_path: syn::LitStr, variant_paths: std::vec::Vec<syn::LitStr>) -> std::vec::Vec<std::vec::Vec<u8>> {
    let base = std::path::PathBuf::from(base_path.value());
    let mut data = vec![];
    for var_path in variant_paths {
        let name = base.join(var_path.value());
        match std::fs::read(&name) {
            Ok(blob) => data.push(blob),
            Err(err) => panic!("Couldn't read file {}: {}", name.display(), err),
        }
    }
    data
}

pub struct EnumArchive {
    pub compressed_data: std::vec::Vec<u8>,
    pub data_end_offsets: std::vec::Vec<u32>,
}

pub fn prepare_asset_archive<C: Codec + ?Sized>(codec: &C, data: std::vec::Vec<std::vec::Vec<u8>>) -> EnumArchive {
    let mut uncompressed_data = vec![];
    let mut data_end_offsets = vec![];
    for blob in data {
        uncompressed_data.extend_from_slice(blob.as_slice());
        data_end_offsets.push(u32::try_from(uncompressed_data.len()).unwrap());
    }
    let compressed_data = codec.compress(uncompressed_data.as_slice()).expect("compression should succeed");
    EnumArchive {
        compressed_data,
        data_end_offsets,
    }
}
