# cargo-bundle-licenses

<p align="center">
  <a href="https://github.com/sstadick/cargo-bundle-licenses/actions?query=workflow%3ACheck"><img src="https://github.com/sstadick/cargo-bundle-licenses/workflows/Check/badge.svg" alt="Build Status"></a>
  <img src="https://img.shields.io/crates/l/cargo-bundle-licenses.svg" alt="license">
  <a href="https://crates.io/crates/cargo-bundle-licenses"><img src="https://img.shields.io/crates/v/cargo-bundle-licenses.svg?colorB=319e8c" alt="Version info"></a><br>
</p>

Bundle all third-party licenses into a single file.


**NOTE** This tools is not a lawyer and no guarantee of correctness can be made regarding the licenses that it selects. This tool relies on the information supplied in package metadata to be correct, this is not guaranteed so for "real" scenarios it is recommended that all licenses be reviewed and verified manually as well.

## Install

```bash
cargo install cargo-bundle-licenses
```

## Usage

The typical use case for this tool is as follows:

1. Generate an initial bundle file:

```bash
cargo bundle-licenses --format yaml --output THIRDPARTY.yml
```

2. Go through the listed warnings and track down licenses that could not be found and paste the text of the license into the "THIRDPARTY.yml" file.
   - Note: if the licence _should_ have been found by `cargo-bundle-licenses` then please create an issue, or even better, a pull request!
3. In your CI, run `cargo-bundle-licenses` in the following way to check for changes and fail if they are found. This will generate a new thirdparty file, apply any licenses that have been added by hand to fill in the "NOT FOUND" licenses, and then compare the newly generated version against the previous version and fail if there are _any_ differences.
   
```bash
cargo bundle-licenses --format yaml --output CI.yaml --previous THIRDPARTY.yml --check-previous
```

4. Export a human readable format
```bash
cargo bundle-licenses --previous .\THIRDPARTY.yml --human-readable-artifact --output "THIRDPARTY.txt"
```

To aid in copy-pasting licenses that couldn't be found for one reason other another the [esc](https://github.com/sstadick/esc) tool can be used to properly create escaped license strings to drop into the format of your choosing. 

## Formats

Currently the supported formats are `json`, `yaml`, and `toml`. 

You can also export those previously exported (and maybe edited) files to a human-readable format with `--human-readable-artifact`

## Common warnings and resolutions

The most common cause of missing licenses seems to be workspaces that don't `include` forward their license files. Go to the repo for the workspace and copy the relevant files from there.

A package license may receive a confidence warning stating that `cargo-bundle-licenses` is "unsure" or "semi" confident. This means that when the found license was compared to a template license it was found to have diverged in more than a few words. You should verify that the licence text is in fact correct in these cases.

## Differences from other tools

- `cargo-about` also finds and aggregates the license texts of all dependencies. `cargo-bundle-licenses` is different in that it is focused more on generating _something_ and then allowing that to be manually updated with not-found licenses whereas `cargo-about` will not generate output in that scenario. Additionally, `cargo-bundle-licenses` allows for manual updates to be applied to later versions of the thirdparty file. However, `cargo-about` does generate a very nice human readable format, and provides more granularity in what packages / licenses are allowed.
- `cargo-license` only generates a list of licenses as found in the `Cargo.toml` files.
- `cargo-deny` also gathers a list of licenses for dependencies, as well as analysing `LICENSE` files, but it does so with the goal of warning or erroring if any dependencies have licenses that the user deems unacceptable; no collection of license text is emitted

## Attributions

This crate was heavily inspired by [`cargo-lichking`](https://github.com/Nemo157/cargo-lichking).
