use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{Data, DeriveInput, Fields, LitStr};

pub fn impl_ser_derive(input: &mut DeriveInput) -> TokenStream {
    let ident = &input.ident;
    match &input.data {
        Data::Struct(d) => {
            let field_serialize = ser_fields(&d.fields);
            quote! {
                #[automatically_derived]
                impl WriteBytes for #ident {
                    fn write_to(&self, writer: &mut Writer) -> Result<(), FontError> {
                        #(#field_serialize)*
                        Ok(())
                    }
                }
            }
        }
        _ => quote! {},
    }
}

fn ser_fields(fields: &Fields) -> Vec<TokenStream> {
    let mut field_capacity = HashMap::new();

    for field in fields.iter() {
        let name = &field.ident;

        for attr in &field.attrs {
            if attr.path().is_ident("rfont")
                && let Err(e) = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("capacity_with") {
                        let item: LitStr = meta.value()?.parse()?;
                        field_capacity.insert(item.value(), name);
                    }
                    Ok(())
                })
            {
                println!("error: {:?}", e)
            };
        }
    }

    fields
        .iter()
        .map(|field| {
            let name = &field.ident;
            let Some(name_str) = name.as_ref() else {
                return quote!({});
            };
            // let ty = &field.ty;

            let f = field_capacity.get(&name_str.to_string());
            if f.is_some() {
                return quote! {
                    writer.write_u16(self.#f.len() as u16)?;
                };
            }

            quote! { self.#name.write_to(writer)?; }
        })
        .collect()
}
