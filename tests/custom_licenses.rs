#![cfg(feature = "binary-dependencies")]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use bundle_licenses_lib::{bundle::Bundle, format::Format};

// Each CLI invocation gets its own current directory, so the tests can run in
// parallel without changing the test process's working directory. Use only
// path dependencies, with networking disabled, to keep these fixtures offline.
struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(license: &str) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let dir = loop {
            let dir = std::env::temp_dir().join(format!(
                "cargo-bundle-licenses-test-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&dir) {
                Ok(()) => break dir,
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("could not create test directory: {}", err),
            }
        };
        let project = Self { dir };
        fs::create_dir_all(project.dir.join("src")).unwrap();
        fs::create_dir_all(project.dir.join("dep/src")).unwrap();
        fs::write(
            project.dir.join("Cargo.toml"),
            "[package]\nname = 'license-test-root'\nversion = '0.1.0'\nedition = '2021'\n\
             [dependencies]\ncustom-dep = { path = 'dep' }\n",
        )
        .unwrap();
        fs::write(project.dir.join("src/lib.rs"), "").unwrap();
        fs::write(
            project.dir.join("dep/Cargo.toml"),
            format!(
                "[package]\nname = 'custom-dep'\nversion = '0.1.0'\nedition = '2021'\nlicense = '{license}'\n"
            ),
        )
        .unwrap();
        fs::write(project.dir.join("dep/src/lib.rs"), "").unwrap();
        project
    }

    fn license_file(&self, name: &str, text: &str) {
        fs::write(self.dir.join("dep").join(name), text).unwrap();
    }

    fn run(&self, format: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-bundle-licenses"))
            .current_dir(&self.dir)
            .env("CARGO_NET_OFFLINE", "true")
            .args(["--format", format])
            .args(args)
            .output()
            .unwrap()
    }

    fn bundle(&self) -> Bundle {
        parse_output(self.run("json", &[]), Format::Json)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn parse_output(output: Output, format: Format) -> Bundle {
    assert!(
        output.status.success(),
        "command failed: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    format
        .deserialize_from_reader(output.stdout.as_slice())
        .unwrap()
}

#[test]
fn discovers_bare_custom_and_explicit_reference_filenames() {
    for name in ["DoesNotExist42", "LicenseRef-DoesNotExist42"] {
        let project = Project::new(name);
        project.license_file(&format!("LICENSE-{name}"), "The custom license text\n");
        let bundle = project.bundle();
        assert_eq!(bundle.third_party_libraries().len(), 1);
        let package = &bundle.third_party_libraries()[0];
        assert_eq!(package.license, name);
        assert_eq!(package.licenses.len(), 1);
        assert_eq!(package.licenses[0].license, name);
        assert_eq!(package.licenses[0].text, "The custom license text\n");
    }
}

#[test]
fn discovers_all_licenses_in_mixed_expressions() {
    for expression in [
        "DoesNotExist42 OR MIT",
        "MIT OR DoesNotExist42 AND Apache-2.0",
        "(DoesNotExist42 OR MIT) AND Apache-2.0",
    ] {
        let project = Project::new(expression);
        project.license_file("LICENSE-DoesNotExist42", "Custom license text\n");
        project.license_file("LICENSE-MIT", include_str!("../src/licenses/MIT"));
        project.license_file("LICENSE-APACHE", include_str!("../src/licenses/Apache-2.0"));
        let bundle = project.bundle();
        let package = &bundle.third_party_libraries()[0];
        assert_eq!(package.license, expression);
        let mut actual: Vec<_> = package
            .licenses
            .iter()
            .map(|license| (license.license.as_str(), license.text.as_str()))
            .collect();
        actual.sort();
        let mut expected = vec![
            ("DoesNotExist42", "Custom license text\n"),
            ("MIT", include_str!("../src/licenses/MIT")),
        ];
        if expression.contains("Apache-2.0") {
            expected.push(("Apache-2.0", include_str!("../src/licenses/Apache-2.0")));
        }
        expected.sort();
        assert_eq!(actual, expected, "{expression}");
    }
}

#[test]
fn reuses_previous_custom_text_and_checks_unchanged_bundles() {
    for (format, name) in [
        (Format::Json, "json"),
        (Format::Yaml, "yaml"),
        (Format::Toml, "toml"),
    ] {
        for has_license_file in [false, true] {
            let project = Project::new("DoesNotExist42");
            let text = "Manually verified custom license text\n";
            if has_license_file {
                project.license_file("LICENSE-DoesNotExist42", text);
            }
            // Construct the pre-PR output explicitly, rather than generating it
            // with the executable under test (which might rename the key).
            let previous: Bundle = serde_json::from_value(serde_json::json!({
                "root_name": "license-test-root",
                "third_party_libraries": [{
                    "package_name": "custom-dep",
                    "package_version": "0.1.0",
                    "repository": "",
                    "license": "DoesNotExist42",
                    "licenses": [{ "license": "DoesNotExist42", "text": text }]
                }]
            }))
            .unwrap();
            format
                .serialize_to_writer(
                    fs::File::create(project.dir.join("previous.bundle")).unwrap(),
                    &previous,
                )
                .unwrap();
            let current = parse_output(
                project.run(name, &["--previous", "previous.bundle", "--check-previous"]),
                format,
            );
            // Check the full serialized content as well as the CLI exit status.
            assert_eq!(
                serde_json::to_value(current).unwrap(),
                serde_json::to_value(previous).unwrap()
            );
        }
    }
}
