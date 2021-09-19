use std::{collections::HashMap, fs, path::PathBuf};

use cargo_metadata::Package;
use regex::Regex;
use slug::slugify;
use thiserror::Error;

use crate::license::License;

const HIGH_CONFIDENCE_LIMIT: f32 = 0.10;
const LOW_CONFIDENCE_LIMIT: f32 = 0.15;

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Confidence {
    MultiplePossibleLicenseFiles,
    MissingLicenseFile,
    Confident,
    SemiConfident,
    Unsure,
    NoTemplate,
    UnspecifiedLicenseInPackage,
}

#[derive(Debug)]
pub struct LicenseText {
    pub path: PathBuf,
    pub text: String,
    pub confidence: Confidence,
}

fn add_frequencies(freq: &mut HashMap<String, u32>, text: &str) {
    for word in Regex::new(r"\w+").unwrap().find_iter(text) {
        *freq
            .entry(word.as_str().to_lowercase().clone())
            .or_insert(0) += 1;
    }
}

fn calculate_frequency(text: &str) -> HashMap<String, u32> {
    let mut freq = HashMap::new();
    add_frequencies(&mut freq, text);
    freq
}

fn compare(mut text_freq: HashMap<String, u32>, template_freq: &HashMap<String, u32>) -> u32 {
    let mut errors = 0;

    for (word, &count) in template_freq {
        let text_count = text_freq.remove(word).unwrap_or(0);
        let diff = ((text_count as i32) - (count as i32)).abs() as u32;
        errors += diff;
    }

    for (_, count) in text_freq {
        errors += count;
    }

    errors
}

fn check_against_template(text: &str, license: &License) -> Confidence {
    let text_freq = calculate_frequency(text);

    let template_freq = if let License::Multiple(ref licenses) = *license {
        let mut template_freq = HashMap::new();
        for license in licenses {
            if let Some(template) = license.template() {
                add_frequencies(&mut template_freq, template);
            } else {
                return Confidence::NoTemplate;
            }
        }
        template_freq
    } else if let Some(template) = license.template() {
        calculate_frequency(template)
    } else {
        return Confidence::NoTemplate;
    };

    let total: u32 = template_freq.values().sum();
    let errors = compare(text_freq, &template_freq);
    let score = (errors as f32) / (total as f32);

    if score < HIGH_CONFIDENCE_LIMIT {
        Confidence::Confident
    } else if score < LOW_CONFIDENCE_LIMIT {
        Confidence::SemiConfident
    } else {
        Confidence::Unsure
    }
}

pub fn find_package_license(
    package: &Package,
    license: &License,
) -> Result<Vec<LicenseText>, DiscoveryError> {
    /// Is this a generic license name
    fn generic_license_name(name: &str) -> bool {
        name.to_uppercase() == "LICENSE"
            || name.to_uppercase() == "LICENCE"
            || name.to_uppercase() == "LICENSE.MD"
            || name.to_uppercase() == "LICENSE.TXT"
            || name.to_uppercase() == "COPYING"
    }

    fn name_matches(name: &str, license: &License) -> bool {
        let name = slugify(name).to_lowercase();
        match *license {
            License::Custom(ref custom) => {
                let custom = slugify(custom).to_lowercase();
                name == custom
                    || name == format!("license-{}", custom)
                    || name == format!("license-{}-md", custom)
                    || name == format!("license-{}-txt", custom)
                    || name == format!("{}-license", custom)
                    || name == format!("{}-license-md", custom)
                    || name == format!("{}-license-txt", custom)
            }
            ref license => {
                let mut found = false;
                for lic in license.synonyms() {
                    if name == lic
                        || name == format!("license-{}", lic)
                        || name == format!("license-{}-md", lic)
                        || name == format!("license-{}-txt", lic)
                        || name == format!("{}-license", lic)
                        || name == format!("{}-license-md", lic)
                        || name == format!("{}-license-txt", lic)
                    {
                        found = true;
                        break;
                    }
                }
                found
            }
        }
    }

    let mut generic = None;
    let mut texts = vec![];
    for entry in fs::read_dir(package.manifest_path.parent().unwrap())? {
        let entry = entry?;
        let path = entry.path().clone();
        let name = entry.file_name().to_string_lossy().into_owned();

        if name_matches(&name, license) {
            if let Ok(text) = fs::read_to_string(&path) {
                let confidence = check_against_template(&text, license);
                texts.push(LicenseText {
                    path,
                    text,
                    confidence,
                });
            }
        } else if generic_license_name(&name) {
            if let Ok(text) = fs::read_to_string(&path) {
                let confidence = check_against_template(&text, license);
                generic = Some(LicenseText {
                    path,
                    text,
                    confidence,
                });
            }
        }
    }

    if texts.is_empty() {
        if let Some(generic) = generic {
            texts.push(generic);
        }
    }

    Ok(texts)
}
