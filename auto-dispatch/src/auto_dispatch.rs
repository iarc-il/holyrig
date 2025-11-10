use quote::quote;
use quote::ToTokens;
use syn::parse::Parse;

pub struct AutoDispatch {}

impl Parse for AutoDispatch {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

impl ToTokens for AutoDispatch {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    }
}
