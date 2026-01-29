use std::path::PathBuf;

/// Represents a download source with priority and metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadSource {
    /// The URL to download from
    pub url: String,

    /// Priority (lower = higher priority, 0 = highest)
    pub priority: u32,

    /// Expected checksum for this specific source
    pub checksum: Option<[u8; 32]>,

    /// Source type/category
    pub source_type: SourceType,

    /// Optional geographic region hint
    pub region: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceType {
    /// Primary/official source
    Primary,

    /// Mirror/replica
    Mirror,

    /// CDN edge location
    Cdn,

    /// Fallback source
    Fallback,
}

impl DownloadSource {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            priority: 0,
            checksum: None,
            source_type: SourceType::Primary,
            region: None,
        }
    }

    pub fn priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn checksum(mut self, checksum: [u8; 32]) -> Self {
        self.checksum = Some(checksum);
        self
    }

    pub fn source_type(mut self, source_type: SourceType) -> Self {
        self.source_type = source_type;
        self
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }
}

/// Multi-source download configuration
#[derive(Clone, Debug)]
pub struct MultiSourceOptions {
    /// List of sources to try
    pub sources: Vec<DownloadSource>,

    /// Strategy for selecting sources
    pub strategy: SourceSelectionStrategy,

    /// Whether to verify all sources have same content
    pub verify_consistency: bool,

    /// Timeout for each source attempt
    pub per_source_timeout: Option<std::time::Duration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceSelectionStrategy {
    /// Try in priority order until success
    Priority,

    /// Try fastest responding source first
    FastestFirst,

    /// Try geographically closest source first
    Geographic,

    /// Try all sources in parallel, use first success
    RaceAll,
}
