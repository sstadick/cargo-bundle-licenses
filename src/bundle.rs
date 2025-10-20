//! Find all LICENSE-like files in each packages source repo and match them with the
//! the licenses specified in the Cargo.toml file.

use std::str::FromStr as _;

use crate::{
    finalized_license::{
        finalized_licenses_lookup, FinalizedLicense, LicenseKey, LICENSE_NOT_FOUNT_TEXT,
    },
    found_license::{FoundLicense, FoundLicenseError},
    license::License,
    package_loader::PackageLoader,
};
use cargo_metadata::Package;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BundleError {
    #[error(transparent)]
    FoundLicenseError(#[from] crate::found_license::FoundLicenseError),
    #[error(transparent)]
    PackageLoaderError(#[from] crate::package_loader::PackageLoaderError),
}

#[derive(Clone, Debug, Default)]
pub struct BundleBuilder {
    previous: Option<Bundle>,
    features: Vec<String>,
    prefer: Vec<License>,
}

impl BundleBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn previous(mut self, previous: &Bundle) -> Self {
        self.previous = Some(previous.clone());
        self
    }

    pub fn features(mut self, features: &[String]) -> Self {
        self.features = features.to_vec();
        self
    }

    pub fn prefer(mut self, prefer: &[String]) -> Self {
        self.prefer = prefer
            .iter()
            .map(|p| License::from_str(p).unwrap())
            .collect();
        self
    }

    pub fn exec(&self) -> Result<Bundle, BundleError> {
        let loader = PackageLoader::new(&self.features)?;

        let roots = loader.get_package_roots()?;
        let packages = {
            let mut packages = loader
                .get_root_dependencies(&roots)?
                .into_iter()
                .filter(|&p| !roots.iter().any(|&r| r.name == p.name))
                .collect::<Vec<_>>();
            packages.sort_by_key(|p| (&p.name, &p.version));
            packages
        };

        // Find best possible license candidates
        // let found_licenses: Result<Vec<FoundLicense>, FoundLicenseError> =
        let found_licenses = packages
            .iter()
            .map(|&p| FoundLicense::new(p))
            .collect::<Result<Vec<FoundLicense>, FoundLicenseError>>()?;

        // Write out any errors / warnings associated with each found license
        // TODO: apply some level of warning level filters here?
        found_licenses.iter().for_each(FoundLicense::check);

        // Convert to serializable licence
        let mut finalized_licenses: Vec<FinalizedLicense> =
            found_licenses.iter().map(FoundLicense::finalize).collect();

        // For any Not Found check in previous to see if a license was manually added for that package-version-license combo and add it
        if let Some(previous) = &self.previous {
            let lookup = finalized_licenses_lookup(&previous.third_party_libraries);

            for lic in &mut finalized_licenses {
                if lic
                    .licenses
                    .iter()
                    .any(|l| l.text == LICENSE_NOT_FOUNT_TEXT)
                {
                    let key =
                        LicenseKey::new(lic.package_name.clone(), lic.package_version.clone());
                    if let Some(previous_licenses) = lookup.get(&key) {
                        for inner_license in &mut lic.licenses {
                            if let Some(previous_license) =
                                previous_licenses.get(inner_license.license.as_str())
                            {
                                if previous_license.text != LICENSE_NOT_FOUNT_TEXT {
                                    log::info!(
                                        "Using previous license text for {} license {}:{}",
                                        inner_license.license,
                                        lic.package_name,
                                        lic.package_version
                                    );
                                    inner_license.text = previous_license.text.clone();
                                }
                            }
                        }
                    }
                }
            }
        }

        // For packages with multiple licenses, retain only the preferred license
        for lic in &mut finalized_licenses {
            // TODO: handle AND in licenses
            if lic.license.contains("AND") {
                continue;
            }

            if let Some(preferred) = self.prefer.iter().find(|&preferred| {
                lic.licenses
                    .iter()
                    .any(|l| &License::from_str(&l.license).unwrap() == preferred)
            }) {
                lic.licenses
                    .retain(|l| &License::from_str(&l.license).unwrap() == preferred);
                lic.license = preferred.to_string();
            }
        }

        Ok(Bundle::new(&roots, finalized_licenses))
    }
}

/// A bundle of licenses
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Bundle {
    root_name: String,
    third_party_libraries: Vec<FinalizedLicense>,
}

impl Bundle {
    pub fn new(roots: &[&Package], third_party_libraries: Vec<FinalizedLicense>) -> Self {

        let roots = if let Some((first, rest)) = roots.split_first() {
            let mut roots = first.name.to_string();
            for root in rest {
                roots = format!("{roots}, {}", root.name.as_str())
            }
            roots
        } else {
            String::new()
        };

        Self {
            root_name: roots,
            third_party_libraries,
        }
    }

    /// Compare another [`Bundle`] against this [`Bundle`] requiring that "other" be a strict subset of self.
    pub fn check_subset(&self, other: &Self) -> bool {
        if self.root_name != other.root_name {
            log::error!(
                "Checked package root {} does not match existing package root {}",
                self.root_name,
                other.root_name
            );
            return false;
        }

        for lic in &other.third_party_libraries {
            if let Some(self_lic) = self.third_party_libraries.iter().find(|self_lic| {
                self_lic.package_name == lic.package_name
                    && self_lic.package_version == lic.package_version
            }) {
                if self_lic != lic {
                    log::error!(
                        "Previous {}:{} does not match new {}:{}",
                        self_lic.package_name,
                        self_lic.package_version,
                        lic.package_name,
                        lic.package_version
                    );
                    return false;
                }
            } else {
                log::error!(
                    "Could not find {}:{} in previous",
                    lic.package_name,
                    lic.package_version
                );
                return false;
            }
        }
        true
    }
}

impl PartialEq for Bundle {
    fn eq(&self, other: &Self) -> bool {
        if self.root_name != other.root_name {
            return false;
        }

        for (a, b) in self
            .third_party_libraries
            .iter()
            .zip(other.third_party_libraries.iter())
        {
            if a != b {
                return false;
            }
        }
        true
    }
}
