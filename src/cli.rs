use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

#[derive(Debug, Parser)]
#[command(
    name = "aetherlattice",
    version,
    about = "Context-Aware Network State & Logic Enumeration Framework",
    long_about = "AetherLattice is a CLI-first network intelligence and protocol research framework."
)]
pub struct Cli {
    /// Path to configuration file
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Path to persistent data directory
    #[arg(long, global = true, value_name = "DIR", default_value = ".aetherlattice")]
    pub data_dir: PathBuf,

    /// Explicit log level
    #[arg(long, global = true, value_enum)]
    pub log_level: Option<LogLevel>,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-essential diagnostic output
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Emit machine-readable JSON on stdout
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable ANSI color output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// System diagnostics, metadata, and shell completions
    System(SystemArgs),
    /// Authorization scope management and validation
    Scope(ScopeArgs),
    /// Persistent network intelligence graph operations
    Graph(GraphArgs),
    /// Network discovery and inventory import
    Discover(DiscoverArgs),
    /// Offline capture replay and observation
    Observe(ObserveArgs),
    /// Historical snapshots and temporal diffing
    Snapshot(SnapshotArgs),
    /// Offline network digital twin modelling and simulation
    Twin(TwinArgs),
    /// Protocol Definition File (PDEF) management
    Pdef(PdefArgs),
    /// Protocol FSM inspection and safe state-transition testing
    Protocol(ProtocolArgs),
    /// Evidence store management and redaction
    Evidence(EvidenceArgs),
    /// Explainable network intelligence analysis
    Intelligence(IntelligenceArgs),
    /// Evidence-linked investigation case management
    Investigate(InvestigateArgs),
    /// Build investigation and assessment reports
    Report(ReportArgs),
}

#[derive(Debug, Args)]
pub struct SystemArgs {
    #[command(subcommand)]
    pub command: SystemCommand,
}

#[derive(Debug, Subcommand)]
pub enum SystemCommand {
    /// Check permissions, storage, dependencies, and platform features
    Doctor {
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },
    /// Display build and platform information
    Info,
    /// Generate shell completion scripts
    Completion {
        #[arg(long, value_enum)]
        shell: ShellType,
    },
}

#[derive(Debug, Args)]
pub struct ScopeArgs {
    #[command(subcommand)]
    pub command: ScopeCommand,
}

#[derive(Debug, Subcommand)]
pub enum ScopeCommand {
    /// Create a new authorization scope file template
    Init {
        #[arg(long, value_name = "FILE", default_value = "./scope.yaml")]
        output: PathBuf,
    },
    /// Validate an authorization scope file
    Validate {
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
    /// Display the contents of a scope file
    Show {
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
    /// Add a permitted target to a scope file
    AddTarget {
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
        #[arg(long, value_name = "TARGET")]
        target: String,
    },
    /// List permitted and excluded targets in a scope file
    List {
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
    /// Explain whether a specific target is permitted by a scope file
    Explain {
        #[arg(long, value_name = "TARGET")]
        target: String,
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GraphExportFormat {
    Json,
    Csv,
    Gexf,
}

#[derive(Debug, Args)]
pub struct GraphArgs {
    #[command(subcommand)]
    pub command: GraphCommand,
}

#[derive(Debug, Subcommand)]
pub enum GraphCommand {
    /// Show graph node and edge statistics
    Stats,
    /// List device nodes
    Devices,
    /// List service nodes
    Services,
    /// List relationships (edges)
    Relationships,
    /// Query graph nodes using the safe query grammar
    Query {
        #[arg(long)]
        query: String,
    },
    /// Find shortest path between two nodes
    Path {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long, default_value_t = 6)]
        max_hops: usize,
        #[arg(long)]
        explain: bool,
    },
    /// Show neighborhood around a node
    Neighborhood {
        #[arg(long)]
        node: String,
        #[arg(long, default_value_t = 2)]
        depth: usize,
    },
    /// Export graph to a file
    Export {
        #[arg(long, value_enum, default_value = "json")]
        format: GraphExportFormat,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
    },
    /// Validate database schema and graph integrity
    Validate,
    /// Run or preview database migrations
    Migrate {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Args)]
pub struct SnapshotArgs {
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
    /// Create a named snapshot of the current graph
    Create {
        #[arg(long)]
        name: String,
        #[arg(long, value_name = "FILE")]
        scope: PathBuf,
    },
    /// List stored snapshots
    List,
    /// Show details of a snapshot
    Show {
        #[arg(long)]
        id: String,
    },
    /// Diff two snapshots
    Diff {
        #[arg(long)]
        first: String,
        #[arg(long)]
        second: String,
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },
    /// Prune snapshots older than N days
    Prune {
        #[arg(long, default_value_t = 30)]
        older_than: i64,
        #[arg(long)]
        dry_run: bool,
    },
    /// Export a snapshot as an offline model copy
    RestoreModel {
        #[arg(long)]
        id: String,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct TwinArgs {
    #[command(subcommand)]
    pub command: TwinCommand,
}

#[derive(Debug, Subcommand)]
pub enum TwinCommand {
    /// Build an offline digital twin model from a snapshot
    Build {
        #[arg(long)]
        snapshot: String,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
    },
    /// Inspect an offline digital twin model
    Inspect {
        #[arg(long, value_name = "FILE")]
        model: PathBuf,
    },
    /// Simulate a scenario against an offline digital twin model
    Simulate {
        #[arg(long, value_name = "FILE")]
        scenario: PathBuf,
        #[arg(long, value_name = "FILE")]
        model: PathBuf,
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Validate a twin scenario YAML file
    ScenarioValidate {
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct PdefArgs {
    #[command(subcommand)]
    pub command: PdefCommand,
}

#[derive(Debug, Subcommand)]
pub enum PdefCommand {
    /// Initialize a Protocol Definition File template
    Init {
        #[arg(long, default_value = "smtp")]
        protocol: String,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
    },
    /// Validate a Protocol Definition File
    Validate {
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
    /// Lint a Protocol Definition File for potential issues
    Lint {
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct ProtocolArgs {
    #[command(subcommand)]
    pub command: ProtocolCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProtocolCommand {
    /// Run safe FSM state-transition tests against a local fixture or authorized target
    Test {
        #[arg(long)]
        target: String,
        #[arg(long, value_name = "FILE")]
        definition: PathBuf,
        #[arg(long, default_value = "safe-transition-checks")]
        suite: String,
        #[arg(long, default_value_t = 20)]
        max_cases: usize,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        i_have_permission: bool,
    },
    /// Fingerprint protocol behavior ("Protocol DNA")
    Fingerprint {
        #[arg(long)]
        target: String,
        #[arg(long, default_value = "smtp")]
        profile: String,
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Compare two Protocol DNA fingerprint files
    Compare {
        #[arg(long, value_name = "FILE")]
        first: PathBuf,
        #[arg(long, value_name = "FILE")]
        second: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct EvidenceArgs {
    #[command(subcommand)]
    pub command: EvidenceCommand,
}

#[derive(Debug, Subcommand)]
pub enum EvidenceCommand {
    /// List stored evidence records
    List {
        #[arg(long)]
        case: Option<String>,
    },
    /// Show a specific evidence record
    Show {
        #[arg(long)]
        id: String,
    },
    /// Redact sensitive tokens from a text file
    Redact {
        #[arg(long, value_name = "FILE")]
        input: PathBuf,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct IntelligenceArgs {
    #[command(subcommand)]
    pub command: IntelligenceCommand,
}

#[derive(Debug, Subcommand)]
pub enum IntelligenceCommand {
    /// Analyze the current network graph and produce explainable findings
    Analyze,
}

#[derive(Debug, Args)]
pub struct InvestigateArgs {
    #[command(subcommand)]
    pub command: InvestigateCommand,
}

#[derive(Debug, Subcommand)]
pub enum InvestigateCommand {
    /// List investigation cases
    List,
    /// Show an investigation case
    Show {
        #[arg(long)]
        case: String,
    },
    /// Open a new investigation case for a finding
    Run {
        #[arg(long)]
        finding: String,
    },
    /// Close an investigation case
    Close {
        #[arg(long)]
        case: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    #[command(subcommand)]
    pub command: ReportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ReportCommand {
    /// Build a report for an investigation case
    Build {
        #[arg(long)]
        case: String,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long, value_name = "DIR", default_value = "./reports")]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct DiscoverArgs {
    #[command(subcommand)]
    pub command: DiscoverCommand,
}

#[derive(Debug, Subcommand)]
pub enum DiscoverCommand {
    /// Import an inventory from JSON or CSV into the network graph
    Import {
        #[arg(long, value_name = "FILE")]
        input: PathBuf,
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Validate and preview a discovery inventory run
    Inventory {
        #[arg(long, value_name = "FILE")]
        scope: PathBuf,
        #[arg(long, default_value = "passive")]
        mode: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        i_have_permission: bool,
    },
}

#[derive(Debug, Args)]
pub struct ObserveArgs {
    #[command(subcommand)]
    pub command: ObserveCommand,
}

#[derive(Debug, Subcommand)]
pub enum ObserveCommand {
    /// Replay an offline JSON capture file
    Replay {
        #[arg(long, value_name = "FILE")]
        input: PathBuf,
        #[arg(long, default_value_t = 1.0)]
        speed: f64,
    },
}
