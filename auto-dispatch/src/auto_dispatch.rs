use std::collections::BTreeMap;

use quote::ToTokens;
use syn::parse::Parse;
use syn::spanned::Spanned;

pub struct AutoDispatch {
    dispid_to_name: BTreeMap<i32, String>,
}

#[derive(Debug)]
enum PropertyType {
    IUnknown,
    Bstr,
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    F64,
}

struct DispatchFunc {
    get_func: Option<syn::ImplItemFn>,
    put_func: Option<syn::ImplItemFn>,
    method_func: Option<syn::ImplItemFn>,
}

fn simple_path_to_string(path: &syn::Path) -> syn::Result<&syn::PathSegment> {
    if path.segments.len() == 1 {
        Ok(path.segments.first().unwrap())
    } else {
        Err(syn::Error::new(path.span(), "Expected simple path"))
    }
}

impl AutoDispatch {
    fn new() -> Self {
        Self {
            dispid_to_name: BTreeMap::new(),
        }
    }

    fn parse_function(&mut self, func: &syn::ImplItemFn) -> syn::Result<()> {
        let id = if let [attr] = &func.attrs[..]
            && let syn::Meta::List(list) = &attr.meta
            && let Some(ident) = list.path.get_ident()
            && ident.to_string().as_str() == "id"
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
        if let Some(name) = self.dispid_to_name.get(&id)
            && name != &func_name
        {
            return Err(syn::Error::new(
                func.span(),
                format!("Duplicated id for functions `{name}`, `{func_name}`"),
            ));
        }
        self.dispid_to_name.insert(id, func.sig.ident.to_string());

        let syn::ReturnType::Type(_, output_type) = &func.sig.output else {
            return Err(syn::Error::new(
                func.span(),
                format!("Missing output type for function `{func_name}`"),
            ));
        };

        if let syn::Type::Path(type_path) = output_type.as_ref() {
            let segment = simple_path_to_string(&type_path.path)?;
            let syn::PathArguments::AngleBracketed(segment_args) = &segment.arguments else {
                return Err(syn::Error::new(
                    segment.span(),
                    format!("Invalid output type for function `{func_name}`"),
                ));
            };

            let args: Vec<_> = segment_args.args.iter().cloned().collect();

            let [
                syn::GenericArgument::Type(return_type),
                syn::GenericArgument::Type(error_type),
            ] = &args[..]
            else {
                return Err(syn::Error::new(
                    segment_args.args.span(),
                    format!("Invalid output type for function `{func_name}`"),
                ));
            };

            if let syn::Type::Path(type_path) = error_type {
                let err_type = simple_path_to_string(&type_path.path)?.ident.to_string();
                if err_type.as_str() != "HRESULT" {
                    return Err(syn::Error::new(
                        segment_args.args.span(),
                        format!(
                            "Err type of return type of `{func_name}` must be HRESULT, but it is `{err_type}`"
                        ),
                    ));
                }
            } else {
                return Err(syn::Error::new(
                    segment_args.args.span(),
                    "Invalid err type of return type",
                ));
            }

            let return_type = match return_type {
                syn::Type::Path(type_path) => {
                    let segment = simple_path_to_string(&type_path.path)?.ident.to_string();
                    let property_type = match segment.as_str() {
                        "IUnknown" => PropertyType::IUnknown,
                        "BSTR" => PropertyType::Bstr,
                        "bool" => PropertyType::Bool,
                        "u8" => PropertyType::U8,
                        "i8" => PropertyType::I8,
                        "u16" => PropertyType::U16,
                        "i16" => PropertyType::I16,
                        "u32" => PropertyType::U32,
                        "i32" => PropertyType::I32,
                        "f32" => PropertyType::F32,
                        "f64" => PropertyType::F64,
                        _ => {
                            return Err(syn::Error::new(
                                type_path.span(),
                                "Unsupported return type",
                            ));
                        }
                    };
                    Some(property_type)
                }
                syn::Type::Tuple(type_tuple) => {
                    if !type_tuple.elems.is_empty() {
                        return Err(syn::Error::new(
                            type_tuple.span(),
                            "Unsupported return type",
                        ));
                    }
                    None
                }
                _ => {
                    return Err(syn::Error::new(
                        return_type.span(),
                        "Unsupported return type",
                    ));
                }
            };
        } else {
            return Err(syn::Error::new(
                output_type.span(),
                format!("Invalid output type for function `{func_name}`"),
            ));
        }

        Ok(())
    }
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

        let mut auto_dispatch = AutoDispatch::new();

        for inner_item in impl_block.items {
            let syn::ImplItem::Fn(func) = inner_item else {
                return Err(syn::Error::new(
                    inner_item.span(),
                    "Only functions are allowed",
                ));
            };

            auto_dispatch.parse_function(&func)?;
        }

        Ok(auto_dispatch)
    }
}

impl ToTokens for AutoDispatch {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {}
}
