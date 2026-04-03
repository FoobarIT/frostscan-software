# Roadmap

## 
Create a simple disk exploration and analysis tool.

---

## v0.0.2 — Usability, Analysis & Infrastructure

### Features
- [x] Add English language support as main
- [x] Add a quick overview of storage usage by file extension
- [x] Add a dedicated Tree Map page
- [x] Add top extensions summary with size, count, and percentage
### Core
- [x] Refactor filesystem scanning into a dedicated module
- [x] Changing the way we get system disks to support multiple platforms
- [x] Improve recursive scan performance
- [x] Handle symlinks and permission errors safely
- [x] Add scan progress reporting and cancellation support
### Performance
- [x] Investigate tab navigation performance
- [x] Cache computed results between tabs
- [x] Move heavy computations off the UI thread
### Quality
- [x] Add unit tests for extension aggregation and size calculation
- [x] Add integration tests for directory scanning
- [x] Improve error handling and logging
### CI / Project setup
- [x] Setup GitHub Actions workflow
- [x] Add `cargo fmt`, `cargo clippy`, and `cargo test` checks
- [x] Add cross-platform build validation
- [x] Add contributing notes to README

