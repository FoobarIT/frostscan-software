# FrostScan

FrostScan is a disk usage analyzer built in Rust with the Iced GUI framework. It scans folders, computes directory sizes asynchronously, and presents the results in a responsive desktop interface.

## Status

FrostScan is currently an alpha-stage project. The codebase is actively evolving, but it already includes a working explorer, directory size scanning, multilingual UI, and basic project automation.

## Current Features

- Disk selection and folder exploration
- Asynchronous directory size computation
- Scan cancellation and progress footer
- Safe handling of symlinks and partial scans
- Extension overview for the current folder
- English and French UI
- Cross-platform CI validation on Windows, Linux, and macOS

## Planned Next Steps

- Dedicated Tree Map page
- Richer extension statistics with percentages
- Faster scanning on large folders
- More work moved off the UI thread

See the full roadmap in [ROADMAP.md](./ROADMAP.md).

## Getting Started

### Requirements

- Rust toolchain
- Platform GUI dependencies for Iced

### Run locally

```bash
cargo run
```

### Quality checks

```bash
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Contributing

Use GitHub Issues to report bugs or suggest features. Pull requests are welcome!
If you want to add code or documentation, please create a new branch from `dev` like `dev/feature-name`. And follow the actual roadmap for the next features to implement.

## Credits

- [Iced](https://github.com/iced-rs/iced)
- [Rust](https://www.rust-lang.org/)
