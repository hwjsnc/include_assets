/// assignment of the form `identifier = literal`
pub struct KVIdentLit {
    pub ident: syn::Ident,
    pub lit: syn::Lit,
}

impl syn::parse::Parse for KVIdentLit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let _: syn::token::Eq = input.parse()?;
        let lit: syn::Lit = input.parse()?;
        Ok(KVIdentLit { ident, lit })
    }
}

/// A literal string, followed by a number of `ident = literal` arguments
pub struct IncludeDirArgs {
    pub path: syn::LitStr,
    pub opts: std::vec::Vec<KVIdentLit>,
}

impl syn::parse::Parse for IncludeDirArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path: syn::LitStr = input.parse()?;
        let lookahead = input.lookahead1();
        let opts = if lookahead.peek(syn::Token![,]) {
            let _: syn::token::Comma = input.parse()?;
            let kv = syn::punctuated::Punctuated::<KVIdentLit, syn::Token![,]>::parse_terminated(input)?;
            kv.into_iter().collect()
        } else {
            vec![]
        };
        Ok(IncludeDirArgs { path, opts })
    }
}

pub struct KVList {
    pub kvs: std::vec::Vec<KVIdentLit>,
}

impl syn::parse::Parse for KVList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            kvs: syn::punctuated::Punctuated::<KVIdentLit, syn::Token![,]>::parse_terminated(input)?
                .into_iter()
                .collect(),
        })
    }
}

pub fn kv_args_to_hashmap<I: Iterator<Item = KVIdentLit>>(kvs: I, allowed: std::collections::HashSet<&str>) -> std::collections::HashMap<&str, syn::Lit> {
    let mut result = std::collections::HashMap::new();
    for kv in kvs {
        let key = kv.ident.to_string();
        if let Some(s) = allowed.get(key.as_str()) {
            let is_new = result.insert(*s, kv.lit).is_none();
            if !is_new {
                panic!("Duplicate option {s}");
            }
        } else {
            panic!("Unknown/invalid option {key}")
        }
    }
    result
}
