# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0](https://github.com/insd47/branchline/compare/v0.1.0...v0.2.0) - 2026-11-06

### Changed

- [**breaking**] generate the client in one call with transport-generic nodes ([#1](https://github.com/insd47/branchline/pull/1))

## [0.1.0] - 2026-11-06

### Added

- Generate a typed resource-chain client that follows the OpenAPI 3.1 path tree, in `build.rs`
- `bon` builders for generated types, with required fields checked at compile time
- `Http` and `SigV4` transports behind the `reqwest` and `sigv4` features
