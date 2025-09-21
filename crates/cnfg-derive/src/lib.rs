use darling::{Error, FromField, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta, Type, parse_macro_input};

/// Parsed representation of a field with #[cnfg(...)] attributes.
#[derive(Debug, FromField)]
#[darling(attributes(cnfg))]
struct CnfgField {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    default: Option<syn::Lit>,

    #[darling(default)]
    env: Option<String>,

    /// CLI flag support (bare or explicit).
    #[darling(default)]
    cli: Option<CliAttr>,

    #[darling(default)]
    required: bool,

    #[darling(default)]
    nested: bool,

    #[darling(default, multiple, rename = "validate")]
    validators: Vec<ValidatorAttr>,
}

/// Represents `#[cnfg(cli)]` or `#[cnfg(cli = "--flag")]`.
#[derive(Debug, Clone)]
enum CliAttr {
    /// bare form: `#[cnfg(cli)]`
    Flag,
    /// explicit flag form: `#[cnfg(cli = "--custom")]`
    Custom(String),
}

impl FromMeta for CliAttr {
    fn from_meta(meta: &syn::Meta) -> Result<Self, Error> {
        match meta {
            syn::Meta::Path(_) => Ok(CliAttr::Flag),
            syn::Meta::NameValue(nv) => match &nv.value {
                syn::Expr::Lit(expr_lit) => parse_cli_lit(&expr_lit.lit),
                other => Err(Error::custom("expected a literal value").with_span(other)),
            },
            syn::Meta::List(list) => Err(Error::custom(
                "unsupported cli format; use #[cnfg(cli)] or #[cnfg(cli = \"--flag\")]",
            )
            .with_span(list)),
        }
    }
}

fn parse_cli_lit(lit: &Lit) -> Result<CliAttr, Error> {
    match lit {
        Lit::Str(s) => Ok(CliAttr::Custom(s.value())),
        Lit::Bool(b) => {
            if b.value() {
                Ok(CliAttr::Flag)
            } else {
                Err(Error::custom(
                    "use #[cnfg(cli)] to enable CLI parsing; remove the attribute to disable it",
                )
                .with_span(lit))
            }
        }
        _ => Err(Error::custom("expected a string flag or boolean true").with_span(lit)),
    }
}

/// Validator attributes: range, regex, url.
#[derive(Debug, FromMeta)]
#[darling(rename_all = "kebab-case")]
enum ValidatorAttr {
    Range(RangeArgs),
    Regex(String),
    Url,
}

#[derive(Debug, Default, FromMeta)]
struct RangeArgs {
    #[darling(default)]
    min: Option<f64>,
    #[darling(default)]
    max: Option<f64>,
}

#[proc_macro_derive(Cnfg, attributes(cnfg))]
pub fn derive_cnfg(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let struct_doc_tokens = doc_option_tokens(doc_from_attrs(&input.attrs));

    let fields = match &input.data {
        Data::Struct(ds) => match &ds.fields {
            Fields::Named(n) => &n.named,
            _ => panic!("Cnfg expects a struct with named fields"),
        },
        _ => panic!("Cnfg expects a struct"),
    };

    let mut defaults_kv = Vec::new();
    let mut field_spec_stmts = Vec::new();
    let mut cli_spec_stmts = Vec::new();
    let mut required_stmts = Vec::new();
    let mut validate_body = Vec::new();

    for f in fields {
        let cf = CnfgField::from_field(f).expect("parse #[cnfg] attributes");
        let ident = cf.ident.clone().expect("cnfg requires named fields");
        let fname = ident.to_string();
        let path_lit = syn::LitStr::new(&fname, Span::call_site());
        let field_name_lit = path_lit.clone();
        let required_flag = cf.required;
        let nested_flag = cf.nested;
        let field_doc_for_field = doc_option_tokens(doc_from_attrs(&f.attrs));
        let field_doc_for_cli = field_doc_for_field.clone();
        let env_tokens = option_str_tokens(cf.env.as_deref());
        let (is_option, inner_ty) = option_inner(&cf.ty);
        let nested_ty = if nested_flag && is_option {
            inner_ty
        } else {
            &cf.ty
        };

        let mut field_kind = kind_for_type(&cf.ty);
        if nested_flag {
            field_kind = quote! { cnfg::Kind::Object };
        }

        let default_literal = cf.default.as_ref().map(default_literal);
        let default_tokens_field = option_str_tokens(default_literal.as_deref());
        let default_tokens_cli = default_tokens_field.clone();

        if let Some(lit) = cf.default.clone() {
            defaults_kv.push(quote! {
                map.insert(#fname.to_string(), serde_json::json!(#lit));
            });
        } else if nested_flag {
            defaults_kv.push(quote! {
                map.insert(#fname.to_string(), <#nested_ty as cnfg::ConfigMeta>::defaults_json());
            });
        }

        field_spec_stmts.push(quote! {
            items.push(cnfg::FieldSpec {
                name: #field_name_lit,
                env: #env_tokens,
                path: #path_lit,
                doc: #field_doc_for_field,
                kind: #field_kind,
                default: #default_tokens_field,
                required: #required_flag,
            });
        });

        if required_flag {
            required_stmts.push(quote! {
                required.push(#path_lit);
            });
        }

        if let Some(cli_attr) = &cf.cli {
            let flag_raw = match cli_attr {
                CliAttr::Flag => fname.replace('_', "-"),
                CliAttr::Custom(explicit) => explicit.trim_start_matches("--").to_string(),
            };
            let flag_lit = syn::LitStr::new(&flag_raw, Span::call_site());
            let cli_kind = kind_for_type(&cf.ty);
            let takes_value_tokens = if is_bool(inner_ty) {
                quote! { false }
            } else {
                quote! { true }
            };
            cli_spec_stmts.push(quote! {
                items.push(cnfg::CliSpec {
                    flag: #flag_lit,
                    field: #field_name_lit,
                    kind: #cli_kind,
                    path: #path_lit,
                    doc: #field_doc_for_cli,
                    takes_value: #takes_value_tokens,
                    default: #default_tokens_cli,
                    required: #required_flag,
                });
            });
        }

        for v in cf.validators.iter() {
            match v {
                ValidatorAttr::Range(args) => {
                    let checks = range_checks(&ident, &cf.ty, args.min, args.max);
                    validate_body.push(checks);
                }
                ValidatorAttr::Regex(pattern) => {
                    if is_string_type(&cf.ty) {
                        if is_option_type(&cf.ty) {
                            validate_body.push(quote! {
                                if let Some(s) = &self.#ident {
                                    let re = regex::Regex::new(#pattern).expect("invalid regex");
                                    if !re.is_match(s) {
                                        errs.push(cnfg::error::Issue {
                                            field: #fname.to_string(),
                                            kind: cnfg::error::IssueKind::Regex,
                                            message: format!("regex not matched: {}", #pattern),
                                        });
                                    }
                                }
                            });
                        } else {
                            validate_body.push(quote! {
                                let re = regex::Regex::new(#pattern).expect("invalid regex");
                                if !re.is_match(&self.#ident) {
                                    errs.push(cnfg::error::Issue {
                                        field: #fname.to_string(),
                                        kind: cnfg::error::IssueKind::Regex,
                                        message: format!("regex not matched: {}", #pattern),
                                    });
                                }
                            });
                        }
                    }
                }
                ValidatorAttr::Url => {
                    if is_string_type(&cf.ty) {
                        if is_option_type(&cf.ty) {
                            validate_body.push(quote! {
                                if let Some(s) = &self.#ident {
                                    if url::Url::parse(s).is_err() {
                                        errs.push(cnfg::error::Issue {
                                            field: #fname.to_string(),
                                            kind: cnfg::error::IssueKind::Url,
                                            message: "invalid URL".to_string(),
                                        });
                                    }
                                }
                            });
                        } else {
                            validate_body.push(quote! {
                                if url::Url::parse(&self.#ident).is_err() {
                                    errs.push(cnfg::error::Issue {
                                        field: #fname.to_string(),
                                        kind: cnfg::error::IssueKind::Url,
                                        message: "invalid URL".to_string(),
                                    });
                                }
                            });
                        }
                    }
                }
            }
        }

        if nested_flag {
            let prefix = path_lit.clone();
            field_spec_stmts.push(quote! {
                for nested in <#nested_ty as cnfg::ConfigMeta>::field_specs() {
                    items.push(nested.with_prefix(#prefix));
                }
            });
            cli_spec_stmts.push(quote! {
                for nested in <#nested_ty as cnfg::ConfigMeta>::cli_specs() {
                    items.push(nested.with_prefix(#prefix));
                }
            });
            if !is_option {
                required_stmts.push(quote! {
                    for nested in <#nested_ty as cnfg::ConfigMeta>::required_fields() {
                        required.push(cnfg::util::leak_string(format!("{}.{nested}", #prefix)));
                    }
                });
            }
            if is_option {
                validate_body.push(quote! {
                    if let Some(value) = &self.#ident {
                        if let Err(nested_errs) = <#nested_ty as cnfg::Validate>::validate(value) {
                            errs.extend(nested_errs.with_prefix(#prefix));
                        }
                    }
                });
            } else {
                validate_body.push(quote! {
                    if let Err(nested_errs) = <#nested_ty as cnfg::Validate>::validate(&self.#ident) {
                        errs.extend(nested_errs.with_prefix(#prefix));
                    }
                });
            }
        }
    }

    let tokens = quote! {
        impl cnfg::ConfigMeta for #name {
            fn defaults_json() -> serde_json::Value {
                let mut map = serde_json::Map::new();
                #(#defaults_kv)*
                serde_json::Value::Object(map)
            }
            fn field_specs() -> &'static [cnfg::FieldSpec] {
                static FIELD_SPECS: std::sync::OnceLock<Vec<cnfg::FieldSpec>> = std::sync::OnceLock::new();
                FIELD_SPECS.get_or_init(|| {
                    let mut items = Vec::new();
                    #(#field_spec_stmts)*
                    items
                }).as_slice()
            }
            fn cli_specs() -> &'static [cnfg::CliSpec] {
                static CLI_SPECS: std::sync::OnceLock<Vec<cnfg::CliSpec>> = std::sync::OnceLock::new();
                CLI_SPECS.get_or_init(|| {
                    let mut items = Vec::new();
                    #(#cli_spec_stmts)*
                    items
                }).as_slice()
            }
            fn required_fields() -> &'static [&'static str] {
                static REQUIRED: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
                REQUIRED.get_or_init(|| {
                    let mut required = Vec::new();
                    #(#required_stmts)*
                    required
                }).as_slice()
            }
            fn doc() -> Option<&'static str> {
                #struct_doc_tokens
            }
        }

        impl cnfg::Validate for #name {
            fn validate(&self) -> Result<(), cnfg::ValidationErrors> {
                let mut errs = cnfg::ValidationErrors::new();
                #(#validate_body)*
                if errs.is_empty() { Ok(()) } else { Err(errs) }
            }
        }

        impl cnfg::LoaderExt for #name {
            fn validate(&self) -> Result<(), cnfg::ValidationErrors> {
                <Self as cnfg::Validate>::validate(self)
            }
        }

        impl #name {
            /// Load config using defaults, files, env, CLI, and validations.
            pub fn load() -> Result<Self, cnfg::CnfgError> {
                <Self as cnfg::LoaderExt>::load()
            }
        }
    };
    tokens.into()
}

// ---------- helpers ----------

fn kind_for_type(ty: &Type) -> proc_macro2::TokenStream {
    let (is_option, inner) = option_inner(ty);
    let t = if is_option { inner } else { ty };
    if is_bool(t) {
        quote! { cnfg::Kind::Bool }
    } else if is_int(t) {
        quote! { cnfg::Kind::Int }
    } else if is_float(t) {
        quote! { cnfg::Kind::Float }
    } else {
        quote! { cnfg::Kind::String }
    }
}

fn option_inner<'a>(ty: &'a Type) -> (bool, &'a Type) {
    if let Type::Path(tp) = ty {
        if tp.path.segments.len() == 1 && tp.path.segments[0].ident == "Option" {
            if let syn::PathArguments::AngleBracketed(ab) = &tp.path.segments[0].arguments {
                if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                    return (true, inner);
                }
            }
        }
    }
    (false, ty)
}

fn is_option_type(ty: &Type) -> bool {
    option_inner(ty).0
}

fn is_string_type(ty: &Type) -> bool {
    let (_, inner) = option_inner(ty);
    match inner {
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|s| s.ident == "String")
            .unwrap_or(false),
        _ => false,
    }
}

fn is_bool(ty: &Type) -> bool {
    is_ident(ty, &["bool"])
}

fn is_float(ty: &Type) -> bool {
    is_ident(ty, &["f32", "f64"])
}

fn is_int(ty: &Type) -> bool {
    is_ident(
        ty,
        &[
            "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128",
        ],
    )
}

fn is_ident(ty: &Type, names: &[&str]) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return names.iter().any(|n| seg.ident == *n);
        }
    }
    false
}

fn range_checks(
    ident: &syn::Ident,
    ty: &Type,
    min: Option<f64>,
    max: Option<f64>,
) -> proc_macro2::TokenStream {
    if !(is_int(ty)
        || is_float(ty)
        || (is_option_type(ty) && {
            let (_, inner) = option_inner(ty);
            is_int(inner) || is_float(inner)
        }))
    {
        return quote! {};
    }

    let fname = ident.to_string();

    if is_option_type(ty) {
        let min_clause = min
            .map(|m| {
                quote! {
                    if __f < #m as f64 {
                        errs.push(cnfg::error::Issue {
                            field: #fname.to_string(),
                            kind: cnfg::error::IssueKind::Range,
                            message: format!("must be >= {}", #m),
                        });
                    }
                }
            })
            .unwrap_or_else(|| quote! {});
        let max_clause = max
            .map(|m| {
                quote! {
                    if __f > #m as f64 {
                        errs.push(cnfg::error::Issue {
                            field: #fname.to_string(),
                            kind: cnfg::error::IssueKind::Range,
                            message: format!("must be <= {}", #m),
                        });
                    }
                }
            })
            .unwrap_or_else(|| quote! {});
        quote! {
            if let Some(__v) = &self.#ident {
                let __f: f64 = (*__v) as f64;
                #min_clause
                #max_clause
            }
        }
    } else {
        let min_clause = min
            .map(|m| {
                quote! {
                    if __f < #m as f64 {
                        errs.push(cnfg::error::Issue {
                            field: #fname.to_string(),
                            kind: cnfg::error::IssueKind::Range,
                            message: format!("must be >= {}", #m),
                        });
                    }
                }
            })
            .unwrap_or_else(|| quote! {});
        let max_clause = max
            .map(|m| {
                quote! {
                    if __f > #m as f64 {
                        errs.push(cnfg::error::Issue {
                            field: #fname.to_string(),
                            kind: cnfg::error::IssueKind::Range,
                            message: format!("must be <= {}", #m),
                        });
                    }
                }
            })
            .unwrap_or_else(|| quote! {});
        quote! {
            let __f: f64 = (self.#ident) as f64;
            #min_clause
            #max_clause
        }
    }
}

fn doc_from_attrs(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();
    for attr in attrs {
        if let Meta::NameValue(nv) = attr.meta.clone() {
            if nv.path.is_ident("doc") {
                if let Expr::Lit(expr_lit) = nv.value {
                    if let Lit::Str(lit_str) = expr_lit.lit {
                        let line = lit_str.value().trim().to_string();
                        if !line.is_empty() {
                            docs.push(line);
                        }
                    }
                }
            }
        }
    }
    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

fn doc_option_tokens(doc: Option<String>) -> proc_macro2::TokenStream {
    match doc {
        Some(text) => {
            let lit = syn::LitStr::new(&text, Span::call_site());
            quote! { Some(#lit) }
        }
        None => quote! { None },
    }
}

fn option_str_tokens(value: Option<&str>) -> proc_macro2::TokenStream {
    match value {
        Some(text) => {
            let lit = syn::LitStr::new(text, Span::call_site());
            quote! { Some(#lit) }
        }
        None => quote! { None },
    }
}

fn default_literal(lit: &Lit) -> String {
    match lit {
        Lit::Str(s) => s.value(),
        Lit::Bool(b) => b.value().to_string(),
        Lit::Int(i) => i.base10_digits().to_string(),
        Lit::Float(f) => f.base10_digits().to_string(),
        _ => lit.to_token_stream().to_string(),
    }
}
