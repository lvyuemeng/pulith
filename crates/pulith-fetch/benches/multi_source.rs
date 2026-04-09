use std::collections::BTreeMap;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use futures_util::stream;
use pulith_fetch::{
    BoxStream, DownloadSource, Fetcher, HttpClient, MultiSourceFetcher, MultiSourceOptions,
    SourceSelectionStrategy,
};

#[derive(Debug, Clone)]
struct SourceBehavior {
    head_delay: Duration,
    stream_delay: Duration,
    size: usize,
    fail_on_head: bool,
    fail_on_stream: bool,
}

impl SourceBehavior {
    fn success(size: usize, delay_ms: u64) -> Self {
        Self {
            head_delay: Duration::from_millis(delay_ms),
            stream_delay: Duration::from_millis(delay_ms),
            size,
            fail_on_head: false,
            fail_on_stream: false,
        }
    }

    fn head_failure(delay_ms: u64) -> Self {
        Self {
            head_delay: Duration::from_millis(delay_ms),
            stream_delay: Duration::from_millis(delay_ms),
            size: 0,
            fail_on_head: true,
            fail_on_stream: true,
        }
    }

    fn stream_failure(size: usize, delay_ms: u64) -> Self {
        Self {
            head_delay: Duration::from_millis(delay_ms),
            stream_delay: Duration::from_millis(delay_ms),
            size,
            fail_on_head: false,
            fail_on_stream: true,
        }
    }
}

#[derive(Debug, Clone)]
struct BenchHttpClient {
    behaviors: Arc<BTreeMap<String, SourceBehavior>>,
    chunk_size: usize,
}

impl BenchHttpClient {
    fn new(behaviors: BTreeMap<String, SourceBehavior>) -> Self {
        Self {
            behaviors: Arc::new(behaviors),
            chunk_size: 64 * 1024,
        }
    }

    fn behavior(&self, url: &str) -> Result<SourceBehavior, BenchHttpError> {
        self.behaviors
            .get(url)
            .cloned()
            .ok_or_else(|| BenchHttpError(format!("missing behavior for {url}")))
    }
}

#[derive(Debug, Clone)]
struct BenchHttpError(String);

impl std::fmt::Display for BenchHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BenchHttpError {}

impl HttpClient for BenchHttpClient {
    type Error = BenchHttpError;

    async fn head(&self, url: &str) -> std::result::Result<Option<u64>, Self::Error> {
        let behavior = self.behavior(url)?;
        tokio::time::sleep(behavior.head_delay).await;
        if behavior.fail_on_head {
            return Err(BenchHttpError(format!("head failure for {url}")));
        }
        Ok(Some(behavior.size as u64))
    }

    async fn stream(
        &self,
        url: &str,
        _headers: &[(String, String)],
    ) -> std::result::Result<BoxStream<'static, std::result::Result<Bytes, Self::Error>>, Self::Error>
    {
        let behavior = self.behavior(url)?;
        tokio::time::sleep(behavior.stream_delay).await;
        if behavior.fail_on_stream {
            return Err(BenchHttpError(format!("stream failure for {url}")));
        }

        let chunk_size = self.chunk_size;
        let fill = url.bytes().next().unwrap_or(b'x');
        let data = stream::unfold(behavior.size, move |remaining| async move {
            if remaining == 0 {
                None
            } else {
                let len = remaining.min(chunk_size);
                Some((Ok(Bytes::from(vec![fill; len])), remaining - len))
            }
        });

        Ok(Box::pin(data))
    }
}

struct MultiSourceBenchContext {
    _temp: tempfile::TempDir,
    fetcher: MultiSourceFetcher<BenchHttpClient>,
    sources: Vec<DownloadSource>,
    destination: PathBuf,
    options: MultiSourceOptions,
}

fn setup_priority_success_first(
    source_count: usize,
    payload_size: usize,
) -> MultiSourceBenchContext {
    let temp = tempfile::tempdir().unwrap();
    let mut behaviors = BTreeMap::new();
    let mut sources = Vec::with_capacity(source_count);

    for index in 0..source_count {
        let url = format!("https://priority-fast.example/{index}");
        let behavior = if index == 0 {
            SourceBehavior::success(payload_size, 1)
        } else {
            SourceBehavior::head_failure(1)
        };
        behaviors.insert(url.clone(), behavior);
        sources.push(DownloadSource::new(url).priority(index as u32));
    }

    build_context(temp, behaviors, sources, SourceSelectionStrategy::Priority)
}

fn setup_priority_fallback(source_count: usize, payload_size: usize) -> MultiSourceBenchContext {
    let temp = tempfile::tempdir().unwrap();
    let mut behaviors = BTreeMap::new();
    let mut sources = Vec::with_capacity(source_count);

    for index in 0..source_count {
        let url = format!("https://priority-fallback.example/{index}");
        let behavior = if index + 1 == source_count {
            SourceBehavior::success(payload_size, 2)
        } else if index % 2 == 0 {
            SourceBehavior::head_failure(1)
        } else {
            SourceBehavior::stream_failure(payload_size, 1)
        };
        behaviors.insert(url.clone(), behavior);
        sources.push(DownloadSource::new(url).priority(index as u32));
    }

    build_context(temp, behaviors, sources, SourceSelectionStrategy::Priority)
}

fn setup_race_one_success(source_count: usize, payload_size: usize) -> MultiSourceBenchContext {
    let temp = tempfile::tempdir().unwrap();
    let mut behaviors = BTreeMap::new();
    let mut sources = Vec::with_capacity(source_count);

    for index in 0..source_count {
        let url = format!("https://race.example/{index}");
        let behavior = if index + 1 == source_count {
            SourceBehavior::success(payload_size, 2)
        } else {
            SourceBehavior::head_failure(1)
        };
        behaviors.insert(url.clone(), behavior);
        sources.push(DownloadSource::new(url).priority(index as u32));
    }

    build_context(temp, behaviors, sources, SourceSelectionStrategy::RaceAll)
}

fn build_context(
    temp: tempfile::TempDir,
    behaviors: BTreeMap<String, SourceBehavior>,
    sources: Vec<DownloadSource>,
    strategy: SourceSelectionStrategy,
) -> MultiSourceBenchContext {
    let client = BenchHttpClient::new(behaviors);
    let fetcher = Arc::new(Fetcher::new(client, temp.path().join("workspace")));
    MultiSourceBenchContext {
        _temp: temp,
        fetcher: MultiSourceFetcher::new(fetcher),
        sources: sources.clone(),
        destination: PathBuf::from("downloads/artifact.bin"),
        options: MultiSourceOptions {
            sources,
            strategy,
            verify_consistency: false,
            per_source_timeout: None,
        },
    }
}

fn bench_priority_success_first(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_source_priority_success_first");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for source_count in [2usize, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(source_count),
            &source_count,
            |b, &source_count| {
                b.iter_batched(
                    || setup_priority_success_first(source_count, 512 * 1024),
                    |context| {
                        runtime.block_on(async move {
                            let destination = context._temp.path().join(&context.destination);
                            let receipt = context
                                .fetcher
                                .fetch_multi_source_with_receipt(
                                    context.sources,
                                    &destination,
                                    context.options,
                                )
                                .await
                                .unwrap();
                            black_box(receipt);
                        })
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_priority_fallback(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_source_priority_fallback");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for source_count in [3usize, 5, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(source_count),
            &source_count,
            |b, &source_count| {
                b.iter_batched(
                    || setup_priority_fallback(source_count, 512 * 1024),
                    |context| {
                        runtime.block_on(async move {
                            let destination = context._temp.path().join(&context.destination);
                            let receipt = context
                                .fetcher
                                .fetch_multi_source_with_receipt(
                                    context.sources,
                                    &destination,
                                    context.options,
                                )
                                .await
                                .unwrap();
                            black_box(receipt);
                        })
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_race_one_success(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_source_race_one_success");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for source_count in [3usize, 5, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(source_count),
            &source_count,
            |b, &source_count| {
                b.iter_batched(
                    || setup_race_one_success(source_count, 512 * 1024),
                    |context| {
                        runtime.block_on(async move {
                            let destination = context._temp.path().join(&context.destination);
                            let receipt = context
                                .fetcher
                                .fetch_multi_source_with_receipt(
                                    context.sources,
                                    &destination,
                                    context.options,
                                )
                                .await
                                .unwrap();
                            black_box(receipt);
                        })
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    multi_source_benches,
    bench_priority_success_first,
    bench_priority_fallback,
    bench_race_one_success
);
criterion_main!(multi_source_benches);
