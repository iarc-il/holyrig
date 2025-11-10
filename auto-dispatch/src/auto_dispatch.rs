use quote::ToTokens;
use syn::parse::Parse;

pub struct AutoDispatch {}

impl Parse for AutoDispatch {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _impl_block: syn::Item = input.parse()?;
        Ok(Self {})
    }
}

impl ToTokens for AutoDispatch {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {
    }
}
