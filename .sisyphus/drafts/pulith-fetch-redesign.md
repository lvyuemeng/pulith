# Draft: pulith-fetch redesign

## Requirements (confirmed)
- Apply the redesign plan from docs/design/fetch.md
- Fix consistency issues (e.g., Result<T> type alias already exists)
- Implement missing functionality based on design document
- Follow pulith ecosystem patterns (F1-F5 principles)

## Technical Decisions
- The crate already has Result<T> type alias in place (no changes needed)
- Current structure is mostly correct but missing implementations
- Need to implement all the missing effect modules (multi_source, resumable, segmented, batch, cache)
- Need to implement transform modules (decompress, verify)

## Research Findings
- Current crate is at v0.2.0 with basic HTTP download functionality
- Error handling infrastructure is already in place
- Module structure follows data/core/effects/transform pattern
- Contains several placeholder implementations (e.g., fetcher.rs has todo! macro)
- Has 1 active panic! in segment.rs that needs to be fixed
- Contains backup file with unwrap() calls but not in production code

## Open Questions
- How to prioritize the implementation of the 5 phases from the design document?

## Scope Boundaries
- INCLUDE: Complete implementation of pulith-fetch redesign
- INCLUDE: All missing modules and functionality from design doc
- EXCLUDE: Changes to other pulith crates
- EXCLUDE: Implementation of protocol extensions (deferred to v1.0+)