//! A finalized license representation meant for serializing / deserialzing
use std::collections::HashMap;

use cargo_metadata::Package;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::license::License;

pub static LICENSE_NOT_FOUNT_TEXT: &str = "NOT FOUND";

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct LicenseAndText {
    /// The license itself in SPDX format
    pub license: String,
    /// The lines of the license text, or NOT FOUND
    pub text: String,
}

impl LicenseAndText {
    pub fn new(license: &License, text: String) -> Self {
        Self {
            license: license.to_string(),
            text,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FinalizedLicense {
    /// The name of the package this license is for.
    pub package_name: String,
    /// The version of the package this license is for.
    pub package_version: String,
    /// The url of the repository from the Cargo.toml.
    pub repository: String,
    /// The full license from the Cargo.toml.
    pub license: String,
    /// The licenses and their associated text.
    pub licenses: Vec<LicenseAndText>,
}

impl FinalizedLicense {
    pub fn new(package: &Package, license: License, licenses: Vec<LicenseAndText>) -> Self {
        Self {
            package_name: package.name.clone(),
            package_version: package.version.to_string(),
            repository: package.repository.to_owned().unwrap_or_default(),
            license: package
                .license
                .to_owned()
                .unwrap_or_else(|| license.to_string()),
            licenses,
        }
    }
}

impl PartialEq for FinalizedLicense {
    fn eq(&self, other: &Self) -> bool {
        if self.package_version != other.package_version || self.package_name != other.package_name
        {
            return false;
        }

        for (a, b) in self
            .licenses
            .iter()
            .sorted_by_key(|l| l.license.clone())
            .zip(other.licenses.iter().sorted_by_key(|l| l.license.clone()))
        {
            if a != b {
                return false;
            }
        }
        true
    }
}

/// Hashable struct to for a package for easy lookup
// TODO: could probably use package id? or str refs?
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct LicenseKey {
    package_name: String,
    package_version: String,
}

impl LicenseKey {
    pub fn new(package_name: String, package_version: String) -> Self {
        Self {
            package_name,
            package_version,
        }
    }
}

/// Get a lookup of package name + package version to another hashmpa of the licenses for that package.
pub fn finalized_licenses_lookup(
    licenses: &[FinalizedLicense],
) -> HashMap<LicenseKey, HashMap<String, LicenseAndText>> {
    let mut map = HashMap::new();
    for final_license in licenses {
        let mut inner_map = HashMap::new();
        for lic in &final_license.licenses {
            inner_map.insert(lic.license.to_string(), lic.clone());
        }
        let key = LicenseKey::new(
            final_license.package_name.clone(),
            final_license.package_version.clone(),
        );
        map.insert(key, inner_map);
    }
    map
}
