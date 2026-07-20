// use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, LitInt, LitStr};

struct Attr<T> {
    value: Option<T>,
}

impl<T> Attr<T> {
    fn new() -> Self {
        Self { value: None }
    }
    fn set(&mut self, value: T) {
        self.value = Some(value);
    }
    fn get(self) -> Option<T> {
        self.value
    }
    fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}

pub fn impl_de_derive(input: &mut DeriveInput) -> TokenStream {
    let ident = &input.ident;

    match &input.data {
        Data::Struct(d) => {
            match de_fields(&d.fields) {
                Ok(body) => {
                    // 区分命名结构体和元组结构体
                    let constructor = match &d.fields {
                        Fields::Named(_) => {
                            let names: Vec<_> = d
                                .fields
                                .iter()
                                .map(|field| field.ident.as_ref().unwrap())
                                .collect();
                            quote! { #ident { #(#names,)* } }
                        }
                        Fields::Unnamed(_) => {
                            let indices: Vec<_> =
                                (0..d.fields.len()).map(syn::Index::from).collect();
                            quote! { #ident(#(#indices),*) }
                        }
                        Fields::Unit => {
                            quote! { #ident }
                        }
                    };

                    quote! {
                        #[automatically_derived]
                        #[allow(non_snake_case)]
                        impl<'a> ReadBytes<'a> for #ident {
                            fn read_from(reader: &mut Reader<'a>) -> Result<Self, FontError> {
                                #(#body)*
                                Ok(#constructor)
                            }
                        }
                    }
                }
                Err(e) => e.into_compile_error(),
            }
        }
        _ => Error::new_spanned(input, "derive macro only supports structs").to_compile_error(),
    }
}

fn de_fields(fields: &Fields) -> syn::Result<Vec<TokenStream>> {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ty = &field.ty;

            // 对于命名结构体，使用字段名；对于元组结构体，使用索引变量名
            let var_name = if let Some(ident) = &field.ident {
                quote! { #ident }
            } else {
                // 为元组字段生成临时变量名 _field_0, _field_1, ...
                let var_ident =
                    syn::Ident::new(&format!("_field_{}", index), proc_macro2::Span::call_site());
                quote! { #var_ident }
            };

            let mut capacity_field = Attr::new();
            let mut capacity_with_field = Attr::new();
            for attr in &field.attrs {
                if attr.path().is_ident("rfont") {
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("capacity") {
                            let item: LitInt = meta.value()?.parse()?;
                            capacity_field.set(item);
                            return Ok(());
                        }
                        if meta.path.is_ident("capacity_with") {
                            let item: LitStr = meta.value()?.parse()?;
                            capacity_with_field.set(item);
                            return Ok(());
                        }
                        Ok(())
                    })?;
                }
            }

            if !capacity_field.is_empty() {
                let Some(v) = capacity_field.get() else {
                    return Ok(quote!());
                };
                return Ok(quote! {
                    let #var_name: #ty = reader.read_array::<#ty>(#v)?;
                });
            }

            if !capacity_with_field.is_empty() {
                let Some(v) = capacity_with_field.get() else {
                    return Ok(quote!());
                };
                // 注意：capacity_with 在元组结构中可能无法工作，因为需要引用前面的字段
                return Ok(quote! {
                    let #var_name: #ty = reader.read_array::<#ty>(#v as usize)?;
                });
            }

            Ok(quote! { let #var_name: #ty = ReadBytes::read_from(reader)?; })
        })
        .collect::<syn::Result<Vec<_>>>()
}
