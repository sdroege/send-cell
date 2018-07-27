# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html),
specifically the [variant used by Rust](http://doc.crates.io/manifest.html#the-version-field).

## [0.1.4] - 2018-07-27
### Changed
- This is now only a thing wrapper around fragile::Fragile instead of
  re-implementing the same thing again. Please use fragile directly.

## [0.1.3] - 2018-04-07
### Fixed
- Avoid running Drop::drop() from the incorrect thread and make sure to leak
  the contained value. Otherwise it would still be dropped even though we
  panic.

### Added
- Crate-level documentation instead of just for the struct/functions

## [0.1.2] - 2017-08-06
### Fixed
- Fix up test-suite

## [0.1.1] - 2017-08-05
### Fixed
- Panic in any of the trait impls too if used from the wrong thread instead
  of just when dereferencing.
- Panic in Drop::drop() too if called from the wrong thread.

### Changed
- Improve panic messages

## [0.1.0] - 2017-08-04

- Initial release of send-cell.

[Unreleased]: https://github.com/sdroege/send-cell/compare/0.1.4...HEAD
[0.1.4]: https://github.com/sdroege/send-cell/compare/0.1.2...0.1.4
[0.1.3]: https://github.com/sdroege/send-cell/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/sdroege/send-cell/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/sdroege/send-cell/compare/0.1.0...0.1.1
