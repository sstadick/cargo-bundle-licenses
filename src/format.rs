//! The allowed serialization / deserialization formats.
use crate::bundle::Bundle;
use std::io::{self, Read, Write};
use strum::{EnumString, VariantNames};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(EnumString, VariantNames, Debug, Copy, Clone)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[strum(serialize_all = "kebab-case")]
pub enum Format {
    #[strum(serialize = "json")]
    Json,
    #[strum(serialize = "toml", serialize = "tml")]
    Toml,
    #[strum(serialize = "yaml", serialize = "yml")]
    Yaml,
}

impl Format {
    pub fn serialize_to_writer<W: Write>(
        self,
        mut writer: W,
        bundle: &Bundle,
    ) -> Result<(), FormatError> {
        match self {
            Format::Json => {
                writer.write_all(serde_json::to_string_pretty(&bundle)?.as_bytes())?;
            }
            Format::Toml => {
                writer.write_all(toml::to_string(&bundle)?.as_bytes())?;
            }
            Format::Yaml => {
                writer.write_all(serde_yaml::to_string(&bundle)?.as_bytes())?;
            }
        }
        Ok(())
    }

    pub fn deserialize_from_reader<R: Read>(self, mut reader: R) -> Result<Bundle, FormatError> {
        let bundle: Bundle = match self {
            Format::Json => serde_json::from_reader(reader)?,
            Format::Toml => {
                let mut buffer = String::new();
                reader.read_to_string(&mut buffer)?;
                toml::from_str(&buffer)?
            }
            Format::Yaml => serde_yaml::from_reader(reader)?,
        };
        Ok(bundle)
    }
}
