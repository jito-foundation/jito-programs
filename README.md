# Jito Programs
This repository is home to Jito's on-chain programs that enable MEV collection and MEV sharing with SOL stakers; additionally
we may host useful on-chain program reference implementations here.

## Version Management

This project uses `cargo-release` for version management. The release process is configured to work with version branches (e.g., `v3.0.x`).

### Prerequisites for Release

1. **Install cargo-release** (if not already installed):
   ```bash
   cargo install cargo-release
   ```

### Release Process

1. **Run the release command**:
   ```bash
   cargo release patch --execute
   ```

This will:
- Bump the version number
- Create a git tag
- Push changes and tags
- Follow the configuration in `release.toml`

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
