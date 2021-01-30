use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, DeriveInput};

mod enum_impl;
mod named_struct;
mod attr;

use enum_impl::{EnumInput};
use named_struct::NamedStructInput;

#[proc_macro_derive(Parse, attributes(nommy))]
pub fn derive_parse(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let DeriveInput { attrs, vis: _, ident, generics, data } = input;

    match data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(fields) => NamedStructInput::new(ident, generics, attrs, fields).process().into_token_stream(),
            syn::Fields::Unnamed(_) =>  unimplemented!(),
            syn::Fields::Unit => unimplemented!(),
        },
        syn::Data::Enum(enum_data) => EnumInput::new(ident, generics, attrs, enum_data).process().into_token_stream(),
        syn::Data::Union(_) => panic!("unions not supported"),
    }
    .into()
}
