use std::fs::File;
use std::path::Path;

use pulith_archive::extract_from_reader;
use pulith_archive::options::ExtractOptions;

#[test]
fn extract_tar_gz() {
    let fixture_path = Path::new("tests/fixtures/test.tar.gz");
    let mut file = File::open(fixture_path).expect("Failed to open test.tar.gz");

    let temp_dir = tempfile::Builder::new()
        .prefix("pulith-test-tar-")
        .tempdir()
        .expect("Failed to create temp dir");

    let options = ExtractOptions::default();
    let result = extract_from_reader(&mut file, temp_dir.path(), &options);

    assert!(
        result.is_ok(),
        "Extraction of test.tar.gz failed: {:?}",
        result.err()
    );
    let report = result.unwrap();
    assert!(
        report.entry_count > 0,
        "No entries extracted from test.tar.gz"
    );

    println!("TAR.GZ extraction results:");
    println!("  Total entries: {}", report.entry_count);
    println!("  Total bytes: {}", report.total_bytes);
    for entry in &report.entries {
        println!("  - {} ({})", entry.original_path.display(), entry.size);
    }
}

#[test]
fn extract_zip() {
    let fixture_path = Path::new("tests/fixtures/test.zip");
    let mut file = File::open(fixture_path).expect("Failed to open test.zip");

    let temp_dir = tempfile::Builder::new()
        .prefix("pulith-test-zip-")
        .tempdir()
        .expect("Failed to create temp dir");

    let options = ExtractOptions::default();
    let result = extract_from_reader(&mut file, temp_dir.path(), &options);

    assert!(
        result.is_ok(),
        "Extraction of test.zip failed: {:?}",
        result.err()
    );
    let report = result.unwrap();
    assert!(report.entry_count > 0, "No entries extracted from test.zip");

    println!("ZIP extraction results:");
    println!("  Total entries: {}", report.entry_count);
    println!("  Total bytes: {}", report.total_bytes);
    for entry in &report.entries {
        println!("  - {} ({})", entry.original_path.display(), entry.size);
    }
}
