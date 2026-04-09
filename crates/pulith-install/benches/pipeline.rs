use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use futures_util::stream;
use pulith_archive::{ExtractOptions, extract_from_reader};
use pulith_fetch::{BoxStream, FetchOptions, Fetcher, HttpClient, MultiSourceFetcher};
use pulith_install::{InstallInput, InstallReady, InstallSpec, PlannedInstall};
use pulith_resource::{
    RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceLocator, ResourceSpec,
    ValidUrl,
};
use pulith_source::{SelectionStrategy, SourceSpec};
use pulith_state::StateReady;
use pulith_store::{StoreKey, StoreReady, StoreRoots};

#[derive(Debug, Clone)]
struct BenchHttpClient {
    size: usize,
    chunk_size: usize,
    delay: Duration,
}

impl BenchHttpClient {
    fn new(size: usize) -> Self {
        Self {
            size,
            chunk_size: 64 * 1024,
            delay: Duration::from_millis(1),
        }
    }
}

#[derive(Debug)]
struct BenchHttpError(String);

impl std::fmt::Display for BenchHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BenchHttpError {}

impl HttpClient for BenchHttpClient {
    type Error = BenchHttpError;

    async fn head(&self, _url: &str) -> std::result::Result<Option<u64>, Self::Error> {
        tokio::time::sleep(self.delay).await;
        Ok(Some(self.size as u64))
    }

    async fn stream(
        &self,
        _url: &str,
        _headers: &[(String, String)],
    ) -> std::result::Result<BoxStream<'static, std::result::Result<Bytes, Self::Error>>, Self::Error>
    {
        tokio::time::sleep(self.delay).await;

        let size = self.size;
        let chunk_size = self.chunk_size;
        let data = stream::unfold(size, move |remaining| async move {
            if remaining == 0 {
                None
            } else {
                let len = remaining.min(chunk_size);
                Some((Ok(Bytes::from(vec![0x5a; len])), remaining - len))
            }
        });

        Ok(Box::pin(data))
    }
}

struct FetchPipelineContext {
    _temp: tempfile::TempDir,
    multi: MultiSourceFetcher<BenchHttpClient>,
    planned: pulith_source::PlannedSources,
    store: StoreReady,
    ready: InstallReady,
    resource: pulith_resource::ResolvedResource,
    destination: PathBuf,
    install_root: PathBuf,
}

struct ArchivePipelineContext {
    _temp: tempfile::TempDir,
    archive_bytes: Vec<u8>,
    extract_root: PathBuf,
    store: StoreReady,
    ready: InstallReady,
    resource: pulith_resource::ResolvedResource,
    install_root: PathBuf,
}

fn resolved_fetch_resource(url: &str, version: &str) -> pulith_resource::ResolvedResource {
    RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("bench/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse(url).unwrap()),
        )
        .version(pulith_resource::VersionSelector::exact(version).unwrap()),
    )
    .resolve(
        ResolvedVersion::new(version).unwrap(),
        ResolvedLocator::Url(ValidUrl::parse(url).unwrap()),
        None,
    )
}

fn resolved_archive_resource(version: &str) -> pulith_resource::ResolvedResource {
    RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("bench/archive-runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        )
        .version(pulith_resource::VersionSelector::exact(version).unwrap()),
    )
    .resolve(
        ResolvedVersion::new(version).unwrap(),
        ResolvedLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        None,
    )
}

fn setup_fetch_pipeline(size: usize) -> FetchPipelineContext {
    let temp = tempfile::tempdir().unwrap();
    let resource = resolved_fetch_resource("https://example.com/runtime.bin", "1.0.0");
    let planned = SourceSpec::from_locator(&resource.spec().locator)
        .unwrap()
        .plan(SelectionStrategy::OrderedFallback);
    let fetcher = Fetcher::new(
        BenchHttpClient::new(size),
        temp.path().join("fetch-workspace"),
    );
    let multi = MultiSourceFetcher::new(Arc::new(fetcher));
    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state);

    FetchPipelineContext {
        _temp: temp,
        multi,
        planned,
        store,
        ready,
        resource,
        destination: PathBuf::from("downloads/runtime.bin"),
        install_root: PathBuf::from("installs/runtime"),
    }
}

fn setup_archive_pipeline(size: usize) -> ArchivePipelineContext {
    let temp = tempfile::tempdir().unwrap();
    let archive_bytes = build_zip_archive(size);
    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state);

    ArchivePipelineContext {
        _temp: temp,
        archive_bytes,
        extract_root: PathBuf::from("extract/runtime"),
        store,
        ready,
        resource: resolved_archive_resource("1.0.0"),
        install_root: PathBuf::from("installs/archive-runtime"),
    }
}

fn build_zip_archive(size: usize) -> Vec<u8> {
    use std::io::Write;

    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(&mut cursor);
    writer
        .start_file("bin/tool.bin", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(&vec![0x42; size]).unwrap();
    writer.finish().unwrap();
    cursor.into_inner()
}

fn bench_fetch_store_install(c: &mut Criterion) {
    let mut group = c.benchmark_group("fetch_store_install_pipeline");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [1024 * 1024, 8 * 1024 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || setup_fetch_pipeline(size),
                |context| {
                    runtime.block_on(async move {
                        let destination = context._temp.path().join(&context.destination);
                        let fetched = context
                            .multi
                            .fetch_planned_sources_with_receipt(
                                &context.planned,
                                &destination,
                                &FetchOptions::default(),
                            )
                            .await
                            .unwrap();

                        let stored = context
                            .store
                            .import_artifact(
                                &StoreKey::logical("bench-fetch-artifact").unwrap(),
                                &fetched.destination,
                            )
                            .unwrap();

                        let receipt = PlannedInstall::new(
                            context.ready,
                            InstallSpec::new(
                                context.resource,
                                InstallInput::StoredArtifact {
                                    artifact: stored,
                                    file_name: "runtime.bin".to_string(),
                                },
                                context._temp.path().join(&context.install_root),
                            ),
                        )
                        .stage()
                        .unwrap()
                        .commit()
                        .unwrap()
                        .finish();

                        black_box(receipt);
                    })
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_archive_extract_store_install(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_extract_store_install_pipeline");

    for size in [1024 * 1024, 8 * 1024 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || setup_archive_pipeline(size),
                |context| {
                    let extract_root = context._temp.path().join(&context.extract_root);
                    std::fs::create_dir_all(&extract_root).unwrap();

                    let report = extract_from_reader(
                        std::io::Cursor::new(context.archive_bytes),
                        &extract_root,
                        &ExtractOptions::default(),
                    )
                    .unwrap();

                    let extracted = context
                        .store
                        .register_extract_dir(
                            &StoreKey::logical("bench-archive-extract").unwrap(),
                            &extract_root,
                        )
                        .unwrap();

                    let receipt = PlannedInstall::new(
                        context.ready,
                        InstallSpec::new(
                            context.resource,
                            InstallInput::ExtractedArtifact(extracted),
                            context._temp.path().join(&context.install_root),
                        ),
                    )
                    .stage()
                    .unwrap()
                    .commit()
                    .unwrap()
                    .finish();

                    black_box(report.entry_count);
                    black_box(receipt);
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

criterion_group!(
    pipeline_benches,
    bench_fetch_store_install,
    bench_archive_extract_store_install
);
criterion_main!(pipeline_benches);
