extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

use syn::{parse_macro_input, DeriveInput};

mod de;
mod ser;

#[proc_macro_derive(ReadBytes, attributes(rfont))]
pub fn read_bytes_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    de::impl_de_derive(&mut input).into()
}

#[proc_macro_derive(WriteBytes, attributes(rfont))]
pub fn write_bytes_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    ser::impl_ser_derive(&mut input).into()
}
