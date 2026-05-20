# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.56] - 2026-05-20

### Added
- CHANGELOG.md for release tracking
- Improved crates.io metadata (homepage, documentation URLs)
- GitHub Release workflow now publishes to crates.io automatically

### Changed
- Release workflow creates published (non-draft) releases
- Release workflow generates release notes from commits

## [1.0.55] - 2026-05-20

### Changed
- Updated dependencies (obs, Cargo)

### Fixed
- Correct guard clearing by using single dereference

## [1.0.31] - 2026-05-18

### Changed
- Add contents:write permission to release workflow

## [1.0.30] - 2026-05-18

### Changed
- Reformat with latest rustfmt

## [1.0.29] - 2026-05-18

### Fixed
- Fix CI — pin dtolnay/rust-toolchain@stable

## [1.0.28] - 2026-05-18

### Changed
- Clean CI/CD workflows (Swatinem/rust-cache, reduced steps)

## [1.0.27] - 2026-05-18

### Fixed
- Keyword limit fix (crates.io max 5)

## [1.0.26] - 2026-05-18

### Changed
- Improved package metadata and GitHub repo topics

## [1.0.25] - 2026-05-18

### Fixed
- Fix test_expand_home under sandboxed HOME

## [1.0.24] - 2026-05-18

### Changed
- Docs and crates.io metadata sync

## [1.0.22-1.0.23] - 2026-05-18

### Fixed
- OBS WebSocket connection (ws:// scheme in TcpStream + missing rpcVersion serde rename)
- Continuous background reconnection, no 10-attempt cap