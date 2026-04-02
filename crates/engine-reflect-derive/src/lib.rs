use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Expr, Field, Fields, Meta};

#[proc_macro_derive(RegisterReflect, attributes(reflect, engine_reflect))]
pub fn derive_register_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    let metadata_actions = match collect_metadata_actions(&input.data) {
        Ok(actions) => actions,
        Err(error) => return error.to_compile_error().into(),
    };

    quote! {
        impl ::engine_reflect::ReflectRegistration for #ident
        where
            Self: ::bevy_ecs::component::Component
                + ::bevy_reflect::Reflect
                + ::bevy_reflect::GetTypeRegistration
                + ::core::default::Default
                + 'static,
        {
            fn register_reflect(
                type_registry: &mut ::engine_reflect::ReflectTypeRegistry,
                component_registry: &mut ::engine_reflect::ComponentRegistry,
                metadata_registry: &mut ::engine_reflect::ReflectMetadataRegistry,
            ) {
                let _ = ::engine_reflect::register_component_type::<Self>(
                    type_registry,
                    component_registry,
                );
                #(#metadata_actions)*
            }
        }
    }
    .into()
}

fn collect_metadata_actions(data: &Data) -> syn::Result<Vec<TokenStream2>> {
    let mut actions = Vec::new();

    match data {
        Data::Struct(struct_data) => {
            collect_from_fields("", &struct_data.fields, &mut actions)?;
        }
        Data::Enum(enum_data) => {
            for variant in &enum_data.variants {
                collect_from_fields(&variant.ident.to_string(), &variant.fields, &mut actions)?;
            }
        }
        Data::Union(union_data) => {
            return Err(syn::Error::new(
                union_data.union_token.span(),
                "RegisterReflect does not support unions",
            ));
        }
    }

    Ok(actions)
}

fn collect_from_fields(
    variant_prefix: &str,
    fields: &Fields,
    actions: &mut Vec<TokenStream2>,
) -> syn::Result<()> {
    match fields {
        Fields::Named(named) => {
            for field in &named.named {
                let field_ident = field
                    .ident
                    .as_ref()
                    .expect("named fields always have an identifier")
                    .to_string();
                let field_path = if variant_prefix.is_empty() {
                    field_ident
                } else {
                    format!("{variant_prefix}.{field_ident}")
                };
                collect_field_actions(field, field_path, actions)?;
            }
        }
        Fields::Unnamed(unnamed) => {
            for (index, field) in unnamed.unnamed.iter().enumerate() {
                let index_path = if variant_prefix.is_empty() {
                    index.to_string()
                } else {
                    format!("{variant_prefix}.{index}")
                };
                collect_field_actions(field, index_path, actions)?;
            }
        }
        Fields::Unit => {}
    }

    Ok(())
}

fn collect_field_actions(
    field: &Field,
    field_path: String,
    actions: &mut Vec<TokenStream2>,
) -> syn::Result<()> {
    let mut hidden = false;
    let mut hint: Option<TokenStream2> = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("reflect") && !attr.path().is_ident("engine_reflect") {
            continue;
        }

        let meta = &attr.meta;
        if !matches!(meta, Meta::List(_)) {
            continue;
        }

        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("skip") || nested.path.is_ident("ignore") {
                hidden = true;
                return Ok(());
            }

            if nested.path.is_ident("range") {
                let mut min_expr: Option<Expr> = None;
                let mut max_expr: Option<Expr> = None;

                nested.parse_nested_meta(|range| {
                    if range.path.is_ident("min") {
                        min_expr = Some(range.value()?.parse()?);
                        return Ok(());
                    }

                    if range.path.is_ident("max") {
                        max_expr = Some(range.value()?.parse()?);
                        return Ok(());
                    }

                    Err(syn::Error::new(
                        range.path.span(),
                        "Unsupported range key. Use min/max",
                    ))
                })?;

                let min_expr = min_expr.ok_or_else(|| {
                    syn::Error::new(nested.path.span(), "range requires min value")
                })?;
                let max_expr = max_expr.ok_or_else(|| {
                    syn::Error::new(nested.path.span(), "range requires max value")
                })?;

                hint = Some(quote! {
                    ::engine_reflect::EditorHint::Range {
                        min: (#min_expr) as f64,
                        max: (#max_expr) as f64,
                    }
                });
                return Ok(());
            }

            if nested.path.is_ident("multiline") {
                hint = Some(quote! { ::engine_reflect::EditorHint::Multiline });
                return Ok(());
            }

            if nested.path.is_ident("color") {
                hint = Some(quote! { ::engine_reflect::EditorHint::Color });
                return Ok(());
            }

            if nested.path.is_ident("hide_in_inspector") {
                hint = Some(quote! { ::engine_reflect::EditorHint::HideInInspector });
                return Ok(());
            }

            if nested.path.is_ident("readonly") || nested.path.is_ident("read_only") {
                hint = Some(quote! { ::engine_reflect::EditorHint::ReadOnly });
                return Ok(());
            }

            if nested.path.is_ident("degrees") {
                hint = Some(quote! { ::engine_reflect::EditorHint::Degrees });
                return Ok(());
            }

            Ok(())
        })?;
    }

    if hidden {
        actions.push(quote! {
            metadata_registry.hide_field::<Self>(#field_path);
        });
    }

    if let Some(hint_tokens) = hint {
        actions.push(quote! {
            metadata_registry.set_hint::<Self>(#field_path, #hint_tokens);
        });
    }

    Ok(())
}
