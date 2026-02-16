// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to generate Rust's `struct`'s from TL definitions.

use crate::ignore_type;
use crate::metadata::Metadata;
use crate::rustifier;
use crate::GeneratorConfig;
use std::io::{self, Write};
use tdlib_rs_parser::tl::{Category, Definition};

/// Defines the `struct` corresponding to the definition:
///
/// ```ignore
/// pub struct Name {
///     pub field: Type,
/// }
/// ```
fn write_struct<W: Write>(
    file: &mut W,
    def: &Definition,
    metadata: &Metadata,
    config: &GeneratorConfig,
) -> io::Result<()> {
    if rustifier::definitions::is_for_bots_only(def) && !config.gen_bots_only_api {
        return Ok(());
    }

    writeln!(file, "{}", rustifier::definitions::description(def, "    "))?;

    let serde_as = def
        .params
        .iter()
        .any(|p| rustifier::parameters::serde_as(p, config.use_shared_string).is_some());

    if serde_as {
        writeln!(file, "    #[serde_as]",)?;
    }

    write!(file, "    #[derive(Clone, Debug, ",)?;
    if metadata.can_def_implement_default(def) {
        write!(file, "Default, ",)?;
    }
    writeln!(file, "PartialEq, Deserialize, Serialize)]",)?;

    writeln!(
        file,
        "    pub struct {} {{",
        rustifier::definitions::type_name(def),
    )?;

    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !config.gen_bots_only_api {
            continue;
        }

        writeln!(
            file,
            "{}",
            rustifier::parameters::description(param, "        ")
        )?;

        if let Some(serde_as) = rustifier::parameters::serde_as(param, config.use_shared_string) {
            writeln!(file, "        #[serde_as(as = \"{serde_as}\")]")?;
        }

        let is_optional = rustifier::parameters::is_optional(param);
        if is_optional {
            writeln!(file, "        #[serde(default)]")?;
        }
        write!(
            file,
            "        pub {}: ",
            rustifier::parameters::attr_name(param),
        )?;

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

        writeln!(file, ",")?;
    }

    writeln!(file, "    }}")?;
    Ok(())
}

/// Writes an entire definition as Rust code (`struct`).
fn write_definition<W: Write>(
    file: &mut W,
    def: &Definition,
    metadata: &Metadata,
    config: &GeneratorConfig,
) -> io::Result<()> {
    write_struct(file, def, metadata, config)?;
    Ok(())
}

/// Write the entire module dedicated to types.
pub(crate) fn write_types_mod<W: Write>(
    mut file: &mut W,
    definitions: &[Definition],
    metadata: &Metadata,
    config: &GeneratorConfig,
) -> io::Result<()> {
    // Begin outermost mod
    writeln!(file, "#[allow(clippy::all)]")?;
    writeln!(file, "pub mod types {{")?;
    writeln!(file, "    use serde::{{Deserialize, Serialize}};")?;
    writeln!(file, "    use serde_with::{{serde_as, DisplayFromStr}};")?;
    if config.use_shared_string {
        writeln!(file, "    use crate::TdString;")?;
    }

    let types = definitions
        .iter()
        .filter(|d| d.category == Category::Types && !ignore_type(&d.ty) && !d.params.is_empty());

    for definition in types {
        write_definition(&mut file, definition, metadata, config)?;
    }

    // End outermost mod
    writeln!(file, "}}")
}
