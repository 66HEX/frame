use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Member, parse_macro_input};

fn marked_field(input: &DeriveInput, attribute: &str) -> syn::Result<Member> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "element trait derives require a struct",
        ));
    };

    let mut found = None;

    for (idx, field) in data.fields.iter().enumerate() {
        if field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident(attribute))
        {
            let member = match &data.fields {
                Fields::Named(..) => Member::Named(field.ident.clone().expect("named field")),
                Fields::Unnamed(..) => Member::Unnamed(idx.into()),
                Fields::Unit => unreachable!(),
            };

            if found.replace(member).is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    format!("only one #[{attribute}] field is allowed"),
                ));
            }
        }
    }

    found.ok_or_else(|| {
        syn::Error::new_spanned(
            &input.ident,
            format!("derive requires a #[{attribute}] field"),
        )
    })
}

pub fn derive_styled(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let field = match marked_field(&input, "style") {
        Ok(field) => field,
        Err(error) => {
            return error.to_compile_error().into();
        }
    };

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics gpui::Styled for #name #type_generics #where_clause {
            fn style(&mut self) -> &mut gpui::StyleRefinement {
                &mut self.#field
            }
        }
    }
    .into()
}

pub fn derive_parent_element(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let field = match marked_field(&input, "children") {
        Ok(field) => field,
        Err(error) => {
            return error.to_compile_error().into();
        }
    };

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics gpui::ParentElement for #name #type_generics #where_clause {
            fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
                self.#field.extend(elements);
            }
        }
    }
    .into()
}

pub fn derive_interactive_element(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let field = match marked_field(&input, "interactivity") {
        Ok(field) => field,
        Err(error) => {
            return error.to_compile_error().into();
        }
    };

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics gpui::InteractiveElement for #name #type_generics #where_clause {
            fn interactivity(&mut self) -> &mut gpui::Interactivity {
                &mut self.#field
            }
        }
    }
    .into()
}

pub fn derive_stateful_interactive_element(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics gpui::StatefulInteractiveElement for #name #type_generics #where_clause {}
    }
    .into()
}
