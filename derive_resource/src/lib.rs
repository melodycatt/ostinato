extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(Resource)]
pub fn resource_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let ast = syn::parse(input).unwrap();

    // Build the impl
    impl_resource(&ast)
}

fn impl_resource(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    // Generate an empty impl of Resource for the type
    let gene = quote! {
        impl Resource for #name {}
    };

    gene.into()
}
