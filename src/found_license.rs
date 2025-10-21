//! A FoundLicense represents the raw form of the found parts of a license for a given package.

use cargo_metadata::{camino::Utf8PathBuf, Package};
use thiserror::Error;

use crate::{
    discovery::{find_package_license, Confidence, LicenseText},
    finalized_license::{FinalizedLicense, LicenseAndText, LICENSE_NOT_FOUNT_TEXT},
    license::License,
};

#[derive(Debug, Error)]
pub enum FoundLicenseError {
    #[error(transparent)]
    Discovery(#[from] crate::discovery::DiscoveryError),
}

enum BestChoice {
    Single(LicenseText),
    Multiple(Vec<LicenseText>),
    None,
}

struct FoundText {
    license: License,
    best_choice: BestChoice,
    confidence: Confidence,
}

impl FoundText {
    fn new(license: License, best_choice: BestChoice, confidence: Confidence) -> Self {
        Self {
            license,
            best_choice,
            confidence,
        }
    }
}

enum FoundTexts {
    Single(FoundText),
    Multiple(Vec<FoundText>),
}

/// An [`FoundLicense`] represents a found set license texts for a given package
/// along with the possible found texts.
pub struct FoundLicense {
    package: Package,
    license: License,
    texts: FoundTexts,
}

impl FoundLicense {
    /// Search a package for a possible license and identify the best candidates.
    pub fn new(package: &Package) -> Result<Self, FoundLicenseError> {
        let license = package.license();
        let texts = match &license {
            License::Unspecified => FoundTexts::Single(FoundText::new(
                license.clone(),
                BestChoice::None,
                Confidence::UnspecifiedLicenseInPackage,
            )),
            License::Multiple(licenses) => {
                let mut choices = vec![];
                for lic in licenses {
                    let texts = find_package_license(package, lic)?;
                    let (choice, conf) = choose(texts);
                    choices.push(FoundText::new(lic.clone(), choice, conf));
                }
                FoundTexts::Multiple(choices)
            }
            license => {
                let texts = find_package_license(package, license)?;
                let (choice, conf) = choose(texts);
                FoundTexts::Single(FoundText::new(license.clone(), choice, conf))
            }
        };

        Ok(Self {
            package: package.clone(),
            license,
            texts,
        })
    }

    /// Check for any errors / issues with found licenses
    pub fn check(&self) {
        // check if multiple possible licneses were found
        // check the confidense level of the best match

        fn check_text(text: &FoundText, package: &Package) {
            match &text.license {
                License::Unspecified => {
                    log::warn!(
                        "License is not specified for {}:{} in package: {}",
                        package.name,
                        package.version,
                        package.manifest_path
                    );
                }
                license => match text.best_choice {
                    BestChoice::Single(_) => match text.confidence {
                        Confidence::Confident => (),
                        Confidence::SemiConfident => log::warn!(
                            "Confidence level SEMI for {} license in {}:{} - {}",
                            license,
                            package.name,
                            package.version,
                            package.manifest_path
                        ),
                        Confidence::Unsure => log::warn!(
                            "Confidence level UNSURE for {} license in {}:{} - {}",
                            license,
                            package.name,
                            package.version,
                            package.manifest_path
                        ),
                        Confidence::NoTemplate => log::warn!(
                            "No template for {} license in {}:{} - {}",
                            license,
                            package.name,
                            package.version,
                            package.manifest_path
                        ),
                        _ => unimplemented!(),
                    },
                    BestChoice::Multiple(_) => {
                        log::warn!(
                            "Multiple possible licenses found for {} license in {}:{} - {}",
                            license,
                            package.name,
                            package.version,
                            package.manifest_path
                        );
                    }
                    BestChoice::None => {
                        log::warn!(
                            "No license found for {} license in {}:{} - {}",
                            license,
                            package.name,
                            package.version,
                            package.manifest_path
                        );
                    }
                },
            };
        }

        match &self.texts {
            FoundTexts::Single(text) => check_text(text, &self.package),
            FoundTexts::Multiple(texts) => {
                for text in texts {
                    check_text(text, &self.package);
                }
            }
        }
    }

    /// Finalize license choices by picking the top choice if multiple are present and filling in not-found licenses with
    /// a signifier value.
    pub fn finalize(&self) -> FinalizedLicense {
        let mut licenses = vec![];
        match &self.texts {
            FoundTexts::Single(text) => match &text.best_choice {
                BestChoice::Single(lic_text) => {
                    licenses.push(LicenseAndText::new(&text.license, lic_text.text.clone()))
                }
                BestChoice::Multiple(lic_texts) => licenses.push(LicenseAndText::new(
                    &text.license,
                    lic_texts[0].text.clone(),
                )),
                BestChoice::None => licenses.push(LicenseAndText::new(
                    &text.license,
                    String::from(LICENSE_NOT_FOUNT_TEXT),
                )),
            },
            FoundTexts::Multiple(texts) => {
                for text in texts {
                    match &text.best_choice {
                        BestChoice::Single(lic_text) => {
                            licenses.push(LicenseAndText::new(&text.license, lic_text.text.clone()))
                        }
                        BestChoice::Multiple(lic_texts) => licenses.push(LicenseAndText::new(
                            &text.license,
                            lic_texts[0].text.clone(),
                        )),
                        BestChoice::None => licenses.push(LicenseAndText::new(
                            &text.license,
                            String::from(LICENSE_NOT_FOUNT_TEXT),
                        )),
                    }
                }
            }
        };

        FinalizedLicense::new(&self.package, self.license.clone(), licenses)
    }
}

/// Choose the highest confidence license of all possible licenses.
fn choose(texts: Vec<LicenseText>) -> (BestChoice, Confidence) {
    // Partition licnese texts by confidense
    let (mut confident, texts): (Vec<LicenseText>, Vec<LicenseText>) = texts
        .into_iter()
        .partition(|text| text.confidence == Confidence::Confident);
    let (mut semi_confident, unconfident): (Vec<LicenseText>, Vec<LicenseText>) = texts
        .into_iter()
        .partition(|text| text.confidence == Confidence::SemiConfident);
    let (mut unsure, mut no_template): (Vec<LicenseText>, Vec<LicenseText>) = unconfident
        .into_iter()
        .partition(|text| text.confidence == Confidence::Unsure);

    if confident.len() == 1 {
        (
            BestChoice::Single(confident.swap_remove(0)),
            Confidence::Confident,
        )
    } else if confident.len() > 1 {
        (BestChoice::Multiple(confident), Confidence::Confident)
    } else if semi_confident.len() == 1 {
        (
            BestChoice::Single(semi_confident.swap_remove(0)),
            Confidence::SemiConfident,
        )
    } else if semi_confident.len() > 1 {
        (
            BestChoice::Multiple(semi_confident),
            Confidence::SemiConfident,
        )
    } else if unsure.len() == 1 {
        (
            BestChoice::Single(unsure.swap_remove(0)),
            Confidence::Unsure,
        )
    } else if unsure.len() > 1 {
        (BestChoice::Multiple(unsure), Confidence::Unsure)
    } else if no_template.len() == 1 {
        (
            BestChoice::Single(no_template.swap_remove(0)),
            Confidence::NoTemplate,
        )
    } else if no_template.len() > 1 {
        (BestChoice::Multiple(no_template), Confidence::NoTemplate)
    } else {
        (BestChoice::None, Confidence::MissingLicenseFile)
    }
}

/// Extension trait get a [`License`].
pub trait Licensed {
    fn license(&self) -> License;
}

impl Licensed for Package {
    /// Extension trait impl to convert the license in package toml to a [`License`].
    fn license(&self) -> License {
        self.license
            .as_ref()
            .and_then(|license| license.parse::<License>().ok())
            .or_else(|| {
                self.license_file().map(|license_file| {
                    // If the license file starts with the cargo_home path, strip it and replace with the ENV var $CARGO_HOME.
                    // This makes licenses comparable across machines.
                    let license_file = if let Some(stripped_license_file) = home::cargo_home()
                        .ok()
                        .and_then(|cargo_home| license_file.strip_prefix(cargo_home).ok())
                    {
                        Utf8PathBuf::from("$CARGO_HOME").join(stripped_license_file)
                    } else {
                        // license file is not under $CARGO_HOME keep it as is
                        license_file
                    };
                    License::File(license_file.into_std_path_buf())
                })
            })
            .unwrap_or_default()
    }
}
