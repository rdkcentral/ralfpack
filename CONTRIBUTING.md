# Contributing to ralfpack

Thank you for your interest in contributing to ralfpack! We welcome bug reports, feature requests, documentation
improvements, and code contributions.

## Table of Contents
- [Overview](#overview)
- [Reporting Issues](#reporting-issues)
- [Requesting Features](#requesting-features)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Submitting Changes](#submitting-changes)
- [Testing](#testing)
- [Contact](#contact)

## Overview
If you would like to contribute code to this project you can do so through GitHub by forking the repository and
sending a pull request.

Before RDK accepts your code into the project you must sign the RDK Contributor License Agreement (CLA).

## Reporting Issues
- Search [existing issues](https://github.com/rdkcentral/ralfpack/issues) before opening a new one.
- Include steps to reproduce, expected behavior, and environment details (OS, Rust version).

## Requesting Features
- Open a GitHub issue describing the feature and its use case.
- Suggest implementation ideas if possible.

## Development Setup
1. Install [Rust](https://rustup.rs/) (latest stable recommended).
2. This project builds C code, so ensure you have a working C toolchain (e.g., `gcc`, `ar`) and autotools
   (e.g., `automake`, `autoconf`) installed.  In addition, on some platforms `clang` may be required.
   ```sh
    # On Ubuntu/Debian
    sudo apt-get install build-essential clang automake autoconf libtool pkg-config
    # On Fedora
    sudo dnf install gcc gcc-c++ clang make automake autoconf libtool pkgconf
    # On macOS (using Homebrew)
    brew install automake autoconf libtool pkg-config
   ```
3. Clone the repository:
   ```sh
   git clone https://github.com/rdkcentral/ralfpack
   cd ralfpack
   ```
4. Build the project:
   ```sh
   cargo build --release
   ```
5. Run tests:
   ```sh
   cargo test
   ```

## Coding Standards
- Follow [Rust style guidelines](https://github.com/rust-lang/rustfmt).
- Run `cargo fmt` before submitting.
- Document public functions and modules.
- Prefer small, focused commits.

## Submitting Changes
1. Fork the repository and create a branch:
   ```sh
   git checkout -b feature/your-feature
   ```
2. Make your changes and commit with clear messages.
3. Run all tests and ensure they pass.
4. Open a pull request (PR) on GitHub. Describe your changes and reference related issues.

## Testing
- Add or update tests for new features and bug fixes.
- Run `cargo test` before submitting.
- For integration with C code, ensure relevant scripts and build steps are tested.

## Contact
For questions, reach out via GitHub issues or email the maintainers listed in `Cargo.toml`.

---
Thank you for helping improve ralfpack!

