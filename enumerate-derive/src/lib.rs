use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

use proc_macro2::Span;
use syn::{Ident, ItemEnum};

/// Derives the `Extract` trait for a struct.
///
/// This macro generates an implementation of the `Extract` trait that uses regex pattern matching
/// to extract values from an input string.
///
/// # Attributes
///
/// - `#[extract(pattern = "...")]` (required): Specifies the regex pattern to use for extraction.
/// - `#[extract(group_name = "...")]` (optional): Specifies the capture group name to extract (defaults to "subdomain").
/// - `#[extract(domain)]` (field attribute, required): Marks a field as the domain field. This field must be a `String`.
///
/// # Example
///
/// ```
/// use enumerate_derive::Extract;
///
/// #[derive(Extract)]
/// #[extract(pattern = r"(?P<subdomain>[a-zA-Z0-9-]+)\.{}")]
/// struct SubdomainExtractor {
///     #[extract(domain)]
///     domain: String,
/// }
/// ```
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

/// Generates a companion enum that derives `clap::ValueEnum`.
///
/// This attribute macro creates a new enum with the same variants as the original enum,
/// but with "Choice" appended to the name and the `clap::ValueEnum` trait derived.
///
/// # Example
///
/// ```ignore
/// use enumerate_derive::enum_choice;
///
/// #[enum_choice]
/// enum Engine {
///     Google,
///     Bing,
///     Yahoo,
/// }
/// ```
///
/// This generates:
///
/// ```ignore
/// #[derive(clap::ValueEnum, Clone, Debug)]
/// enum EngineChoice {
///     Google,
///     Bing,
///     Yahoo,
/// }
/// ```
#[proc_macro_attribute]
pub fn enum_choice(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemEnum);

    let ItemEnum {
        ident,
        variants,
        vis,
        ..
    } = &input;

    let new_ident = Ident::new(&format!("{}Choice", ident.to_string()), ident.span());
    let variant_idents = variants.iter().map(|v| &v.ident).collect::<Vec<_>>();

    quote! {
        #input

        #[derive(clap::ValueEnum, Clone, Debug)]
        #[clap(rename_all = "lower")]
        #vis enum #new_ident {
            #(#variant_idents),*
        }
    }
    .into()
}

/// Generates a method to create a vector containing all enum variants.
///
/// # Example
///
/// ```ignore
/// use enumerate_derive::enum_vec;
///
/// #[enum_vec]
/// enum Engine {
///     Google,
///     Bing,
///     Yahoo,
/// }
/// ```
///
/// This generates a method:
///
/// ```ignore
/// impl Engine {
///     pub fn enum_vec(domain: &str) -> Vec<Engine> {
///         vec![
///             Google::new(domain).into(),
///             Bing::new(domain).into(),
///             Yahoo::new(domain).into(),
///         ]
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn enum_vec(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemEnum);

    let ItemEnum {
        ident,
        variants,
        vis,
        generics,
        ..
    } = &input;

    let fn_name = Ident::new("enum_vec", Span::call_site());

    let variant_idents = variants.iter().map(|v| &v.ident).collect::<Vec<_>>();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #input

        impl #impl_generics #ident #ty_generics #where_clause {
            #vis fn #fn_name (domain: &str) -> std::vec::Vec<#ident #ty_generics> {
                vec![#(#variant_idents::new(domain).into()),*]
            }
        }
    }
    .into()
}
