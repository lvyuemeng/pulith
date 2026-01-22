use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PermissionStrategy {
    #[default]
    Standard,
    ReadOnly,
    Preserve,
    Owned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HashStrategy {
    #[default]
    None,
    Sha256,
    Blake3,
}

#[derive(Clone, Default)]
pub struct ExtractionOptions {
    pub permission_strategy: PermissionStrategy,
    pub hash_strategy: HashStrategy,
    pub strip_components: usize,
    pub expected_total_bytes: Option<u64>,
    pub on_progress: Option<Arc<dyn Fn(Progress) + Send + Sync>>,
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub bytes_processed: u64,
    pub total_bytes: Option<u64>,
    pub percentage: Option<f32>,
    pub current_file: Option<PathBuf>,
}

impl ExtractionOptions {
    pub fn permission_strategy(mut self, strategy: PermissionStrategy) -> Self {
        self.permission_strategy = strategy;
        self
    }

    pub fn hash_strategy(mut self, strategy: HashStrategy) -> Self {
        self.hash_strategy = strategy;
        self
    }

    pub fn strip_components(mut self, n: usize) -> Self {
        self.strip_components = n;
        self
    }

    pub fn expected_total_bytes(mut self, bytes: u64) -> Self {
        self.expected_total_bytes = Some(bytes);
        self
    }

    pub fn on_progress(mut self, callback: Arc<dyn Fn(Progress) + Send + Sync>) -> Self {
        self.on_progress = Some(callback);
        self
    }
}

impl Progress {
    pub fn percentage(&self) -> Option<f32> {
        self.total_bytes.and_then(|total| {
            if total == 0 {
                Some(0.0)
            } else {
                Some((self.bytes_processed as f32 / total as f32) * 100.0)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn extraction_options_default() {
        let options = ExtractionOptions::default();
        assert_eq!(options.permission_strategy, PermissionStrategy::Standard);
        assert_eq!(options.hash_strategy, HashStrategy::None);
        assert_eq!(options.strip_components, 0);
        assert!(options.expected_total_bytes.is_none());
        assert!(options.on_progress.is_none());
    }

    #[test]
    fn extraction_options_builder_pattern() {
        let options = ExtractionOptions::default()
            .permission_strategy(PermissionStrategy::ReadOnly)
            .hash_strategy(HashStrategy::Sha256)
            .strip_components(1)
            .expected_total_bytes(1024);

        assert_eq!(options.permission_strategy, PermissionStrategy::ReadOnly);
        assert_eq!(options.hash_strategy, HashStrategy::Sha256);
        assert_eq!(options.strip_components, 1);
        assert_eq!(options.expected_total_bytes, Some(1024));
    }

    #[test]
    fn extraction_options_on_progress_callback() {
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = counter.clone();

        let options = ExtractionOptions::default().on_progress(Arc::new(move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        let progress = Progress {
            bytes_processed: 50,
            total_bytes: Some(100),
            percentage: Some(50.0),
            current_file: Some(PathBuf::from("test.txt")),
        };

        (options.on_progress.as_ref().unwrap())(progress);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn progress_percentage_with_total() {
        let progress = Progress {
            bytes_processed: 50,
            total_bytes: Some(100),
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), Some(50.0));
    }

    #[test]
    fn progress_percentage_zero_total() {
        let progress = Progress {
            bytes_processed: 0,
            total_bytes: Some(0),
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), Some(0.0));
    }

    #[test]
    fn progress_percentage_none_total() {
        let progress = Progress {
            bytes_processed: 50,
            total_bytes: None,
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), None);
    }

    #[test]
    fn progress_percentage_full() {
        let progress = Progress {
            bytes_processed: 100,
            total_bytes: Some(100),
            percentage: None,
            current_file: None,
        };
        assert_eq!(progress.percentage(), Some(100.0));
    }

    #[test]
    fn permission_strategy_variants() {
        use PermissionStrategy::*;
        let variants = [Standard, ReadOnly, Preserve, Owned];
        for (i, variant) in variants.iter().enumerate() {
            let mut options = ExtractionOptions::default();
            match i {
                0 => options = options.permission_strategy(Standard),
                1 => options = options.permission_strategy(ReadOnly),
                2 => options = options.permission_strategy(Preserve),
                3 => options = options.permission_strategy(Owned),
                _ => unreachable!(),
            }
            assert_eq!(options.permission_strategy, *variant);
        }
    }

    #[test]
    fn hash_strategy_variants() {
        use HashStrategy::*;
        let variants = [None, Sha256, Blake3];
        for (i, variant) in variants.iter().enumerate() {
            let mut options = ExtractionOptions::default();
            match i {
                0 => options = options.hash_strategy(None),
                1 => options = options.hash_strategy(Sha256),
                2 => options = options.hash_strategy(Blake3),
                _ => unreachable!(),
            }
            assert_eq!(options.hash_strategy, *variant);
        }
    }
}
