//! Contains all license related information
//!
//! See https://spdx.dev/wp-content/uploads/sites/41/2020/08/SPDX-specification-2-2.pdf for details on naming.
//!
//! For "exceptions" follow https://spdx.dev/wp-content/uploads/sites/41/2020/08/SPDX-specification-2-2.pdf#%5B%7B%22num%22%3A233%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C69%2C650%2C0%5D
//! and treat a license "with" "exception" as a new license, i.e. Apache-2.0 WITH LLVM-exception is treated as its own license of now.
use std::collections::{HashSet, VecDeque};
use std::{fmt, path::PathBuf, str::FromStr};

use slug::slugify;
use spdx::expression::ExprNode;
use spdx::ParseMode;

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum License {
    // Licenses specified in the [SPDX License List](https://spdx.org/licenses/)
    Unlicense,
    BSD_0_Clause,
    CC0_1_0,
    MIT,
    X11,
    BSD_2_Clause,
    BSD_3_Clause,
    BSL_1_0,
    Apache_2_0,
    Apache_2_0_WITH_LLVM_exception,
    LGPL_2_0,
    LGPL_2_1,
    LGPL_2_1Plus,
    LGPL_3_0,
    LGPL_3_0Plus,
    MPL_1_1,
    MPL_2_0,
    GPL_2_0,
    GPL_2_0Plus,
    GPL_3_0,
    GPL_3_0Plus,
    AGPL_3_0,
    AGPL_3_0Plus,
    Zlib,

    // Special cases
    Custom(String),
    File(PathBuf),
    Multiple(Vec<License>),
    Unspecified,
}

impl Default for License {
    fn default() -> License {
        License::Unspecified
    }
}

impl License {
    pub fn template(&self) -> Option<&'static str> {
        Some(match *self {
            License::Unlicense => include_str!("licenses/Unlicense"),
            License::MIT => include_str!("licenses/MIT"),
            License::Apache_2_0 => include_str!("licenses/Apache-2.0"),
            License::Apache_2_0_WITH_LLVM_exception => {
                include_str!("licenses/Apache-2.0_WITH_LLVM-exception")
            }
            License::BSD_0_Clause => include_str!("licenses/0BSD"),
            License::BSD_2_Clause => include_str!("licenses/BSD-2-Clause"),
            License::BSD_3_Clause => include_str!("licenses/BSD-3-Clause"),
            License::BSL_1_0 => include_str!("licenses/BSL-1.0"),
            License::GPL_2_0Plus => include_str!("licenses/GPL-2.0-or-later"),
            License::GPL_3_0Plus => include_str!("licenses/GPL-3.0-or-later"),
            License::LGPL_2_1Plus => include_str!("licenses/LGPL-2.1-or-later"),
            License::LGPL_3_0Plus => include_str!("licenses/LGPL-3.0-or-later"),
            License::Zlib => include_str!("licenses/Zlib"),
            License::Multiple(_) => unimplemented!(), // This should be impossible to hit
            _ => return None,
        })
    }
}

impl FromStr for License {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<License, core::convert::Infallible> {
        if let Ok(expr) = spdx::expression::Expression::parse_mode(s, ParseMode::LAX) {
            Ok(process_spdx_expression(expr))
        } else {
            Ok(simple_license(s))
        }
    }
}

fn simple_license(s: &str) -> License {
    match s.trim() {
        "Unlicense" => License::Unlicense,
        "0BSD" => License::BSD_0_Clause,
        "CC0-1.0" => License::CC0_1_0,
        "MIT" => License::MIT,
        "X11" => License::X11,
        "BSD-2-Clause" => License::BSD_2_Clause,
        "BSD-3-Clause" => License::BSD_3_Clause,
        "BSL-1.0" => License::BSL_1_0,
        "Apache-2.0" => License::Apache_2_0,
        "Apache-2.0 WITH LLVM-exception" => License::Apache_2_0_WITH_LLVM_exception,
        "LGPL-2.0-only" | "LGPL-2.0" => License::LGPL_2_0,
        "LGPL-2.1-only" | "LGPL-2.1" => License::LGPL_2_1,
        "LGPL-2.1-or-later" | "LGPL-2.1+" => License::LGPL_2_1Plus,
        "LGPL-3.0-only" | "LGPL-3.0" => License::LGPL_3_0,
        "LGPL-3.0-or-later" | "LGPL-3.0+" => License::LGPL_3_0Plus,
        "MPL-1.1" => License::MPL_1_1,
        "MPL-2.0" => License::MPL_2_0,
        "GPL-2.0-only" | "GPL-2.0" => License::GPL_2_0,
        "GPL-2.0-or-later" | "GPL-2.0+" => License::GPL_2_0Plus,
        "GPL-3.0-only" | "GPL-3.0" => License::GPL_3_0,
        "GPL-3.0-or-later" | "GPL-3.0+" => License::GPL_3_0Plus,
        "AGPL-3.0-only" | "AGPL-3.0" => License::AGPL_3_0,
        "AGPL-3.0-or-later" | "AGPL-3.0+" => License::AGPL_3_0Plus,
        "Zlib" => License::Zlib,
        // TODO: Sort out the SPDX "AND"
        s if s.contains('/') || s.contains(" OR ") => {
            let mut licenses = s
                .split('/')
                .flat_map(|s| s.split(" OR "))
                .map(str::parse)
                .map(Result::unwrap)
                .collect::<Vec<License>>();
            licenses.sort();
            License::Multiple(licenses)
        }
        s => License::Custom(s.to_owned()),
    }
}

fn process_spdx_expression(expr: spdx::Expression) -> License {
    let mut collection = Vec::new();
    let mut queue = expr.iter().collect::<VecDeque<_>>();

    while let Some(elem) = queue.pop_front() {
        match elem {
            ExprNode::Op(_) => { /*ignoring operators as we just need a list of used licenses and not how they are combined*/
            }
            ExprNode::Req(req) => collection.push(simple_license(&req.req.to_string())),
        }
    }

    let mut tmp = HashSet::new();

    // de-duplicate while retaining the order
    collection.retain(|elem| tmp.insert(elem.to_owned()));

    match &collection.as_slice() {
        [single] => single.to_owned(),
        [_, ..] => License::Multiple(collection.to_vec()),
        [] => simple_license(expr.as_ref()),
    }
}

impl fmt::Display for License {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            License::Unlicense => write!(w, "Unlicense"),
            License::BSD_0_Clause => write!(w, "0BSD"),
            License::CC0_1_0 => write!(w, "CC0-1.0"),
            License::MIT => write!(w, "MIT"),
            License::X11 => write!(w, "X11"),
            License::BSD_2_Clause => write!(w, "BSD-2-Clause"),
            License::BSD_3_Clause => write!(w, "BSD-3-Clause"),
            License::BSL_1_0 => write!(w, "BSL-1.0"),
            License::Apache_2_0 => write!(w, "Apache-2.0"),
            License::Apache_2_0_WITH_LLVM_exception => write!(w, "Apache-2.0 WITH LLVM-exception"),
            License::LGPL_2_0 => write!(w, "LGPL-2.0-only"),
            License::LGPL_2_1 => write!(w, "LGPL-2.1-only"),
            License::LGPL_2_1Plus => write!(w, "LGPL-2.1-or-later"),
            License::LGPL_3_0 => write!(w, "LGPL-3.0-only"),
            License::LGPL_3_0Plus => write!(w, "LGPL-3.0-or-later"),
            License::MPL_1_1 => write!(w, "MPL-1.1"),
            License::MPL_2_0 => write!(w, "MPL-2.0"),
            License::GPL_2_0 => write!(w, "GPL-2.0-only"),
            License::GPL_2_0Plus => write!(w, "GPL-2.0-or-later"),
            License::GPL_3_0 => write!(w, "GPL-3.0-only"),
            License::GPL_3_0Plus => write!(w, "GPL-3.0-or-later"),
            License::AGPL_3_0 => write!(w, "AGPL-3.0-only"),
            License::AGPL_3_0Plus => write!(w, "AGPL-3.0-or-later"),
            License::Zlib => write!(w, "Zlib"),
            License::Custom(ref s) => write!(w, "{}", s),
            License::File(ref f) => {
                write!(w, "License specified in file ({})", f.to_string_lossy())
            }
            License::Multiple(ref ls) => {
                write!(w, "{}", ls[0])?;
                for l in ls.iter().skip(1) {
                    write!(w, " / {}", l)?;
                }
                Ok(())
            }
            License::Unspecified => write!(w, "No license specified"),
        }
    }
}

impl License {
    /// Slugified synonyms returned with the longest one first on the assumption that it is more specific
    pub fn synonyms(&self) -> Vec<String> {
        let mut synonyms = match self {
            License::Apache_2_0 => vec![
                slugify(self.to_string()).to_lowercase(),
                String::from("apache"),
                String::from("apache2"),
                String::from("apache-2"),
            ],
            License::BSL_1_0 => vec![
                slugify(self.to_string()).to_lowercase(),
                String::from("boost"),
            ],
            _ => vec![slugify(self.to_string()).to_lowercase()],
        };
        synonyms.sort_by_key(|value| -(value.len() as i64));
        synonyms
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_spdx() {
        assert_eq!(License::from_str("Unlicense"), Ok(License::Unlicense));
        assert_eq!(License::from_str("0BSD"), Ok(License::BSD_0_Clause));
        assert_eq!(License::from_str("CC0-1.0"), Ok(License::CC0_1_0));
        assert_eq!(License::from_str("MIT"), Ok(License::MIT));
        assert_eq!(License::from_str("X11"), Ok(License::X11));
        assert_eq!(License::from_str("BSD-2-Clause"), Ok(License::BSD_2_Clause));
        assert_eq!(License::from_str("BSD-3-Clause"), Ok(License::BSD_3_Clause));
        assert_eq!(License::from_str("BSL-1.0"), Ok(License::BSL_1_0));
        assert_eq!(License::from_str("Apache-2.0"), Ok(License::Apache_2_0));
        assert_eq!(License::from_str("LGPL-2.0-only"), Ok(License::LGPL_2_0));
        assert_eq!(License::from_str("LGPL-2.0"), Ok(License::LGPL_2_0));
        assert_eq!(License::from_str("LGPL-2.1-only"), Ok(License::LGPL_2_1));
        assert_eq!(License::from_str("LGPL-2.1"), Ok(License::LGPL_2_1));
        assert_eq!(
            License::from_str("LGPL-2.1-or-later"),
            Ok(License::LGPL_2_1Plus)
        );
        assert_eq!(License::from_str("LGPL-2.1+"), Ok(License::LGPL_2_1Plus));
        assert_eq!(License::from_str("LGPL-3.0-only"), Ok(License::LGPL_3_0));
        assert_eq!(License::from_str("LGPL-3.0"), Ok(License::LGPL_3_0));
        assert_eq!(
            License::from_str("LGPL-3.0-or-later"),
            Ok(License::LGPL_3_0Plus)
        );
        assert_eq!(License::from_str("LGPL-3.0+"), Ok(License::LGPL_3_0Plus));
        assert_eq!(License::from_str("MPL-1.1"), Ok(License::MPL_1_1));
        assert_eq!(License::from_str("MPL-2.0"), Ok(License::MPL_2_0));
        assert_eq!(License::from_str("GPL-2.0-only"), Ok(License::GPL_2_0));
        assert_eq!(License::from_str("GPL-2.0"), Ok(License::GPL_2_0));
        assert_eq!(
            License::from_str("GPL-2.0-or-later"),
            Ok(License::GPL_2_0Plus)
        );
        assert_eq!(License::from_str("GPL-2.0+"), Ok(License::GPL_2_0Plus));
        assert_eq!(License::from_str("GPL-3.0-only"), Ok(License::GPL_3_0));
        assert_eq!(License::from_str("GPL-3.0"), Ok(License::GPL_3_0));
        assert_eq!(
            License::from_str("GPL-3.0-or-later"),
            Ok(License::GPL_3_0Plus)
        );
        assert_eq!(License::from_str("GPL-3.0+"), Ok(License::GPL_3_0Plus));
        assert_eq!(License::from_str("AGPL-3.0-only"), Ok(License::AGPL_3_0));
        assert_eq!(License::from_str("AGPL-3.0"), Ok(License::AGPL_3_0));
        assert_eq!(
            License::from_str("AGPL-3.0-or-later"),
            Ok(License::AGPL_3_0Plus)
        );
        assert_eq!(License::from_str("AGPL-3.0+"), Ok(License::AGPL_3_0Plus));
        assert_eq!(License::from_str("Zlib"), Ok(License::Zlib));
    }

    #[test]
    fn simple_with_exception() {
        assert_eq!(
            License::from_str("Apache-2.0 WITH LLVM-exception"),
            Ok(License::Apache_2_0_WITH_LLVM_exception)
        );
    }

    #[test]
    fn complex_spdx() {
        assert_eq!(
            License::from_str("Apache-2.0 OR MIT"),
            Ok(License::Multiple(vec![License::Apache_2_0, License::MIT]))
        );
        assert_eq!(
            License::from_str("Apache-2.0 / MIT"),
            Ok(License::Multiple(vec![License::Apache_2_0, License::MIT]))
        );
        assert_eq!(
            License::from_str("(Apache-2.0 OR MIT) AND BSD-3-Clause"),
            Ok(License::Multiple(vec![
                License::Apache_2_0,
                License::MIT,
                License::BSD_3_Clause
            ]))
        );
    }
}
