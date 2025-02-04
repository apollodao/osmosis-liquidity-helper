# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [0.4.0] - 2024-08-28

### Changed

- Bump `cw-dex-astroport` to version `0.2.0`.
- Bump `cw-dex-osmosis` to version `0.2.0`.
- Use re-exported version of `cw-dex` from `cw-dex-astroport` and `cw-dex-osmosis` packages.
- Bump `cw-it` to version `0.4.0`.


# [0.3.3] - 2024-04-09

### Changed

- Bump `cosmwasm-std` to version `1.5.3`.
- Use Pool types from `cw-dex-astroport` and `cw-dex-osmosis` packages in place of the deprecated ones in `cw-dex`.
- Support Astroport pair type `astroport-pair-xyk-sale-tax`.

# [0.3.2] - 2024-02-06

### Changed

- Bump `cw-dex` to version `0.5.1`.

## [0.3.1] - 2023-11-06

### Added

- Added the migrate entrypoint for contract `astroport-liquidity-helper`.

## [0.3.0] - 2023-10-30

### Added

- Added support for Astroport PCL pair type.

### Changed

- Upgrade cw-dex to 0.5.0

## [0.2.2] - 2023-10-25

### Changed

- Upgrade cw-dex to 0.4.1

## [0.2.1] - 2023-09-26

### Changed

- Use cw-bigint from crates.io instead of git
- Upgrade cw-dex to 0.4.0

## [0.2.0] - 2023-08-15

### Changed

- Update all dependencies, including Astroport to version 2.8.0
