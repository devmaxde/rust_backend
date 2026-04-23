use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    braced, parse::Parse, parse_macro_input, punctuated::Punctuated, Ident as SynIdent, LitInt,
    Path, Token,
};

struct Spec {
    name: SynIdent,
    root: Option<Path>,
    blocks: Vec<(u16, Vec<SynIdent>)>,
}

impl Parse for Spec {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: SynIdent = input.parse()?;
        input.parse::<Token![,]>()?;
        let lookahead = input.lookahead1();
        let mut root: Option<Path> = None;
        if lookahead.peek(syn::Ident) && input.peek(syn::Ident) {
            let maybe_root_kw: SynIdent = input.parse()?;
            if maybe_root_kw == "root" {
                input.parse::<Token![=]>()?;
                root = Some(input.parse::<Path>()?);
                input.parse::<Token![,]>()?;
            } else {
                return Err(syn::Error::new(
                    maybe_root_kw.span(),
                    "expected `root = <path>,` or a status code",
                ));
            }
        }
        let mut blocks = Vec::new();
        while !input.is_empty() {
            let code_lit: LitInt = input.parse()?;
            let code: u16 = code_lit.base10_parse()?;
            input.parse::<Token![=]>()?;
            let content;
            braced!(content in input);
            let variants: Punctuated<SynIdent, Token![,]> =
                content.parse_terminated(SynIdent::parse, Token![,])?;
            blocks.push((code, variants.into_iter().collect()));
            if input.peek(Token![;]) {
                let _ = input.parse::<Token![;]>();
            }
            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }
        }
        Ok(Spec { name, root, blocks })
    }
}

#[proc_macro]
pub fn build_errors(input: TokenStream) -> TokenStream {
    let Spec { name, root, blocks } = parse_macro_input!(input as Spec);
    let enum_name = Ident::new(&name.to_string(), Span::call_site());
    let docs_mod = Ident::new(
        &format!("{}_error_docs", name.to_string().to_snake_case()),
        Span::call_site(),
    );
    let ctors_mod = Ident::new(
        &format!("{}_ctors", name.to_string().to_snake_case()),
        Span::call_site(),
    );

    let builder_ty: TokenStream2 = match root.as_ref() {
        Some(p) => quote! { #p::ErrorBuilder },
        None => quote! { ErrorBuilder },
    };
    let api_error_ty: TokenStream2 = match root.as_ref() {
        Some(p) => quote! { #p::ApiError },
        None => quote! { ApiError },
    };
    let gen_docs_ty: TokenStream2 = match root.as_ref() {
        Some(p) => quote! { #p::GeneratedDocs },
        None => quote! { GeneratedDocs },
    };

    let mut all_variants: Vec<Ident> = Vec::new();
    for (_, vs) in &blocks {
        for v in vs {
            all_variants.push(Ident::new(&v.to_string(), Span::call_site()));
        }
    }

    let status_match_arms: Vec<TokenStream2> = blocks
        .iter()
        .flat_map(|(code, vs)| {
            vs.iter().map(move |v| {
                let v = Ident::new(&v.to_string(), Span::call_site());
                let code_u16 = *code;
                quote! { Self::#v => axum::http::StatusCode::from_u16(#code_u16).unwrap() }
            })
        })
        .collect();

    let msg_match_arms: Vec<TokenStream2> = all_variants
        .iter()
        .map(|v| {
            let s = v.to_string();
            quote! { Self::#v => #s }
        })
        .collect();

    let mut leaf_defs: Vec<TokenStream2> = Vec::new();
    for (_, vs) in &blocks {
        for v in vs {
            let leaf = format_ident!("{}Error", v);
            leaf_defs.push(quote! {
              #[derive(serde::Serialize, schemars::JsonSchema, Clone)]
              pub struct #leaf {
                pub error: String,
                pub error_id: uuid::Uuid,
                #[serde(skip_serializing_if = "Option::is_none")]
                pub error_details: Option<serde_json::Value>,
              }
            });
        }
    }

    let mut code_to_variants: std::collections::BTreeMap<u16, Vec<Ident>> =
        std::collections::BTreeMap::new();
    for (code, vs) in &blocks {
        let e = code_to_variants.entry(*code).or_default();
        for v in vs {
            e.push(Ident::new(&v.to_string(), Span::call_site()));
        }
    }

    let mut grouped_enums: Vec<TokenStream2> = Vec::new();
    for (code, vs) in &code_to_variants {
        let code_ident = format_ident!("{}{}", name, code);
        let variants_wrapped: Vec<TokenStream2> = vs
            .iter()
            .map(|v| {
                let leaf = format_ident!("{}Error", v);
                quote! { #v(#leaf) }
            })
            .collect();
        grouped_enums.push(quote! {
          #[derive(serde::Serialize, schemars::JsonSchema, Clone)]
          #[serde(tag = "")]
          pub enum #code_ident { #( #variants_wrapped ),* }
        });
    }

    let mut add_code_fns: Vec<TokenStream2> = Vec::new();
    for code in code_to_variants.keys() {
        let code_u16 = *code;
        let code_ident = format_ident!("{}{}", name, code);
        let fn_name = format_ident!("add_{}", code_u16);
        add_code_fns.push(quote! {
      pub fn #fn_name(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
        op.response::<#code_u16, axum::Json<#code_ident>>()
      }
    });
    }

    let add_all_fn: TokenStream2 = {
        let calls: Vec<TokenStream2> = code_to_variants
            .keys()
            .map(|code| {
                let code_ident = format_ident!("{}{}", name, code);
                let code_u16 = *code;
                quote! { op = op.response::<#code_u16, axum::Json<#code_ident>>(); }
            })
            .collect();
        quote! {
          pub fn add_all(mut op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
            #( #calls )*
            op
          }
        }
    };

    let snake_fns: Vec<TokenStream2> = all_variants
        .iter()
        .map(|v| {
            let fn_name = format_ident!("{}", v.to_string().to_snake_case());
            quote! {
              pub fn #fn_name() -> #builder_ty<Self> {
                #builder_ty::new(Self::#v)
              }
            }
        })
        .collect();

    let ctor_structs: Vec<TokenStream2> = all_variants
        .iter()
        .map(|v| {
            quote! {
              pub struct #v;
              impl #v {
                pub fn new() -> #builder_ty<super::#enum_name> {
                  #builder_ty::new(super::#enum_name::#v)
                }
              }
              impl From<#v> for super::#enum_name {
                fn from(_: #v) -> Self { super::#enum_name::#v }
              }
            }
        })
        .collect();

    let expanded = quote! {
      #[derive(Debug, Clone, Copy, serde::Serialize, schemars::JsonSchema)]
      pub enum #enum_name { #( #all_variants ),* }

      impl #api_error_ty for #enum_name {
        fn status_code(&self) -> axum::http::StatusCode { match self { #( #status_match_arms ),* } }
        fn message(&self) -> &'static str { match self { #( #msg_match_arms ),* } }
      }

      pub mod #docs_mod {
        #( #leaf_defs )*
        #( #grouped_enums )*
        #( #add_code_fns )*
        #add_all_fn
      }

      pub mod #ctors_mod {
        #[allow(unused_imports)]
        use super::{#enum_name, ErrorBuilder};
        #( #ctor_structs )*
      }

      impl #enum_name { #( #snake_fns )* }

      impl #gen_docs_ty for #enum_name {
        fn generated_error_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
          #docs_mod::add_all(op)
        }
      }
    };
    TokenStream::from(expanded)
}
