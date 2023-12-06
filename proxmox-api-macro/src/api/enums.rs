use std::convert::{TryFrom, TryInto};

use anyhow::Error;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote_spanned;

use super::Schema;
use crate::serde;
use crate::util::{self, FieldName, JSONObject, JSONValue, Maybe};

/// Enums, provided they're simple enums, simply get an enum string schema attached to them.
pub fn handle_enum(
    mut attribs: JSONObject,
    mut enum_ty: syn::ItemEnum,
) -> Result<TokenStream, Error> {
    if !attribs.contains_key("type") {
        attribs.insert(
            FieldName::new("type".to_string(), Span::call_site()),
            JSONValue::new_ident(Ident::new("String", enum_ty.enum_token.span)),
        );
    }

    if let Some(fmt) = attribs.remove("format") {
        error!(fmt.span(), "illegal key 'format', will be autogenerated");
    }

    let has_default_attrib = attribs.get("default").map(|def| def.span());

    let schema = {
        let mut schema: Schema = attribs.try_into()?;

        if schema.description.is_none() {
            let (comment, span) = util::get_doc_comments(&enum_ty.attrs)?;
            schema.description = Maybe::Derived(syn::LitStr::new(comment.trim(), span));
        }

        let mut ts = TokenStream::new();
        schema.to_typed_schema(&mut ts)?;
        ts
    };

    let container_attrs = serde::ContainerAttrib::try_from(&enum_ty.attrs[..])?;
    let derives_default = util::derives_trait(&enum_ty.attrs, "Default");
    let mut default_value = None;

    let mut variants = TokenStream::new();
    for variant in &mut enum_ty.variants {
        match &variant.fields {
            syn::Fields::Unit => (),
            _ => bail!(variant => "api macro does not support enums with fields"),
        }

        let (mut comment, _doc_span) = util::get_doc_comments(&variant.attrs)?;
        if comment.is_empty() {
            error!(&variant => "enum variant needs a description");
            comment = "<missing description>".to_string();
        }

        let attrs = serde::FieldAttrib::try_from(&variant.attrs[..])?;
        let variant_string = if let Some(renamed) = attrs.rename {
            renamed
        } else if let Some(rename_all) = container_attrs.rename_all {
            let name = rename_all.apply_to_variant(&variant.ident.to_string());
            syn::LitStr::new(&name, variant.ident.span())
        } else {
            let name = &variant.ident;
            syn::LitStr::new(&name.to_string(), name.span())
        };

        if derives_default {
            if let Some(attr) = variant.attrs.iter().find(|a| a.path().is_ident("default")) {
                if let Some(default_value) = &default_value {
                    error!(attr => "multiple default values defined");
                    error!(default_value => "default previously defined here");
                } else {
                    default_value = Some(variant_string.clone());
                    if let Some(span) = has_default_attrib {
                        error!(attr => "#[default] attribute in use with 'default' #[api] key");
                        error!(span, "'default' also defined here");
                    }
                }
            }
        }

        variants.extend(quote_spanned! { variant.ident.span() =>
            ::proxmox_schema::EnumEntry {
                value: #variant_string,
                description: #comment,
            },
        });
    }

    let name = &enum_ty.ident;

    let default_value = match default_value {
        Some(value) => quote_spanned!(value.span() => .default(#value)),
        None => TokenStream::new(),
    };

    Ok(quote_spanned! { name.span() =>
        #enum_ty

        impl ::proxmox_schema::ApiType for #name {
            const API_SCHEMA: ::proxmox_schema::Schema =
                #schema
                .format(&::proxmox_schema::ApiStringFormat::Enum(&[#variants]))
                #default_value
                .schema();
        }

        impl ::proxmox_schema::UpdaterType for #name {
            type Updater = Option<Self>;
        }
    })
}
