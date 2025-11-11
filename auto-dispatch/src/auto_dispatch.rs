use std::collections::BTreeMap;

use proc_macro2::Span;
use quote::ToTokens;
use syn::parse::Parse;
use syn::spanned::Spanned;

pub struct AutoDispatch {
    dispid_to_name: BTreeMap<i32, String>,
}

impl Parse for AutoDispatch {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let item: syn::Item = input.parse()?;

        let syn::Item::Impl(impl_block) = item else {
            return Err(syn::Error::new(item.span(), "Expected impl block"));
        };

        if impl_block.trait_.is_some() {
            return Err(syn::Error::new(
                impl_block.span(),
                "impl <trait> for <type> { ... } is not allowed, only impl <type> { ...  }",
            ));
        }

        let id_ident = syn::Ident::new("id", Span::call_site());

        let mut dispid_to_name: BTreeMap<i32, String> = BTreeMap::new();

        for inner_item in impl_block.items {
            let syn::ImplItem::Fn(func) = inner_item else {
                return Err(syn::Error::new(
                    inner_item.span(),
                    "Only functions are allowed",
                ));
            };

            let id = if let [attr] = &func.attrs[..]
                && let syn::Meta::List(list) = &attr.meta
                && let Some(ident) = list.path.get_ident()
                && ident == &id_ident
                && let syn::Lit::Int(id) = syn::parse::Parser::parse2(
                    |input: syn::parse::ParseStream<'_>| input.parse::<syn::Lit>(),
                    list.tokens.clone(),
                )? {
                id.base10_parse::<i32>()?
            } else {
                return Err(syn::Error::new(
                    func.span(),
                    "Expected id attribute: #[id(<dispid>)]",
                ));
            };

            let func_name = func.sig.ident.to_string();
            if let Some(name) = dispid_to_name.get(&id)
                && name != &func_name
            {
                return Err(syn::Error::new(
                    func.span(),
                    format!("Duplicated id for functions `{name}`, `{func_name}`"),
                ));
            }
            dispid_to_name.insert(id, func.sig.ident.to_string());
        }

        println!("{dispid_to_name:#?}");
        Ok(Self { dispid_to_name })
    }
}

impl ToTokens for AutoDispatch {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {}
}
