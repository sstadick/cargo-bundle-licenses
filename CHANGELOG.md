# v2.2.0
- [fix](https://github.com/sstadick/cargo-bundle-licenses/pull/45) closes [issue 44](https://github.com/sstadick/cargo-bundle-licenses/issues/44), improve detection of BSD license files.

# v2.1.1
- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/41) Add Unicode-3.0 License from @jwodder

# v2.0.0
- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/32) Climb path and check workspace directory for LICENSE file from @BlueGreenMagick
- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/33) Add MPL-2.0 License from @BlueGreenMagick
- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/34) detect LICENSE-UNICODE file for Unicode-dfs-2016 in crate unicode-ident from @BlueGreenMagick
	= This change will cause existing generated files to now fail in CI, which is why I've bumped the verions to 2.0
- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/30) Add repository field to license from @troppmann
	- This is also a potentially breaking change to exsiting thirdparty yaml files

# v1.3.0
- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/28) Added ISC license @jwodder
- [bugfix](https://github.com/sstadick/cargo-bundle-licenses/pull/29) Bad ordering of generic license application @jwodder

# v1.2.0
- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/16) Remove build time libgit2 dep.

# v1.1.0

- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/3) Allow for finding workspace licenses when explicitly given a path.
- [bugfix](https://github.com/sstadick/cargo-bundle-licenses/pull/14) Sort licenses by SPDX string before comparing them when checking equality of `FinalizedLicense`.
- 

# v1.0.0 (added retroactively)

- Normalize $CARGO_HOME https://github.com/sstadick/cargo-bundle-licenses/pull/12 @skgland

# v0.5.0

- [feature](https://github.com/sstadick/cargo-bundle-licenses/pull/11) parse license expressions with spdx crate. Implemented by @skgland
