# Roadmap

## 
Create a simple disk exploration and analysis tool.

---
## v0.0.3 - Polish, Stability & User Experience
### Features
- [ ] Add a settings page for user preferences
- [ ] Add a light mode theme
- [ ] Add a file type filter for the Tree Map
- [ ] Add duplication detection and visualization
### Core
- [ ] Refactor the UI to use a more modular component structure
- [ ] Improve error handling and user feedback for failed scans
- [ ] Add support for scanning network drives and external storage
- [ ] Dictated module text file structure and naming conventions.

### Performance
- [ ] Optimize Tree Map rendering for large datasets
- [ ] Implement lazy loading for directory contents in the Tree Map
- [ ] Add caching of scan results to speed up subsequent scans
### Quality
- [ ] Add end-to-end tests for the entire scanning and visualization workflow
- [ ] Add performance benchmarks for scanning and rendering
- [ ] Get user feedback and iterate on the UI/UX design
### CI / Project setup
- [ ] Investigate for failure CI runs and fix them.
- [ ] Add script helper for checking crate versions. 

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

