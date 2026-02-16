// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to generate Rust's `fn`'s from TL definitions.

use crate::metadata::Metadata;
use crate::rustifier;
use crate::GeneratorConfig;
use std::io::{self, Write};
use tdlib_rs_parser::tl::{Category, Definition};

/// Defines the `function` corresponding to the definition:
///
/// ```ignore
/// pub async fn name(client_id: i32, field: Type) -> Result {
///
/// }
/// ```
fn write_function<W: Write>(
    file: &mut W,
    def: &Definition,
    _metadata: &Metadata,
    config: &GeneratorConfig,
) -> io::Result<()> {
    if rustifier::definitions::is_for_bots_only(def) && !config.gen_bots_only_api {
        return Ok(());
    }

    // Documentation
    writeln!(file, "{}", rustifier::definitions::description(def, "    "))?;
    writeln!(file, "    /// # Arguments")?;
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !config.gen_bots_only_api {
            continue;
        }

        writeln!(
            file,
            "    /// * `{}` - {}",
            rustifier::parameters::attr_name(param),
            param.description.replace('\n', "\n    /// ")
        )?;
    }
    writeln!(
        file,
        "    /// * `client_id` - The client id to send the request to"
    )?;

    // Function
    writeln!(file, "    #[allow(clippy::too_many_arguments)]")?;
    write!(
        file,
        "    pub async fn {}(",
        rustifier::definitions::function_name(def)
    )?;
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !config.gen_bots_only_api {
            continue;
        }

        write!(file, "{}: ", rustifier::parameters::attr_name(param))?;

        let is_optional = rustifier::parameters::is_optional(param);
        if is_optional {
            write!(file, "Option<")?;
        }
        write!(
            file,
            "{}",
            rustifier::parameters::qual_name(param, config.use_shared_string)
        )?;
        if is_optional {
            write!(file, ">")?;
        }

        write!(file, ", ")?;
    }

    writeln!(
        file,
        "client_id: i32) -> Result<{}, crate::TdError> {{",
        rustifier::types::qual_name(&def.ty, false, config.use_shared_string)
    )?;

    // Compose request
    writeln!(file, "        let request = json!({{")?;
    writeln!(file, "            \"@type\": \"{}\",", def.name)?;
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !config.gen_bots_only_api {
            continue;
        }

        writeln!(
            file,
            "            \"{0}\": {1},",
            param.name,
            rustifier::parameters::attr_name(param),
        )?;
    }
    writeln!(file, "        }});")?;

    // Send request and deserialize response
    writeln!(
        file,
        "        let response = send_request(client_id, request).await;"
    )?;

    let return_type_name = rustifier::definitions::type_name(def);
    if rustifier::types::is_ok(&def.ty) {
        // For () return types, only check for API errors
        writeln!(file, "        if let Ok(api_error) = serde_json::from_str::<crate::types::Error>(&response) {{")?;
        writeln!(file, "            return Err(crate::TdError::Api(api_error));")?;
        writeln!(file, "        }}")?;
        writeln!(file, "        Ok(())")?;
    } else {
        // Try to deserialize as the target type; on failure check for API error
        writeln!(file, "        match serde_json::from_str(&response) {{")?;
        writeln!(file, "            Ok(result) => Ok(result),")?;
        writeln!(file, "            Err(e) => {{")?;
        writeln!(file, "                if let Ok(api_error) = serde_json::from_str::<crate::types::Error>(&response) {{")?;
        writeln!(file, "                    Err(crate::TdError::Api(api_error))")?;
        writeln!(file, "                }} else {{")?;
        writeln!(file, "                    Err(crate::TdError::Deserialization {{ expected_type: \"{return_type_name}\", payload: response, error: e }})")?;
        writeln!(file, "                }}")?;
        writeln!(file, "            }}")?;
        writeln!(file, "        }}")?;
    }

    writeln!(file, "    }}")?;
    Ok(())
}

/// Writes an entire definition as Rust code (`fn`).
fn write_definition<W: Write>(
    file: &mut W,
    def: &Definition,
    metadata: &Metadata,
    config: &GeneratorConfig,
) -> io::Result<()> {
    write_function(file, def, metadata, config)?;
    Ok(())
}

/// Write the entire module dedicated to functions.
pub(crate) fn write_functions_mod<W: Write>(
    mut file: &mut W,
    definitions: &[Definition],
    metadata: &Metadata,
    config: &GeneratorConfig,
) -> io::Result<()> {
    // Begin outermost mod
    writeln!(file, "#[allow(clippy::all)]")?;
    writeln!(file, "pub mod functions {{")?;
    writeln!(file, "    use serde_json::json;")?;
    writeln!(file, "    use crate::send_request;")?;
    if config.use_shared_string {
        writeln!(file, "    use crate::TdString;")?;
    }

    let functions = definitions
        .iter()
        .filter(|d| d.category == Category::Functions);

    for definition in functions {
        write_definition(&mut file, definition, metadata, config)?;
    }

    // End outermost mod
    writeln!(file, "}}")
}
