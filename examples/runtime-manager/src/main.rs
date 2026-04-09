use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use pulith_archive::{ExtractOptions, extract_from_reader};
use pulith_fetch::{FetchOptions, Fetcher, MultiSourceFetcher, ReqwestClient};
use pulith_install::{
    ActivationReceipt, ActivationRequest, ActivationTarget, Activator, InstallInput, InstallReady,
    InstallSpec, PlannedInstall,
};
use pulith_resource::{
    RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceLocator, ResourceSpec,
    VersionSelector,
};
use pulith_source::{PlannedSources, SelectionStrategy};
use pulith_state::{StateReady, StoreRetentionPolicy};
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
    let planned =
        PlannedSources::from_resolved_resource(&resource, SelectionStrategy::OrderedFallback)?;

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
            .fetch_planned_sources_with_receipt(&planned, &destination, &FetchOptions::default())
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

fn inspect_resource(resource_id: &str, workspace_root: &Path) -> Result<()> {
    let state = init_state(workspace_root)?;
    let store = init_store(workspace_root)?;
    let inspection = state.inspect_resource(&ResourceId::parse(resource_id)?, Some(&store))?;

    println!("resource: {}", inspection.snapshot.resource.as_string());
    if inspection.issues.is_empty() {
        println!("issues: none");
    } else {
        for issue in inspection.issues {
            println!("issue: {issue:?}");
        }
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
    let policy = match policy {
        "all" => StoreRetentionPolicy::AllReferenced,
        "active" => StoreRetentionPolicy::ActiveOnly,
        _ => StoreRetentionPolicy::InstalledAndActive,
    };
    let plan = state.plan_store_metadata_retention(&store, policy)?;

    println!("policy: {:?}", plan.policy);
    println!("protected keys: {}", plan.protected_keys.len());
    for key in &plan.protected_keys {
        println!("protect: {}", key.relative_name());
    }
    println!("protected metadata: {}", plan.protected_metadata.len());
    println!("removable metadata: {}", plan.removable_metadata.len());
    Ok(())
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
    println!("  inspect <resource-id> <workspace-root>");
    println!("  repair-plan <resource-id> <workspace-root>");
    println!("  prune-plan <workspace-root> [all|installed-active|active]");
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
