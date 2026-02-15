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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_source_new() {
        let source = DownloadSource::new("https://example.com/file");
        assert_eq!(source.url, "https://example.com/file");
        assert_eq!(source.priority, 0);
        assert!(source.checksum.is_none());
        assert_eq!(source.source_type, SourceType::Primary);
        assert!(source.region.is_none());
    }

    #[test]
    fn test_download_source_builder() {
        let checksum = [1u8; 32];
        let source = DownloadSource::new("https://example.com/file")
            .priority(5)
            .checksum(checksum)
            .source_type(SourceType::Mirror)
            .region("us-west");

        assert_eq!(source.url, "https://example.com/file");
        assert_eq!(source.priority, 5);
        assert_eq!(source.checksum, Some(checksum));
        assert_eq!(source.source_type, SourceType::Mirror);
        assert_eq!(source.region, Some("us-west".to_string()));
    }

    #[test]
    fn test_download_source_clone() {
        let source = DownloadSource::new("https://example.com/file")
            .priority(3)
            .source_type(SourceType::Cdn);

        let cloned = source.clone();
        assert_eq!(cloned, source);
    }

    #[test]
    fn test_source_type_equality() {
        assert_eq!(SourceType::Primary, SourceType::Primary);
        assert_ne!(SourceType::Primary, SourceType::Mirror);
        assert_ne!(SourceType::Mirror, SourceType::Cdn);
        assert_ne!(SourceType::Cdn, SourceType::Fallback);
    }

    #[test]
    fn test_multi_source_options_default() {
        // Test that MultiSourceOptions can be created
        let options = MultiSourceOptions {
            sources: vec![],
            strategy: SourceSelectionStrategy::Priority,
            verify_consistency: false,
            per_source_timeout: None,
        };

        assert!(options.sources.is_empty());
        assert_eq!(options.strategy, SourceSelectionStrategy::Priority);
        assert!(!options.verify_consistency);
        assert!(options.per_source_timeout.is_none());
    }

    #[test]
    fn test_multi_source_options_with_sources() {
        let source1 = DownloadSource::new("https://primary.example.com")
            .priority(0)
            .source_type(SourceType::Primary);

        let source2 = DownloadSource::new("https://mirror.example.com")
            .priority(1)
            .source_type(SourceType::Mirror)
            .region("eu");

        let options = MultiSourceOptions {
            sources: vec![source1.clone(), source2.clone()],
            strategy: SourceSelectionStrategy::FastestFirst,
            verify_consistency: true,
            per_source_timeout: Some(std::time::Duration::from_secs(30)),
        };

        assert_eq!(options.sources.len(), 2);
        assert_eq!(options.sources[0], source1);
        assert_eq!(options.sources[1], source2);
        assert_eq!(options.strategy, SourceSelectionStrategy::FastestFirst);
        assert!(options.verify_consistency);
        assert_eq!(
            options.per_source_timeout,
            Some(std::time::Duration::from_secs(30))
        );
    }

    #[test]
    fn test_source_selection_strategies() {
        let strategies = vec![
            SourceSelectionStrategy::Priority,
            SourceSelectionStrategy::FastestFirst,
            SourceSelectionStrategy::Geographic,
            SourceSelectionStrategy::RaceAll,
        ];

        // Test all strategies are different
        for (i, strategy1) in strategies.iter().enumerate() {
            for (j, strategy2) in strategies.iter().enumerate() {
                if i != j {
                    assert_ne!(strategy1, strategy2);
                }
            }
        }
    }

    #[test]
    fn test_download_source_with_string_conversion() {
        let url_string = "https://example.com/file".to_string();
        let source = DownloadSource::new(url_string.clone());
        assert_eq!(source.url, url_string);

        let region_string = "us-east".to_string();
        let source_with_region = source.region(region_string.clone());
        assert_eq!(source_with_region.region, Some(region_string));
    }

    #[test]
    fn test_download_source_debug() {
        let source = DownloadSource::new("https://example.com/file")
            .priority(2)
            .source_type(SourceType::Cdn)
            .region("us-west");

        let debug_str = format!("{:?}", source);
        assert!(debug_str.contains("DownloadSource"));
        assert!(debug_str.contains("https://example.com/file"));
        assert!(debug_str.contains("priority: 2"));
        assert!(debug_str.contains("Cdn"));
        assert!(debug_str.contains("us-west"));
    }

    #[test]
    fn test_multi_source_options_debug() {
        let options = MultiSourceOptions {
            sources: vec![DownloadSource::new("https://example.com")],
            strategy: SourceSelectionStrategy::RaceAll,
            verify_consistency: true,
            per_source_timeout: Some(std::time::Duration::from_secs(60)),
        };

        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("MultiSourceOptions"));
        assert!(debug_str.contains("RaceAll"));
        assert!(debug_str.contains("verify_consistency: true"));
    }
}
