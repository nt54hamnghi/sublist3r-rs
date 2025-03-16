use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(Extract, attributes(extract))]
pub fn extract_derive_macro(item: TokenStream) -> TokenStream {
    // generate
    impl_extract_trait(item).unwrap_or_else(|e| e.write_errors().into())
}

#[derive(darling::FromDeriveInput)]
#[darling(attributes(extract), supports(struct_named))]
struct ExtractDeriveInput {
    ident: syn::Ident,

    data: darling::ast::Data<(), ExtractFieldReceiver>,

    pattern: String,

    #[darling(default)]
    group_name: Option<String>,
}

#[derive(FromField)]
#[darling(attributes(extract))]
struct ExtractFieldReceiver {
    /// Get the ident of the field.
    /// For fields in tuple or newtype structs or enum bodies, this can be `None`.
    ident: Option<syn::Ident>,

    /// This magic field name pulls the type from the input.
    ty: syn::Type,

    #[darling(default)]
    domain: bool,
}

fn impl_extract_trait(item: TokenStream) -> darling::Result<TokenStream> {
    // parse & extract attributes
    let ast: DeriveInput = syn::parse(item).unwrap();
    let ExtractDeriveInput {
        ident,
        data,
        pattern,
        group_name,
    } = ExtractDeriveInput::from_derive_input(&ast)?;
    let group_name = group_name.unwrap_or_else(|| "subdomain".to_owned());

    // extract fields
    let ExtractFieldReceiver {
        ident: domain_ident,
        ty: domain_ty,
        ..
    } = data
        .take_struct()
        .expect("should only be named structs")
        .into_iter()
        .find(|f| f.domain)
        .ok_or_else(|| darling::Error::custom("no fields marked as domain"))?;

    if !is_string_type(&domain_ty) {
        return Err(darling::Error::unexpected_type(
            "domain field must be a String",
        ));
    }

    // define impl variables
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    // generate impl
    Ok(quote! {
        static __RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

        impl #impl_generics Extract for #ident #type_generics #where_clause {
            fn extract(&mut self, input: &str) -> std::collections::HashSet<std::string::String> {
                let re = __RE.get_or_init(|| {
                    let domain = self.#domain_ident.replace(".", r"\.");
                    let pat = format!(#pattern);
                    regex::Regex::new(&pat).expect("failed to compile regex")
                });

                re.captures_iter(input)
                    .map(|c| c[#group_name].to_owned())
                    .collect()
            }
        }
    }
    .into())
}

fn is_string_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(syn::TypePath { path, .. }) => match path.segments.len() {
            1 => path.segments[0].ident == "String",
            3 => {
                path.segments[0].ident == "std"
                    && path.segments[1].ident == "string"
                    && path.segments[2].ident == "String"
            }
            _ => false,
        },
        _ => false,
    }
}
