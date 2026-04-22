// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Several functions to "rustify" names.
//!
//! Each parsed type can have a corresponding "rusty" name, and
//! the method for it can be found in the corresponding submodule:
//!
//! * `type_name` for use after a type definition (`type FooBar`, `enum FooBar`).
//! * `qual_name` for the qualified type name (`crate::foo::BarBaz`).
//! * `variant_name` for use inside `enum` variants (`Foo`).
//! * `item_path` for use as a qualified item path (`Vec::<u8>`).
//! * `attr_name` for use as an attribute name (`foo_bar: ()`).

use tdlib_rs_parser::tl::{Definition, Parameter, Type};

/// Get the rusty type name for a certain definition, excluding namespace.
///
/// For example, transforms `"ns.some_OK_name"` into `"SomeOkName"`.
fn rusty_type_name(name: &str) -> String {
    enum Casing {
        Upper,
        Lower,
        Preserve,
    }

    let name = if let Some(pos) = name.rfind('.') {
        &name[pos + 1..]
    } else {
        name
    };

    let mut result = String::with_capacity(name.len());

    name.chars().fold(Casing::Upper, |casing, c| {
        if c == '_' {
            return Casing::Upper;
        }

        match casing {
            Casing::Upper => {
                result.push(c.to_ascii_uppercase());
                Casing::Lower
            }
            Casing::Lower => {
                result.push(c.to_ascii_lowercase());
                if c.is_ascii_uppercase() {
                    Casing::Lower
                } else {
                    Casing::Preserve
                }
            }
            Casing::Preserve => {
                result.push(c);
                if c.is_ascii_uppercase() {
                    Casing::Lower
                } else {
                    Casing::Preserve
                }
            }
        }
    });

    result
}

/// Get the rusty documentation from a string.
fn rusty_doc(indent: &str, doc: &str) -> String {
    format!(
        "{}/// {}",
        indent,
        doc.replace('\n', &format!("\n{indent}/// "))
    )
}

pub mod definitions {
    use super::*;

    pub fn type_name(def: &Definition) -> String {
        rusty_type_name(&def.name)
    }

    pub fn function_name(def: &Definition) -> String {
        let mut result = String::with_capacity(def.name.len());

        def.name.chars().for_each(|c| {
            if c.is_ascii_uppercase() {
                result.push('_');
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        });

        result
    }

    pub fn qual_name(def: &Definition) -> String {
        let mut result = String::new();
        result.push_str("crate::types::");
        result.push_str(&type_name(def));
        result
    }

    pub fn variant_name(def: &Definition) -> String {
        let name = type_name(def);
        let ty_name = types::type_name(&def.ty);

        let variant = if name.starts_with(&ty_name) && name.len() > ty_name.len() {
            let variant_name = &name[ty_name.len()..];
            if variant_name.chars().next().unwrap().is_ascii_lowercase() {
                &name
            } else {
                variant_name
            }
        } else {
            &name
        };

        match variant {
            "" => {
                // Use the name from the last uppercase letter
                &name[name
                    .as_bytes()
                    .iter()
                    .rposition(|c| c.is_ascii_uppercase())
                    .unwrap_or(0)..]
            }
            "Self" => {
                // Use the name from the second-to-last uppercase letter
                &name[name
                    .as_bytes()
                    .iter()
                    .take(name.len() - variant.len())
                    .rposition(|c| c.is_ascii_uppercase())
                    .unwrap_or(0)..]
            }
            _ => variant,
        }
        .to_string()
    }

    pub fn description(def: &Definition, indent: &str) -> String {
        rusty_doc(indent, &def.description)
    }

    pub fn is_for_bots_only(def: &Definition) -> bool {
        def.description.contains("; for bots only")
    }
}

pub mod types {
    use super::*;

    /// Returns the builtin Rust type for a TL type, if applicable.
    /// When `use_shared_string` is true, string types map to `TdString` (a type alias
    /// that resolves to either `String` or `gpui::SharedString` based on feature flags).
    pub(super) fn builtin_type(ty: &Type, use_shared_string: bool) -> Option<&'static str> {
        Some(match ty.name.as_ref() {
            "Bool" => "bool",
            "bytes" => {
                if use_shared_string {
                    "TdString"
                } else {
                    "String"
                }
            }
            "double" => "f64",
            "int32" => "i32",
            "int53" => "i64",
            "int64" => "i64",
            "string" => {
                if use_shared_string {
                    "TdString"
                } else {
                    "String"
                }
            }
            "vector" => "Vec",
            "Ok" => "()",
            _ => return None,
        })
    }

    fn get_base_path(ty: &Type, use_shared_string: bool) -> String {
        if let Some(name) = builtin_type(ty, use_shared_string) {
            name.to_string()
        } else {
            let mut result = String::new();
            if ty.bare {
                result.push_str("crate::types::");
            } else {
                result.push_str("crate::enums::");
            }
            result.push_str(&type_name(ty));
            result
        }
    }

    fn get_path(ty: &Type, optional_generic_arg: bool, use_shared_string: bool) -> String {
        let mut result = get_base_path(ty, use_shared_string);

        if let Some(generic_ty) = &ty.generic_arg {
            result.push('<');
            if optional_generic_arg {
                result.push_str("Option<");
            }

            result.push_str(&qual_name(generic_ty, false, use_shared_string));

            if optional_generic_arg {
                result.push('>');
            }
            result.push('>');
        }

        result
    }

    pub fn type_name(ty: &Type) -> String {
        rusty_type_name(&ty.name)
    }

    pub fn qual_name(ty: &Type, optional_generic_arg: bool, use_shared_string: bool) -> String {
        get_path(ty, optional_generic_arg, use_shared_string)
    }

    pub fn is_ok(ty: &Type) -> bool {
        ty.name == "Ok"
    }

    pub(super) fn serde_as(ty: &Type, use_shared_string: bool) -> Option<String> {
        if ty.name == "int64" {
            return Some("DisplayFromStr".into());
        }

        if let Some(generic_arg) = &ty.generic_arg {
            if let Some(serde_as) = serde_as(generic_arg, use_shared_string) {
                let mut result = get_base_path(ty, use_shared_string);

                result.push('<');
                result.push_str(&serde_as);
                result.push('>');

                return Some(result);
            }
        }

        None
    }
}

pub mod parameters {
    use super::*;

    // -------------------------------------------------------------------------
    // Curated newtype mapping for identifier fields.
    //
    // Each rule is (field-name-suffix-match, TL type gate, newtype path).
    // A rule fires when the field name equals or ends with the suffix AND the
    // field's TL type matches the gate exactly. The TL type gate keeps us from
    // collapsing distinct number spaces (e.g. `secret_chat_id:int32` stays
    // separate from `*_chat_id:int53` which maps to `ChatId`).
    //
    // `STRUCT_OWNED_IDS` covers the `id` fields on top-level entity structs
    // where `id` alone isn't a suffix we can safely pattern-match on.
    // -------------------------------------------------------------------------

    /// `(field_name_suffix, tl_type, newtype_path)` — suffix rules applied to
    /// any struct-field or function-argument whose name ends with the suffix
    /// and whose TL type equals the gate.
    const ID_SUFFIX_RULES: &[(&str, &str, &str)] = &[
        ("chat_id", "int53", "crate::newtypes::ChatId"),
        ("user_id", "int53", "crate::newtypes::UserId"),
        ("message_id", "int53", "crate::newtypes::MessageId"),
        ("topic_id", "int53", "crate::newtypes::TopicId"),
        ("file_id", "int32", "crate::newtypes::FileId"),
        ("secret_chat_id", "int32", "crate::newtypes::SecretChatId"),
    ];

    /// `(TL struct name, field name, TL type gate, newtype_path)` — explicit
    /// overrides for the `id` field on top-level entity structs. The TL
    /// struct name matches the raw lowercase name (e.g. `"chat"`, not
    /// `"Chat"`). The TL type gate is per-entry because `SecretChat.id` is
    /// `int32` while the other `id` fields are `int53`.
    const STRUCT_OWNED_IDS: &[(&str, &str, &str, &str)] = &[
        ("chat", "id", "int53", "crate::newtypes::ChatId"),
        ("user", "id", "int53", "crate::newtypes::UserId"),
        ("message", "id", "int53", "crate::newtypes::MessageId"),
        ("secretChat", "id", "int32", "crate::newtypes::SecretChatId"),
    ];

    /// If the parameter matches a curated ID rule, return the newtype path.
    /// Only applies when the TL type is a plain int (no generic arg, no
    /// optional/vector wrapping) — wrapping an `Option<ChatId>` around a
    /// nullable field happens in the caller, not here.
    fn curated_newtype(param: &Parameter, owning_def_name: Option<&str>) -> Option<&'static str> {
        let tl_ty: &str = &param.ty.name;

        // Explicit struct-owned `id` fields take precedence.
        if let Some(def_name) = owning_def_name {
            for (struct_name, field, expected_ty, newtype) in STRUCT_OWNED_IDS {
                if def_name == *struct_name && param.name == *field && tl_ty == *expected_ty {
                    return Some(newtype);
                }
            }
        }

        // Suffix rules: field name equals or ends with `_<suffix>`, and TL
        // type matches the gate.
        for (suffix, expected_ty, newtype) in ID_SUFFIX_RULES {
            if tl_ty != *expected_ty {
                continue;
            }
            if param.name == *suffix || param.name.ends_with(&format!("_{suffix}")) {
                return Some(newtype);
            }
        }

        None
    }

    /// Type path for a parameter. Resolves curated ID newtypes first (keyed
    /// on the owning definition's TL name for the struct-owned `id` override),
    /// then falls back to the default TL → Rust mapping.
    pub fn qual_name(
        param: &Parameter,
        use_shared_string: bool,
        owning_def_name: Option<&str>,
    ) -> String {
        if let Some(newtype) = curated_newtype(param, owning_def_name) {
            return newtype.to_string();
        }
        // HACK: We're just matching against specific cases because there's not a
        // documented way for knowing optional generic arguments in the tl scheme
        let optional_generic_arg = param.description.contains("; messages may be null");
        types::qual_name(&param.ty, optional_generic_arg, use_shared_string)
    }

    pub fn attr_name(param: &Parameter) -> String {
        match &param.name[..] {
            "final" => "r#final".into(),
            "loop" => "r#loop".into(),
            "self" => "is_self".into(),
            "static" => "r#static".into(),
            "type" => "r#type".into(),
            _ => {
                let mut result = param.name.clone();
                result[..].make_ascii_lowercase();
                result
            }
        }
    }

    pub fn is_builtin_type(param: &Parameter, use_shared_string: bool) -> bool {
        types::builtin_type(&param.ty, use_shared_string).is_some() || is_optional(param)
    }

    pub fn is_optional(param: &Parameter) -> bool {
        param.description.contains("; may be null") || param.description.contains("; pass null")
    }

    pub fn is_for_bots_only(param: &Parameter) -> bool {
        param.description.contains("; for bots only")
    }

    pub fn description(param: &Parameter, indent: &str) -> String {
        rusty_doc(indent, &param.description)
    }

    pub fn serde_as(param: &Parameter, use_shared_string: bool) -> Option<String> {
        types::serde_as(&param.ty, use_shared_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Core methods

    #[test]
    fn check_rusty_type_name() {
        assert_eq!(rusty_type_name("ns.some_OK_name"), "SomeOkName");
    }

    // Definition methods

    #[test]
    fn check_def_type_name() {
        let def = "userEmpty = User".parse().unwrap();
        let name = definitions::type_name(&def);
        assert_eq!(name, "UserEmpty");
    }

    #[test]
    fn check_def_qual_name() {
        let def = "userEmpty = User".parse().unwrap();
        let name = definitions::qual_name(&def);
        assert_eq!(name, "crate::types::UserEmpty");
    }

    #[test]
    fn check_def_variant_name() {
        let def = "new_session_created = NewSession".parse().unwrap();
        let name = definitions::variant_name(&def);
        assert_eq!(name, "Created");
    }

    #[test]
    fn check_def_empty_variant_name() {
        let def = "true = True".parse().unwrap();
        let name = definitions::variant_name(&def);
        assert_eq!(name, "True");
    }

    #[test]
    fn check_def_self_variant_name() {
        let def = "inputPeerSelf = InputPeer".parse().unwrap();
        let name = definitions::variant_name(&def);
        assert_eq!(name, "PeerSelf");
    }

    // Type methods

    #[test]
    fn check_type_type_name() {
        let ty = "storage.FileType".parse().unwrap();
        let name = types::type_name(&ty);
        assert_eq!(name, "FileType");
    }

    #[test]
    fn check_type_qual_name() {
        let ty = "InputPeer".parse().unwrap();
        let name = types::qual_name(&ty, false, false);
        assert_eq!(name, "crate::enums::InputPeer");
    }

    #[test]
    fn check_type_qual_bare_name() {
        let ty = "ipPort".parse().unwrap();
        let name = types::qual_name(&ty, false, false);
        assert_eq!(name, "crate::types::IpPort");
    }

    #[test]
    fn check_type_bytes_qual_name() {
        let ty = "bytes".parse().unwrap();
        let name = types::qual_name(&ty, false, false);
        assert_eq!(name, "String");
    }

    #[test]
    fn check_type_bytes_qual_name_shared_string() {
        let ty = "bytes".parse().unwrap();
        let name = types::qual_name(&ty, false, true);
        assert_eq!(name, "TdString");
    }

    #[test]
    fn check_type_large_int_qual_name() {
        let ty = "int256".parse().unwrap();
        let name = types::qual_name(&ty, false, false);
        assert_eq!(name, "crate::types::Int256");
    }

    #[test]
    fn check_type_raw_vec_qual_name() {
        let ty = "vector<long>".parse().unwrap();
        let name = types::qual_name(&ty, false, false);
        assert_eq!(name, "Vec<crate::types::Long>");
    }

    #[test]
    fn check_type_opt_raw_vec_qual_name() {
        let ty = "vector<long>".parse().unwrap();
        let name = types::qual_name(&ty, true, false);
        assert_eq!(name, "Vec<Option<crate::types::Long>>");
    }

    #[test]
    fn check_type_vec_qual_name() {
        let ty = "Vector<Bool>".parse().unwrap();
        let name = types::qual_name(&ty, false, false);
        assert_eq!(name, "crate::enums::Vector<bool>");
    }

    #[test]
    fn check_type_opt_vec_qual_name() {
        let ty = "Vector<Bool>".parse().unwrap();
        let name = types::qual_name(&ty, true, false);
        assert_eq!(name, "crate::enums::Vector<Option<bool>>");
    }

    // Parameter methods

    #[test]
    fn check_param_qual_name() {
        let param = "pts:int".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "crate::types::Int");
    }

    #[test]
    fn check_param_attr_name() {
        let param = "access_hash:long".parse().unwrap();
        let name = parameters::attr_name(&param);
        assert_eq!(name, "access_hash");
    }

    // Curated ID newtype tests

    #[test]
    fn curated_chat_id_by_suffix() {
        let param = "chat_id:int53".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "crate::newtypes::ChatId");
    }

    #[test]
    fn curated_suffixed_chat_id() {
        let param = "linked_chat_id:int53".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "crate::newtypes::ChatId");
    }

    #[test]
    fn curated_secret_chat_id_not_chat_id() {
        // int32-gated: secret_chat_id must map to SecretChatId, not ChatId.
        let param = "secret_chat_id:int32".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "crate::newtypes::SecretChatId");
    }

    #[test]
    fn curated_int53_message_id() {
        let param = "message_id:int53".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "crate::newtypes::MessageId");
    }

    #[test]
    fn curated_int32_message_id_falls_through() {
        // int32 message_id (secret-chat event) does NOT get MessageId — rule
        // is int53-gated, so this falls back to the built-in `i32`.
        let param = "message_id:int32".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "i32");
    }

    #[test]
    fn curated_struct_owned_id_on_chat() {
        // Inside `chat`, the plain `id` field maps to ChatId.
        let param = "id:int53".parse().unwrap();
        let name = parameters::qual_name(&param, false, Some("chat"));
        assert_eq!(name, "crate::newtypes::ChatId");
    }

    #[test]
    fn curated_struct_owned_id_outside_override() {
        // The same `id:int53` field, but on an unlisted struct, falls back.
        let param = "id:int53".parse().unwrap();
        let name = parameters::qual_name(&param, false, Some("basicGroup"));
        assert_eq!(name, "i64");
    }

    #[test]
    fn curated_file_id_int32() {
        let param = "file_id:int32".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "crate::newtypes::FileId");
    }

    #[test]
    fn curated_unrelated_int53_untouched() {
        let param = "date:int53".parse().unwrap();
        let name = parameters::qual_name(&param, false, None);
        assert_eq!(name, "i64");
    }
}
