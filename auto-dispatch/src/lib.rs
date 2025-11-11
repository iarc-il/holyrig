use proc_macro::TokenStream;
use quote::quote;

use crate::auto_dispatch::AutoDispatch;

mod auto_dispatch;

#[proc_macro_attribute]
pub fn auto_dispatch(_attrs: TokenStream, body: TokenStream) -> TokenStream {
    let auto_dispatch: AutoDispatch = syn::parse2(body.into()).unwrap();
    quote! { #auto_dispatch }.into()
}
