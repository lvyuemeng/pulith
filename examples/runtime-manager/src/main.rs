use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use pulith_archive::{ExtractOptions, extract_from_reader};
use pulith_backend_example::managed_binary;
use pulith_fetch::{FetchOptions, Fetcher, MultiSourceFetcher, ReqwestClient};
use pulith_install::{
    ActivationReceipt, ActivationRequest, ActivationTarget, Activator, InstallInput, InstallReady,
    InstallSpec, PlannedInstall,
};
use pulith_resource::{
    RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceLocator, ResourceSpec,
    VersionSelector,
};
use pulith_source::SelectionStrategy;
use pulith_state::{
    InspectionSeverity, OwnershipReason, ResourceInspectionFinding, ResourceInspectionReport,
    StateReady, StoreRetentionPolicy,
};
use pulith_store::{StoreReady, StoreRoots};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.as_str() {
        "install-local-archive" => {
            let resource_id = args.next().ok_or_else(|| missing_arg("resource id"))?;
            let version = args.next().ok_or_else(|| missing_arg("version"))?;
            let archive_path =
                PathBuf::from(args.next().ok_or_else(|| missing_arg("archive path"))?);
            let workspace_root =
                PathBuf::from(args.next().ok_or_else(|| missing_arg("workspace root"))?);
            install_local_archive(&resource_id, &version, &archive_path, &workspace_root)
        }
        "install-local-file" => {
            let resource_id = args.next().ok_or_else(|| missing_arg("resource id"))?;
            let version = args.next().ok_or_else(|| missing_arg("version"))?;
            let file_path = PathBuf::from(args.next().ok_or_else(|| missing_arg("file path"))?);
            let workspace_root =
                PathBuf::from(args.next().ok_or_else(|| missing_arg("workspace root"))?);
            install_local_file(&resource_id, &version, &file_path, &workspace_root)
        }
        "inspect" => {
            let resource_id = args.next().ok_or_else(|| missing_arg("resource id"))?;
            let workspace_root =
                PathBuf::from(args.next().ok_or_else(|| missing_arg("workspace root"))?);
            inspect_resource(&resource_id, &workspace_root)
        }
        "repair-plan" => {
            let resource_id = args.next().ok_or_else(|| missing_arg("resource id"))?;
            let workspace_root =
                PathBuf::from(args.next().ok_or_else(|| missing_arg("workspace root"))?);
            repair_plan(&resource_id, &workspace_root)
        }
        "prune-plan" => {
            let workspace_root =
                PathBuf::from(args.next().ok_or_else(|| missing_arg("workspace root"))?);
            let policy = args
                .next()
                .unwrap_or_else(|| "installed-active".to_string());
            prune_plan(&workspace_root, &policy)
        }
        "reconcile" => {
            let resource_id = args.next().ok_or_else(|| missing_arg("resource id"))?;
            let workspace_root =
                PathBuf::from(args.next().ok_or_else(|| missing_arg("workspace root"))?);
            let policy = args
                .next()
                .unwrap_or_else(|| "installed-active".to_string());
            reconcile_resource(&resource_id, &workspace_root, &policy)
        }
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn install_local_archive(
    resource_id: &str,
    version: &str,
    archive_path: &Path,
    workspace_root: &Path,
) -> Result<()> {
    let resource = resolved_local_resource(resource_id, version, archive_path)?;

    let fetcher = Fetcher::new(
        ReqwestClient::new()?,
        workspace_root.join("fetch-workspace"),
    );
    let multi = MultiSourceFetcher::new(std::sync::Arc::new(fetcher));
    let destination = workspace_root.join("downloads").join(
        archive_path
            .file_name()
            .ok_or_else(|| missing_arg("archive path file name"))?,
    );
    let receipt = tokio::runtime::Runtime::new()?.block_on(async {
        multi
            .fetch_resolved_resource_with_receipt(
                &resource,
                SelectionStrategy::OrderedFallback,
                &destination,
                &FetchOptions::default(),
            )
            .await
    })?;

    let extract_root = workspace_root
        .join("extracted")
        .join(safe_name(resource_id));
    fs::create_dir_all(&extract_root)?;
    let fetched_file = fs::File::open(&receipt.destination)?;
    let report = extract_from_reader(fetched_file, &extract_root, &ExtractOptions::default())?;

    let store = init_store(workspace_root)?;
    let key = pulith_store::StoreKey::NamedVersion {
        id: ResourceId::parse(resource_id)?,
        version: ResolvedVersion::new(version)?,
    };
    let install_input = InstallInput::store_fetched_archive_extraction(
        &store,
        &key,
        &receipt,
        &extract_root,
        &report,
    )?;

    let state = init_state(workspace_root)?;
    let install_root = workspace_root.join("installs").join(safe_name(resource_id));
    let activation_target = workspace_root.join("active").join(safe_name(resource_id));
    let receipt = PlannedInstall::new(
        InstallReady::new(state),
        InstallSpec::new(resource, install_input, install_root)
            .replace_existing()
            .activation(ActivationTarget {
                path: activation_target,
            }),
    )
    .stage()?
    .commit()?
    .activate(&PointerFileActivator)?
    .finish();

    println!("installed: {}", receipt.install_root.display());
    if let Some(activation) = receipt.activation {
        println!("activated: {}", activation.target.display());
    }
    Ok(())
}

fn install_local_file(
    resource_id: &str,
    version: &str,
    file_path: &Path,
    workspace_root: &Path,
) -> Result<()> {
    let install_root = workspace_root.join("installs").join(safe_name(resource_id));
    let activation_target = workspace_root.join("active").join(safe_name(resource_id));
    let backend = managed_binary(
        resource_id,
        ResourceLocator::LocalPath(file_path.to_path_buf()),
        VersionSelector::exact(version)?,
        install_root,
        PathBuf::from("runtime.bin"),
    )?
    .activation_path(activation_target);

    let requested = backend.requested_resource();
    let resource = requested.clone().resolve(
        ResolvedVersion::new(version)?,
        ResolvedLocator::LocalPath(file_path.to_path_buf()),
        None,
    );

    let fetcher = Fetcher::new(
        ReqwestClient::new()?,
        workspace_root.join("fetch-workspace"),
    );
    let multi = MultiSourceFetcher::new(std::sync::Arc::new(fetcher));
    let destination = workspace_root.join("downloads").join(
        file_path
            .file_name()
            .ok_or_else(|| missing_arg("file path file name"))?,
    );
    let fetch_receipt = tokio::runtime::Runtime::new()?.block_on(async {
        multi
            .fetch_requested_resource_with_receipt(
                &requested,
                SelectionStrategy::OrderedFallback,
                &destination,
                &FetchOptions::default(),
            )
            .await
    })?;

    let state = init_state(workspace_root)?;
    let install = backend.install_spec_from_fetch_receipt(resource, &fetch_receipt);
    let receipt = PlannedInstall::new(InstallReady::new(state), install)
        .stage()?
        .commit()?
        .activate(&PointerFileActivator)?
        .finish();

    println!("installed: {}", receipt.install_root.display());
    if let Some(activation) = receipt.activation {
        println!("activated: {}", activation.target.display());
    }
    Ok(())
}

fn inspect_resource(resource_id: &str, workspace_root: &Path) -> Result<()> {
    let state = init_state(workspace_root)?;
    let store = init_store(workspace_root)?;
    let inspection = state.inspect_resource(&ResourceId::parse(resource_id)?, Some(&store))?;

    println!("resource: {}", inspection.snapshot.resource.as_string());
    println!(
        "summary: total={}, errors={}, warnings={}, info={}",
        inspection.summary.total_findings,
        inspection.summary.error_count,
        inspection.summary.warning_count,
        inspection.summary.info_count,
    );
    if inspection.is_clean() {
        println!("findings: none");
    } else {
        print_findings(&inspection);
    }
    Ok(())
}

fn repair_plan(resource_id: &str, workspace_root: &Path) -> Result<()> {
    let state = init_state(workspace_root)?;
    let store = init_store(workspace_root)?;
    let plan = state.plan_resource_state_repair(&ResourceId::parse(resource_id)?, Some(&store))?;

    println!(
        "resource: {}",
        plan.inspection.snapshot.resource.as_string()
    );
    if plan.actions.is_empty() {
        println!("repair actions: none");
    } else {
        for action in plan.actions {
            println!("repair action: {action:?}");
        }
    }
    Ok(())
}

fn prune_plan(workspace_root: &Path, policy: &str) -> Result<()> {
    let state = init_state(workspace_root)?;
    let store = init_store(workspace_root)?;
    let policy = parse_retention_policy(policy);
    let plan = state.plan_ownership_and_retention(&store, policy)?;

    println!("policy: {:?}", plan.retention.policy);
    if plan.ownership.entries.is_empty() {
        println!("ownership conflicts: none");
    } else {
        println!("ownership conflicts: {}", plan.ownership.entries.len());
        for entry in &plan.ownership.entries {
            let owners = entry
                .owners
                .iter()
                .map(|owner| owner.as_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "ownership [{}]: target {} owners {}",
                ownership_severity_label(entry.severity),
                entry.target.display(),
                owners
            );
            for reason in &entry.reasons {
                println!("  reason: {}", ownership_reason_message(reason));
            }
        }
    }

    println!("protected keys: {}", plan.retention.protected_keys.len());
    for protected in &plan.retention.protected_keys {
        println!("protect: {}", protected.key.relative_name());
        for reason in &protected.reasons {
            println!("  reason: {}", ownership_reason_message(reason));
        }
    }

    println!(
        "protected metadata: {}",
        plan.retention.protected_metadata.len()
    );
    println!(
        "removable metadata: {}",
        plan.retention.removable_metadata.len()
    );
    for removable in &plan.retention.removable_metadata {
        println!("remove metadata: {}", removable.record.key.relative_name());
        for reason in &removable.reasons {
            println!("  reason: {}", ownership_reason_message(reason));
        }
    }
    Ok(())
}

fn reconcile_resource(resource_id: &str, workspace_root: &Path, policy: &str) -> Result<()> {
    let state = init_state(workspace_root)?;
    let store = init_store(workspace_root)?;
    let policy = parse_retention_policy(policy);
    let resource_id = ResourceId::parse(resource_id)?;

    let inspection = state.inspect_resource(&resource_id, Some(&store))?;
    println!(
        "reconcile resource: {}",
        inspection.snapshot.resource.as_string()
    );
    println!(
        "before: total={}, errors={}, warnings={}, info={}",
        inspection.summary.total_findings,
        inspection.summary.error_count,
        inspection.summary.warning_count,
        inspection.summary.info_count,
    );

    let repair = state.plan_resource_state_repair(&resource_id, Some(&store))?;
    if repair.actions.is_empty() {
        println!("repair: no actions");
    } else {
        println!("repair: applying {} action(s)", repair.actions.len());
        for action in &repair.actions {
            println!("  apply: {action:?}");
        }
        state.apply_resource_state_repair(&repair)?;
    }

    let retention = state.plan_ownership_and_retention(&store, policy)?;
    let protected_keys = retention
        .retention
        .protected_keys
        .iter()
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();
    let prune_report = store.prune_missing_with_protection(&protected_keys)?;
    println!(
        "retention prune: removed_metadata={}, protected_metadata={}",
        prune_report.removed_metadata, prune_report.protected_metadata
    );

    let after = state.inspect_resource(&resource_id, Some(&store))?;
    println!(
        "after: total={}, errors={}, warnings={}, info={}",
        after.summary.total_findings,
        after.summary.error_count,
        after.summary.warning_count,
        after.summary.info_count,
    );
    Ok(())
}

fn parse_retention_policy(policy: &str) -> StoreRetentionPolicy {
    match policy {
        "all" => StoreRetentionPolicy::AllReferenced,
        "active" => StoreRetentionPolicy::ActiveOnly,
        _ => StoreRetentionPolicy::InstalledAndActive,
    }
}

fn init_store(workspace_root: &Path) -> Result<StoreReady> {
    Ok(StoreReady::initialize(StoreRoots::new(
        workspace_root.join("store").join("artifacts"),
        workspace_root.join("store").join("extracts"),
        workspace_root.join("store").join("metadata"),
    ))?)
}

fn init_state(workspace_root: &Path) -> Result<StateReady> {
    Ok(StateReady::initialize(
        workspace_root.join("state").join("state.json"),
    )?)
}

fn resolved_local_resource(
    resource_id: &str,
    version: &str,
    archive_path: &Path,
) -> Result<pulith_resource::ResolvedResource> {
    Ok(RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse(resource_id)?,
            ResourceLocator::LocalPath(archive_path.to_path_buf()),
        )
        .version(VersionSelector::exact(version)?),
    )
    .resolve(
        ResolvedVersion::new(version)?,
        ResolvedLocator::LocalPath(archive_path.to_path_buf()),
        None,
    ))
}

fn safe_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn print_usage() {
    println!("runtime-manager-example");
    println!();
    println!("commands:");
    println!("  install-local-archive <resource-id> <version> <archive-path> <workspace-root>");
    println!("  install-local-file <resource-id> <version> <file-path> <workspace-root>");
    println!("  inspect <resource-id> <workspace-root>");
    println!("  repair-plan <resource-id> <workspace-root>");
    println!("  prune-plan <workspace-root> [all|installed-active|active]");
    println!("  reconcile <resource-id> <workspace-root> [all|installed-active|active]");
}

fn print_findings(report: &ResourceInspectionReport) {
    for severity in [
        InspectionSeverity::Error,
        InspectionSeverity::Warning,
        InspectionSeverity::Info,
    ] {
        for finding in report
            .findings
            .iter()
            .filter(|finding| finding.severity() == severity)
        {
            println!(
                "finding [{}:{}]: {}",
                severity_label(severity),
                finding.summary_label(),
                finding_message(finding)
            );
        }
    }
}

fn severity_label(severity: InspectionSeverity) -> &'static str {
    match severity {
        InspectionSeverity::Info => "info",
        InspectionSeverity::Warning => "warning",
        InspectionSeverity::Error => "error",
    }
}

fn finding_message(finding: &ResourceInspectionFinding) -> String {
    match finding {
        ResourceInspectionFinding::MissingResourceRecord { resource } => {
            format!("resource record missing for {}", resource.as_string())
        }
        ResourceInspectionFinding::MissingInstallPath { resource, path } => format!(
            "install path missing for {} at {}",
            resource.as_string(),
            path.display()
        ),
        ResourceInspectionFinding::MissingActivationTarget { resource, target } => format!(
            "activation target missing for {} at {}",
            resource.as_string(),
            target.display()
        ),
        ResourceInspectionFinding::ActivationTargetConflict {
            resource,
            target,
            conflicting_owners,
        } => format!(
            "activation target {} for {} is also owned by {}",
            target.display(),
            resource.as_string(),
            conflicting_owners
                .iter()
                .map(|owner| owner.as_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ResourceInspectionFinding::MissingStoreEntry { resource, key } => format!(
            "store entry missing for {} key {}",
            resource.as_string(),
            key.relative_name()
        ),
        ResourceInspectionFinding::MissingStoreMetadata { resource, key } => format!(
            "store metadata missing for {} key {}",
            resource.as_string(),
            key.relative_name()
        ),
    }
}

fn ownership_severity_label(severity: pulith_state::OwnershipSeverity) -> &'static str {
    match severity {
        pulith_state::OwnershipSeverity::Info => "info",
        pulith_state::OwnershipSeverity::Warning => "warning",
        pulith_state::OwnershipSeverity::Error => "error",
    }
}

fn ownership_reason_message(reason: &OwnershipReason) -> String {
    match reason {
        OwnershipReason::SharedActivationTarget { target, owners } => format!(
            "shared activation target {} owned by {}",
            target.display(),
            owners
                .iter()
                .map(|owner| owner.as_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        OwnershipReason::StateStoreReference {
            key,
            owner,
            lifecycle,
        } => format!(
            "state references key {} via {} ({lifecycle:?})",
            key.relative_name(),
            owner.as_string()
        ),
        OwnershipReason::RetentionPolicyExcludesLifecycle {
            policy,
            resource,
            lifecycle,
        } => format!(
            "policy {policy:?} excludes {} at lifecycle {lifecycle:?}",
            resource.as_string()
        ),
        OwnershipReason::UnreferencedStoreMetadata { key } => {
            format!("no state references key {}", key.relative_name())
        }
    }
}

#[derive(Debug, Default)]
struct PointerFileActivator;

impl Activator for PointerFileActivator {
    fn activate(&self, request: &ActivationRequest) -> pulith_install::Result<ActivationReceipt> {
        if let Some(parent) = request.target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &request.target,
            request.installed_path.to_string_lossy().as_bytes(),
        )
        .map_err(pulith_install::InstallError::Io)?;
        Ok(ActivationReceipt {
            target: request.target.clone(),
            installed_path: request.installed_path.clone(),
        })
    }
}

fn missing_arg(name: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, format!("missing {name}"))
}
