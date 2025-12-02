# Jito Programs
This repository is home to Jito's on-chain programs that enable MEV collection and MEV sharing with SOL stakers; additionally
we may host useful on-chain program reference implementations here.

## Version Management

### Release Process

1. **Run the release command**:
   ```bash
   ./release
   ```

This will:
- Bump the version number
- Rebuild the IDLs
- Create a git tag
- Push changes and tags
- Follow the configuration in `release.toml`

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
