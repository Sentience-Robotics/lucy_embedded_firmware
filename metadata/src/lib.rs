use proc_macro::TokenStream;
use std::sync::atomic::{AtomicU16, Ordering};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};


static COUNTER: AtomicU16 = AtomicU16::new(0);

#[proc_macro_derive(GenerateMeta, attributes(meta))]
pub fn derive_generate_meta(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let vis = &input.vis;
    let meta_name = format_ident!("{}Meta", name);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("GenerateMeta doesn't support unnamed fields")
        },
        _ => panic!("GenerateMeta can only be derived for structs"),
    };

    let mut counter: u16 = 0;

    for field in fields {
        let has_meta = field.attrs.iter().any(|attr| attr.path().is_ident("meta"));
        if has_meta {
            counter += 1;
        }
    }

    let expanded = quote! {
        static __REGISTER_COUNTER: AtomicU16 = AtomicU16::new(0);

        #vis struct #meta_name {
            pub nb_register: u16,
            pub base_register: u16,
            pub inner: #name
        }

        impl ::core::ops::Deref for #meta_name {
            type Target = #name;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl ::core::ops::DerefMut for #meta_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }

        impl #name {
            #vis fn new_with_meta() -> #meta_name {
                let base_register = __REGISTER_COUNTER.load(Ordering::SeqCst);
                __REGISTER_COUNTER.fetch_add(#counter, Ordering::SeqCst);

                #meta_name {
                    nb_register: #counter,
                    base_register: base_register,
                    inner: Self::default()
                }
            }
        }
    };

    TokenStream::from(expanded)
}
