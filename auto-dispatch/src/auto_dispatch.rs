use std::collections::BTreeMap;

use proc_macro2::Span;
use quote::ToTokens;
use quote::quote;
use syn::Ident;
use syn::parse::Parse;
use syn::spanned::Spanned;

type DispId = i32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    property_type: Option<PropertyType>,
    get_func: Option<syn::ImplItemFn>,
    put_func: Option<syn::ImplItemFn>,
    // method_func: Option<syn::ImplItemFn>,
}

pub struct AutoDispatch {
    target: String,
    dispid_to_name: BTreeMap<DispId, String>,
    dispid_to_const: BTreeMap<DispId, Ident>,
    dispatch_funcs: BTreeMap<DispId, DispatchFunc>,
}

fn simple_path_to_string(path: &syn::Path) -> syn::Result<&syn::PathSegment> {
    if path.segments.len() == 1 {
        Ok(path.segments.first().unwrap())
    } else {
        Err(syn::Error::new(path.span(), "Expected simple path"))
    }
}

impl AutoDispatch {
    fn new(target: String) -> Self {
        Self {
            target,
            dispid_to_name: BTreeMap::new(),
            dispid_to_const: BTreeMap::new(),
            dispatch_funcs: BTreeMap::new(),
        }
    }

    fn parse_id_attribute(&mut self, func: &syn::ImplItemFn) -> syn::Result<DispId> {
        let id = if let [attr] = &func.attrs[..]
            && let syn::Meta::List(list) = &attr.meta
            && let Some(ident) = list.path.get_ident()
            && ident.to_string().as_str() == "id"
            && let syn::Lit::Int(id) = syn::parse::Parser::parse2(
                |input: syn::parse::ParseStream<'_>| input.parse::<syn::Lit>(),
                list.tokens.clone(),
            )? {
            id.base10_parse::<DispId>()?
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
        let name = func.sig.ident.to_string();

        self.dispid_to_const.insert(
            id,
            Ident::new(
                format!("{}_DISPID", name.to_uppercase()).as_str(),
                func.sig.ident.span(),
            ),
        );
        self.dispid_to_name.insert(id, name);

        Ok(id)
    }

    fn parse_property_type(property_type: &syn::Type) -> syn::Result<Option<PropertyType>> {
        match property_type {
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
                        return Err(syn::Error::new(type_path.span(), "Unsupported return type"));
                    }
                };
                Ok(Some(property_type))
            }
            syn::Type::Tuple(type_tuple) => {
                if type_tuple.elems.is_empty() {
                    Ok(None)
                } else {
                    Err(syn::Error::new(type_tuple.span(), "Unsupported type"))
                }
            }
            _ => Err(syn::Error::new(property_type.span(), "Unsupported type")),
        }
    }

    fn parse_return_type(func: &syn::ImplItemFn) -> syn::Result<Option<PropertyType>> {
        let func_name = func.sig.ident.to_string();

        let syn::ReturnType::Type(_, output_type) = &func.sig.output else {
            return Err(syn::Error::new(
                func.span(),
                format!("Missing output type for function `{func_name}`"),
            ));
        };

        let syn::Type::Path(type_path) = output_type.as_ref() else {
            return Err(syn::Error::new(
                output_type.span(),
                format!("Invalid output type for function `{func_name}`"),
            ));
        };

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

        Self::parse_property_type(return_type)
    }

    fn parse_input_type(func: &syn::ImplItemFn) -> syn::Result<Option<PropertyType>> {
        let args: Vec<_> = func.sig.inputs.iter().cloned().collect();

        let (receiver, maybe_arg) = match &args[..] {
            [syn::FnArg::Receiver(receiver), syn::FnArg::Typed(arg)] => (receiver, Some(arg)),
            [syn::FnArg::Receiver(receiver)] => (receiver, None),
            _ => {
                return Err(syn::Error::new(
                    func.sig.inputs.span(),
                    "Invalid function args",
                ));
            }
        };

        match &receiver.reference {
            None | Some((_, Some(_))) => {
                return Err(syn::Error::new(
                    receiver.span(),
                    "self parameter must be `&self`",
                ));
            }
            Some((_, None)) => {}
        }
        if receiver.mutability.is_some() {
            return Err(syn::Error::new(
                receiver.span(),
                "self parameter must not be mutable",
            ));
        }

        if receiver.colon_token.is_some() {
            return Err(syn::Error::new(
                receiver.span(),
                "self parameter must not be typed",
            ));
        }

        if let Some(arg) = maybe_arg {
            Self::parse_property_type(&arg.ty)
        } else {
            Ok(None)
        }
    }

    fn parse_function(&mut self, func: &syn::ImplItemFn) -> syn::Result<()> {
        let id = self.parse_id_attribute(func)?;
        let return_type = Self::parse_return_type(func)?;
        let input_type = Self::parse_input_type(func)?;
        match (input_type, return_type) {
            (None, Some(return_type)) => {
                if let Some(dispatch_func) = self.dispatch_funcs.get_mut(&id) {
                    if dispatch_func.get_func.is_some() {
                        return Err(syn::Error::new(
                            func.span(),
                            "Duplicated get_property function",
                        ));
                    }
                    if dispatch_func.property_type.unwrap() != return_type {
                        return Err(syn::Error::new(func.span(), "Mismatch in property type"));
                    }
                    dispatch_func.get_func = Some(func.clone());
                } else {
                    self.dispatch_funcs.insert(
                        id,
                        DispatchFunc {
                            property_type: Some(return_type),
                            get_func: Some(func.clone()),
                            put_func: None,
                        },
                    );
                }
            }
            (Some(input_type), None) => {
                if let Some(dispatch_func) = self.dispatch_funcs.get_mut(&id) {
                    if dispatch_func.put_func.is_some() {
                        return Err(syn::Error::new(
                            func.span(),
                            "Duplicated set_property function",
                        ));
                    }
                    if dispatch_func.property_type.unwrap() != input_type {
                        return Err(syn::Error::new(func.span(), "Mismatch in property type"));
                    }
                    dispatch_func.put_func = Some(func.clone());
                } else {
                    self.dispatch_funcs.insert(
                        id,
                        DispatchFunc {
                            property_type: Some(input_type),
                            get_func: None,
                            put_func: Some(func.clone()),
                        },
                    );
                }
            }
            _ => {
                return Err(syn::Error::new(
                    func.span(),
                    "Function can be either property get or property set",
                ));
            }
        }
        Ok(())
    }

    fn generate_dispids(&self, impl_struct_ident: &Ident) -> proc_macro2::TokenStream {
        let mut dispids: Vec<_> = self.dispid_to_const.iter().collect();
        dispids.sort();

        let dispids_consts: Vec<_> = dispids
            .iter()
            .map(|(dispid, const_name)| {
                quote! { const #const_name: i32 = #dispid; }
            })
            .collect();

        quote! {
            impl #impl_struct_ident {
                #(#dispids_consts)*
            }
        }
    }

    fn generate_get_ids_of_names(&self) -> proc_macro2::TokenStream {
        let match_arms = self.dispid_to_name.iter().map(|(dispid, name)| {
            let const_name = &self.dispid_to_const[dispid];
            quote! {
                #name => Self::#const_name,
            }
        });

        quote! {
            fn GetIDsOfNames(
                &self,
                _riid: *const GUID,
                rgsznames: *const PCWSTR,
                cnames: u32,
                _lcid: u32,
                rgdispid: *mut i32,
            ) -> Result<()> {
                unsafe {
                    if rgsznames.is_null() || rgdispid.is_null() {
                        return Err(E_INVALIDARG.into());
                    }

                    for i in 0..cnames {
                        let name_ptr = *rgsznames.add(i as usize);
                        let name = name_ptr.to_string().unwrap_or_default().to_uppercase();

                        let dispid = match name.as_str() {
                            #(#match_arms)*
                            _ => return Err(DISP_E_MEMBERNOTFOUND.into()),
                        };

                        *rgdispid.add(i as usize) = dispid;
                    }

                    Ok(())
                }
            }
        }
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

        let target = if let syn::Type::Path(type_path) = impl_block.self_ty.as_ref() {
            simple_path_to_string(&type_path.path)?.ident.to_string()
        } else {
            return Err(syn::Error::new(
                impl_block.self_ty.span(),
                "Invalid type name to implement",
            ));
        };

        let mut auto_dispatch = AutoDispatch::new(target);

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
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let impl_struct_ident =
            Ident::new(format!("{}_Impl", self.target).as_str(), Span::call_site());

        let dispids_consts = self.generate_dispids(&impl_struct_ident);
        let get_ids_of_names = self.generate_get_ids_of_names();

        let result = quote! {
            #dispids_consts

            impl IDispatch_Impl for #impl_struct_ident {
                fn GetTypeInfoCount(&self) -> Result<u32> {
                    Ok(0)
                }

                fn GetTypeInfo(&self, _itinfo: u32, _lcid: u32) -> Result<ITypeInfo> {
                    Err(E_NOTIMPL.into())
                }

                #get_ids_of_names

                fn Invoke(
                    &self,
                    dispidmember: i32,
                    _riid: *const GUID,
                    _lcid: u32,
                    wflags: DISPATCH_FLAGS,
                    pdispparams: *const DISPPARAMS,
                    pvarresult: *mut VARIANT,
                    _pexcepinfo: *mut EXCEPINFO,
                    _puargerr: *mut u32,
                ) -> Result<()> {
                    todo!()
                }
            }
        };
        tokens.extend(result);
    }
}
