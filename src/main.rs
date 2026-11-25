mod cli;
mod discover;
mod error;
mod evidence;
mod graph;
mod intelligence;
mod investigate;
mod observe;
mod pdef;
mod protocol;
mod scope;
mod snapshot;
mod storage;
mod system;
mod twin;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::{
    Cli, Commands, DiscoverCommand, EvidenceCommand, GraphCommand, GraphExportFormat,
    IntelligenceCommand, InvestigateCommand, LogLevel, ObserveCommand, OutputFormat, PdefCommand,
    ProtocolCommand, ReportCommand, ScopeCommand, SnapshotCommand, SystemCommand, TwinCommand,
};
use error::{AetherError, ExitCode};
use graph::{NetworkGraph, NodeKind};
use scope::ScopeFile;
use storage::Storage;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    init_tracing(&cli);

    let result = run(cli).await;
    match result {
        Ok(()) => std::process::exit(ExitCode::Success.as_i32()),
        Err(err) => {
            tracing::error!("{err}");
            eprintln!("error: {err}");
            std::process::exit(err.exit_code().as_i32());
        }
    }
}

fn init_tracing(cli: &Cli) {
    let level = if cli.quiet {
        "error"
    } else if let Some(lvl) = cli.log_level {
        match lvl {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    } else {
        match cli.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };

    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(!cli.no_color)
        .try_init();
}

async fn run(cli: Cli) -> Result<(), AetherError> {
    match cli.command {
        Commands::System(args) => match args.command {
            SystemCommand::Doctor { format } => {
                system::run_doctor(&cli.data_dir, format, cli.json)
            }
            SystemCommand::Info => system::run_info(cli.json),
            SystemCommand::Completion { shell } => {
                system::run_completion(shell);
                Ok(())
            }
        },
        Commands::Scope(args) => handle_scope(args.command, cli.json),
        Commands::Graph(args) => handle_graph(args.command, &cli.data_dir, cli.json),
        Commands::Discover(args) => handle_discover(args.command, &cli.data_dir, cli.json),
        Commands::Observe(args) => handle_observe(args.command, cli.json),
        Commands::Snapshot(args) => handle_snapshot(args.command, &cli.data_dir, cli.json),
        Commands::Twin(args) => handle_twin(args.command, &cli.data_dir, cli.json),
        Commands::Pdef(args) => handle_pdef(args.command, cli.json),
        Commands::Protocol(args) => handle_protocol(args.command, cli.json),
        Commands::Evidence(args) => handle_evidence(args.command, &cli.data_dir, cli.json),
        Commands::Intelligence(args) => handle_intelligence(args.command, &cli.data_dir, cli.json),
        Commands::Investigate(args) => handle_investigate(args.command, &cli.data_dir, cli.json),
        Commands::Report(args) => handle_report(args.command, &cli.data_dir, cli.json),
    }
}

fn handle_scope(cmd: ScopeCommand, json_flag: bool) -> Result<(), AetherError> {
    match cmd {
        ScopeCommand::Init { output } => {
            let scope = ScopeFile::default_template();
            scope.save(&output)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "created",
                        "file": output.display().to_string()
                    }))?
                );
            } else {
                println!("Created scope template at {}", output.display());
            }
            Ok(())
        }
        ScopeCommand::Validate { file } => {
            let scope = ScopeFile::load(&file)?;
            scope.validate()?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "valid": true,
                        "file": file.display().to_string(),
                        "owner": scope.owner,
                        "expires_at": scope.expires_at
                    }))?
                );
            } else {
                println!("Scope {} is valid (owner: {}, expires: {})", file.display(), scope.owner, scope.expires_at);
            }
            Ok(())
        }
        ScopeCommand::Show { file } => {
            let scope = ScopeFile::load(&file)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&scope)?);
            } else {
                println!("{}", serde_yaml::to_string(&scope)?);
            }
            Ok(())
        }
        ScopeCommand::AddTarget { file, target } => {
            let mut scope = ScopeFile::load(&file)?;
            if !scope.permitted_targets.contains(&target) {
                scope.permitted_targets.push(target.clone());
                scope.save(&file)?;
            }
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "updated",
                        "added_target": target
                    }))?
                );
            } else {
                println!("Added target '{}' to {}", target, file.display());
            }
            Ok(())
        }
        ScopeCommand::List { file } => {
            let scope = ScopeFile::load(&file)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "permitted_targets": scope.permitted_targets,
                        "excluded_targets": scope.excluded_targets
                    }))?
                );
            } else {
                println!("Permitted targets:");
                for t in &scope.permitted_targets {
                    println!("  + {t}");
                }
                if !scope.excluded_targets.is_empty() {
                    println!("Excluded targets:");
                    for t in &scope.excluded_targets {
                        println!("  - {t}");
                    }
                }
            }
            Ok(())
        }
        ScopeCommand::Explain { target, file } => {
            let scope = ScopeFile::load(&file)?;
            let permitted = scope.is_target_permitted(&target)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "target": target,
                        "permitted": permitted
                    }))?
                );
            } else {
                println!(
                    "Target '{}' is {} by {}",
                    target,
                    if permitted { "PERMITTED" } else { "DENIED" },
                    file.display()
                );
            }
            Ok(())
        }
    }
}

fn handle_graph(
    cmd: GraphCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    let storage = Storage::open(data_dir)?;
    match cmd {
        GraphCommand::Stats => {
            let g = NetworkGraph::load(&storage)?;
            let stats = g.stats();
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!(
                    "Graph Stats: {} nodes ({} devices, {} services), {} edges",
                    stats.total_nodes, stats.devices, stats.services, stats.total_edges
                );
            }
            Ok(())
        }
        GraphCommand::Devices => {
            let g = NetworkGraph::load(&storage)?;
            let devices: Vec<_> = g.nodes.iter().filter(|n| n.kind == NodeKind::Device).collect();
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&devices)?);
            } else {
                for d in devices {
                    println!("{} ({}, trust={})", d.id, d.label, d.trust_level);
                }
            }
            Ok(())
        }
        GraphCommand::Services => {
            let g = NetworkGraph::load(&storage)?;
            let services: Vec<_> = g.nodes.iter().filter(|n| n.kind == NodeKind::Service).collect();
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&services)?);
            } else {
                for s in services {
                    println!("{} ({})", s.id, s.label);
                }
            }
            Ok(())
        }
        GraphCommand::Relationships => {
            let g = NetworkGraph::load(&storage)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&g.edges)?);
            } else {
                for e in &g.edges {
                    println!(
                        "{} -[{}]-> {} (confidence={:.2})",
                        e.source_id,
                        e.kind.as_str(),
                        e.target_id,
                        e.confidence
                    );
                }
            }
            Ok(())
        }
        GraphCommand::Query { query } => {
            let g = NetworkGraph::load(&storage)?;
            let nodes = g.query_nodes(&query)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&nodes)?);
            } else {
                for n in nodes {
                    println!("{} | {:?} | {} (trust={})", n.id, n.kind, n.label, n.trust_level);
                }
            }
            Ok(())
        }
        GraphCommand::Path {
            from,
            to,
            max_hops,
            explain,
        } => {
            let g = NetworkGraph::load(&storage)?;
            let path = g.shortest_path(&from, &to, max_hops);
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "from": from,
                        "to": to,
                        "max_hops": max_hops,
                        "path": path
                    }))?
                );
            } else if let Some(p) = path {
                println!("Path: {}", p.join(" -> "));
                if explain {
                    println!("Hops: {}", p.len().saturating_sub(1));
                }
            } else {
                println!("No path found from '{from}' to '{to}' within {max_hops} hops.");
            }
            Ok(())
        }
        GraphCommand::Neighborhood { node, depth } => {
            let g = NetworkGraph::load(&storage)?;
            let neighbors = g.neighborhood(&node, depth);
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&neighbors)?);
            } else {
                println!("Neighborhood of '{node}' (depth={depth}): {}", neighbors.join(", "));
            }
            Ok(())
        }
        GraphCommand::Export { format, output } => {
            let g = NetworkGraph::load(&storage)?;
            match format {
                GraphExportFormat::Json => g.export_json(&output)?,
                GraphExportFormat::Csv => g.export_csv(&output)?,
                GraphExportFormat::Gexf => g.export_gexf(&output)?,
            }
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "exported",
                        "output": output.display().to_string()
                    }))?
                );
            } else {
                println!("Exported graph to {}", output.display());
            }
            Ok(())
        }
        GraphCommand::Validate => {
            let version = storage.schema_version()?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "valid": true,
                        "schema_version": version
                    }))?
                );
            } else {
                println!("Graph database valid (schema_version={version})");
            }
            Ok(())
        }
        GraphCommand::Migrate { dry_run } => {
            let applied = storage.migrate(dry_run)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "dry_run": dry_run,
                        "migrations": applied
                    }))?
                );
            } else {
                println!("Migrations (dry_run={dry_run}): {:?}", applied);
            }
            Ok(())
        }
    }
}

fn handle_snapshot(
    cmd: SnapshotCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    let storage = Storage::open(data_dir)?;
    match cmd {
        SnapshotCommand::Create { name, scope } => {
            let rec = snapshot::create_snapshot(&storage, &name, &scope)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&rec)?);
            } else {
                println!(
                    "Created snapshot '{}' (id={}, nodes={}, edges={})",
                    rec.name,
                    rec.id,
                    rec.payload.nodes.len(),
                    rec.payload.edges.len()
                );
            }
            Ok(())
        }
        SnapshotCommand::List => {
            let list = snapshot::list_snapshots(&storage)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                for s in list {
                    println!(
                        "{} | {} | nodes={} edges={} | {}",
                        s.name, s.id, s.node_count, s.edge_count, s.created_at
                    );
                }
            }
            Ok(())
        }
        SnapshotCommand::Show { id } => {
            let rec = snapshot::load_snapshot(&storage, &id)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&rec)?);
            } else {
                println!(
                    "Snapshot '{}' (id={}) created {}\n  nodes: {}\n  edges: {}",
                    rec.name,
                    rec.id,
                    rec.created_at,
                    rec.payload.nodes.len(),
                    rec.payload.edges.len()
                );
            }
            Ok(())
        }
        SnapshotCommand::Diff {
            first,
            second,
            format,
        } => {
            let diff = snapshot::diff_snapshots(&storage, &first, &second)?;
            if json_flag || matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&diff)?);
            } else {
                println!("Diff: {} -> {}", diff.first, diff.second);
                println!("  + nodes: {:?}", diff.added_nodes);
                println!("  - nodes: {:?}", diff.removed_nodes);
                println!("  + edges: {:?}", diff.added_edges);
                println!("  - edges: {:?}", diff.removed_edges);
            }
            Ok(())
        }
        SnapshotCommand::Prune {
            older_than,
            dry_run,
        } => {
            let count = snapshot::prune_snapshots(&storage, older_than, dry_run)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "dry_run": dry_run,
                        "affected_snapshots": count
                    }))?
                );
            } else {
                println!("Prune (dry_run={dry_run}): {count} snapshot(s) matched");
            }
            Ok(())
        }
        SnapshotCommand::RestoreModel { id, output } => {
            let rec = snapshot::load_snapshot(&storage, &id)?;
            let model = twin::TwinModel::from_snapshot(&rec.name, rec.payload);
            model.save(&output)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "restored_model",
                        "output": output.display().to_string()
                    }))?
                );
            } else {
                println!("Restored offline model copy to {}", output.display());
            }
            Ok(())
        }
    }
}

fn handle_twin(
    cmd: TwinCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    match cmd {
        TwinCommand::Build { snapshot, output } => {
            let storage = Storage::open(data_dir)?;
            let rec = snapshot::load_snapshot(&storage, &snapshot)?;
            let model = twin::TwinModel::from_snapshot(&rec.name, rec.payload);
            model.save(&output)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "built",
                        "output": output.display().to_string()
                    }))?
                );
            } else {
                println!("Built digital twin model at {}", output.display());
            }
            Ok(())
        }
        TwinCommand::Inspect { model } => {
            let m = twin::TwinModel::load(&model)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&m)?);
            } else {
                println!(
                    "TwinModel (source={}): {} nodes, {} edges",
                    m.source_snapshot,
                    m.nodes.len(),
                    m.edges.len()
                );
            }
            Ok(())
        }
        TwinCommand::Simulate {
            scenario,
            model,
            output,
        } => {
            let m = twin::TwinModel::load(&model)?;
            let s = twin::TwinScenario::load(&scenario)?;
            let res = twin::simulate(&m, &s)?;
            if let Some(out_path) = output {
                std::fs::write(&out_path, serde_json::to_string_pretty(&res)?)?;
            }
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!(
                    "Simulation '{}': nodes {}->{}, edges {}->{}",
                    res.scenario,
                    res.baseline_nodes,
                    res.simulated_nodes,
                    res.baseline_edges,
                    res.simulated_edges
                );
                for note in &res.notes {
                    println!("  - {note}");
                }
            }
            Ok(())
        }
        TwinCommand::ScenarioValidate { file } => {
            let s = twin::TwinScenario::load(&file)?;
            s.validate()?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "valid": true,
                        "scenario": s.name
                    }))?
                );
            } else {
                println!("Scenario '{}' is valid ({} actions)", s.name, s.actions.len());
            }
            Ok(())
        }
    }
}

fn handle_pdef(cmd: PdefCommand, json_flag: bool) -> Result<(), AetherError> {
    match cmd {
        PdefCommand::Init { protocol, output } => {
            let pdef = match protocol.as_str() {
                "smtp" => pdef::ProtocolDefinition::default_smtp(),
                other => {
                    return Err(AetherError::Validation(format!(
                        "unsupported PDEF template '{other}' (supported: smtp)"
                    )))
                }
            };
            pdef.save(&output)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "created",
                        "protocol": pdef.name,
                        "file": output.display().to_string()
                    }))?
                );
            } else {
                println!("Created PDEF template for '{}' at {}", pdef.name, output.display());
            }
            Ok(())
        }
        PdefCommand::Validate { file } => {
            let p = pdef::ProtocolDefinition::load(&file)?;
            p.validate()?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "valid": true,
                        "protocol": p.name,
                        "states": p.states.len(),
                        "transitions": p.transitions.len()
                    }))?
                );
            } else {
                println!(
                    "PDEF '{}' is valid ({} states, {} transitions)",
                    p.name,
                    p.states.len(),
                    p.transitions.len()
                );
            }
            Ok(())
        }
        PdefCommand::Lint { file } => {
            let p = pdef::ProtocolDefinition::load(&file)?;
            p.validate()?;
            let warnings = p.lint();
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "protocol": p.name,
                        "warnings": warnings
                    }))?
                );
            } else if warnings.is_empty() {
                println!("PDEF '{}' passed lint with 0 warnings", p.name);
            } else {
                println!("PDEF '{}' lint warnings:", p.name);
                for w in &warnings {
                    println!("  - {w}");
                }
            }
            Ok(())
        }
    }
}

fn handle_protocol(cmd: ProtocolCommand, json_flag: bool) -> Result<(), AetherError> {
    match cmd {
        ProtocolCommand::Test {
            target,
            definition,
            suite,
            max_cases,
            dry_run,
            i_have_permission,
        } => {
            let p = pdef::ProtocolDefinition::load(&definition)?;
            let report = protocol::run_protocol_test(
                &target,
                &p,
                &suite,
                max_cases,
                dry_run,
                i_have_permission,
            )?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.dry_run {
                println!(
                    "[DRY-RUN] Would run protocol '{}' suite '{}' against '{}'",
                    report.protocol, report.suite, report.target
                );
            } else {
                println!(
                    "Protocol Test '{}' on '{}': {}/{} passed",
                    report.protocol, report.target, report.cases_passed, report.cases_run
                );
                for r in &report.results {
                    println!(
                        "  [{}] {} --({})--> {} (observed: {})",
                        if r.passed { "PASS" } else { "FAIL" },
                        r.from_state,
                        r.input,
                        r.to_state,
                        r.observed_response
                    );
                }
            }
            Ok(())
        }
        ProtocolCommand::Fingerprint {
            target,
            profile,
            output,
        } => {
            let dna = protocol::fingerprint_protocol(&target, &profile)?;
            if let Some(out_path) = output {
                std::fs::write(&out_path, serde_json::to_string_pretty(&dna)?)?;
            }
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&dna)?);
            } else {
                println!(
                    "Protocol DNA '{}' ({}): banner={}, {} transition signatures",
                    dna.target,
                    dna.profile,
                    dna.banner_prefix,
                    dna.transition_signatures.len()
                );
            }
            Ok(())
        }
        ProtocolCommand::Compare { first, second } => {
            let cmp = protocol::compare_dna(&first, &second)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&cmp)?);
            } else {
                println!(
                    "DNA Compare ({} vs {}): identical={}",
                    cmp.first_target, cmp.second_target, cmp.identical
                );
                if !cmp.added_signatures.is_empty() {
                    println!("  + {:?}", cmp.added_signatures);
                }
                if !cmp.removed_signatures.is_empty() {
                    println!("  - {:?}", cmp.removed_signatures);
                }
            }
            Ok(())
        }
    }
}

fn handle_evidence(
    cmd: EvidenceCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    match cmd {
        EvidenceCommand::List { case } => {
            let list = evidence::EvidenceRecord::list(data_dir, case.as_deref())?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                for e in list {
                    println!("{} | {} | {}", e.id, e.kind, e.summary);
                }
            }
            Ok(())
        }
        EvidenceCommand::Show { id } => {
            let rec = evidence::EvidenceRecord::load(data_dir, &id)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&rec)?);
            } else {
                println!("Evidence {} ({}): {}", rec.id, rec.kind, rec.summary);
            }
            Ok(())
        }
        EvidenceCommand::Verify { id } => {
            let rec = evidence::EvidenceRecord::load(data_dir, &id)?;
            let valid = rec.verify()?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "id": rec.id,
                        "sha256": rec.sha256,
                        "valid": valid
                    }))?
                );
            } else {
                println!(
                    "Evidence {} integrity: {} (sha256={})",
                    rec.id,
                    if valid { "VALID" } else { "TAMPERED" },
                    rec.sha256
                );
            }
            if !valid {
                return Err(AetherError::Validation(format!(
                    "evidence '{}' failed SHA-256 integrity verification",
                    rec.id
                )));
            }
            Ok(())
        }
        EvidenceCommand::Redact { input, output } => {
            let content = std::fs::read_to_string(&input)?;
            let redacted = evidence::redact_text(&content);
            std::fs::write(&output, redacted)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "redacted",
                        "output": output.display().to_string()
                    }))?
                );
            } else {
                println!("Redacted output written to {}", output.display());
            }
            Ok(())
        }
    }
}

fn handle_intelligence(
    cmd: IntelligenceCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    let storage = Storage::open(data_dir)?;
    match cmd {
        IntelligenceCommand::Analyze => {
            let g = NetworkGraph::load(&storage)?;
            let findings = intelligence::analyze_graph(&g);
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&findings)?);
            } else if findings.is_empty() {
                println!("No findings.");
            } else {
                for f in &findings {
                    println!(
                        "[{}] {} (score={}): {}",
                        f.id, f.title, f.score, f.explanation
                    );
                }
            }
            Ok(())
        }
    }
}

fn handle_investigate(
    cmd: InvestigateCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    match cmd {
        InvestigateCommand::List => {
            let cases = investigate::InvestigationCase::list(data_dir)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&cases)?);
            } else {
                for c in cases {
                    println!("{} | finding={} | {:?}", c.id, c.finding_id, c.status);
                }
            }
            Ok(())
        }
        InvestigateCommand::Show { case } => {
            let c = investigate::InvestigationCase::load(data_dir, &case)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&c)?);
            } else {
                println!("Case {} (finding={}): {:?}", c.id, c.finding_id, c.status);
                for ev in &c.timeline {
                    println!("  - {} {}", ev.timestamp, ev.description);
                }
            }
            Ok(())
        }
        InvestigateCommand::Run { finding } => {
            let c = investigate::InvestigationCase::create(data_dir, &finding)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&c)?);
            } else {
                println!("Opened investigation case {} for finding '{}'", c.id, c.finding_id);
            }
            Ok(())
        }
        InvestigateCommand::Close { case, reason } => {
            let mut c = investigate::InvestigationCase::load(data_dir, &case)?;
            c.close(data_dir, &reason)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&c)?);
            } else {
                println!("Closed case {} ({})", c.id, reason);
            }
            Ok(())
        }
    }
}

fn handle_report(
    cmd: ReportCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    match cmd {
        ReportCommand::Build {
            case,
            format,
            output,
        } => {
            let c = investigate::InvestigationCase::load(data_dir, &case)?;
            let out_path = investigate::export_case_report(&c, &format, &output)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "built",
                        "case": c.id,
                        "output": out_path.display().to_string()
                    }))?
                );
            } else {
                println!("Built report at {}", out_path.display());
            }
            Ok(())
        }
    }
}

fn handle_discover(
    cmd: DiscoverCommand,
    data_dir: &std::path::Path,
    json_flag: bool,
) -> Result<(), AetherError> {
    match cmd {
        DiscoverCommand::Import { input, format } => {
            let storage = Storage::open(data_dir)?;
            let summary = discover::import_inventory(&storage, &input, &format)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!(
                    "Imported {} nodes and {} edges from {}",
                    summary.nodes_imported,
                    summary.edges_imported,
                    input.display()
                );
            }
            Ok(())
        }
        DiscoverCommand::Inventory {
            scope,
            mode,
            dry_run,
            i_have_permission,
        } => {
            let validated_scope =
                discover::validate_discovery_request(&scope, &mode, i_have_permission)?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "mode": mode,
                        "dry_run": dry_run,
                        "targets": validated_scope.permitted_targets
                    }))?
                );
            } else {
                println!(
                    "Discovery Inventory (mode={mode}, dry_run={dry_run}): {} permitted target(s)",
                    validated_scope.permitted_targets.len()
                );
            }
            Ok(())
        }
    }
}

fn handle_observe(cmd: ObserveCommand, json_flag: bool) -> Result<(), AetherError> {
    match cmd {
        ObserveCommand::Replay { input, speed } => {
            let summary = observe::replay_capture(&input, speed)?;
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!(
                    "Replayed {} event(s) from {} (speed={}x)",
                    summary.events_replayed, summary.input, summary.speed
                );
                for ev in &summary.events {
                    println!("  {} [{}] {} -> {}: {}", ev.timestamp, ev.protocol, ev.src, ev.dst, ev.summary);
                }
            }
            Ok(())
        }
    }
}
