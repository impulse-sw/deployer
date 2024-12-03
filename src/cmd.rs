use clap::{Args, Subcommand, Parser};

/// Build and deploy your services as fast as you can.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
  /// Command
  #[command(subcommand)]
  pub(crate) r#type: DeployerExecType,
  /// Verbose
  #[arg(short)]
  pub(crate) verbose: bool,
  /// Specify cache folder
  #[arg(long)]
  pub(crate) cache_folder: Option<String>,
  /// Specify config folder
  #[arg(long)]
  pub(crate) config_folder: Option<String>,
  /// Specify data folder
  #[arg(long)]
  pub(crate) data_folder: Option<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum DeployerExecType {
  /// List inner Deployer's registries
  #[command(subcommand)]
  Ls(ListType),
  /// Create new inner Deployer's object
  #[command(subcommand)]
  New(NewType),
  /// Print info about inner Deployer's object
  #[command(subcommand)]
  Cat(CatType),
  /// Remove the inner Deployer's object
  #[command(subcommand)]
  Rm(RemoveType),
  
  /// Export action, pipeline or artifact
  Export(ExportArgs),
  
  /// Init the deployable project
  Init(InitArgs),
  /// Add Deployer's Pipeline to the project
  With(WithPipelineArgs),
  /// Build the project
  Build(BuildArgs),
  /// Clean the project's builds
  Clean(CleanArgs),
  
  #[cfg(feature = "tests")]
  Tests,
}

#[derive(Subcommand, Debug)]
pub(crate) enum ListType {
  /// List available Actions
  Actions,
  /// List available Pipelines
  Pipelines,
}

#[derive(Subcommand, Debug)]
pub(crate) enum RemoveType {
  /// Remove an Action
  Action,
  /// Remove a Pipeline
  Pipeline,
}

#[derive(Subcommand, Debug)]
pub(crate) enum CatType {
  /// Prints an Action
  Action(CatActionArgs),
  /// Prints a Pipeline
  Pipeline(CatPipelineArgs),
  /// Prints all Pipelines used by current Project
  Project,
}

#[derive(Args, Debug)]
pub(crate) struct CatActionArgs {
  pub(crate) action_short_info_and_version: String,
}

#[derive(Args, Debug)]
pub(crate) struct CatPipelineArgs {
  pub(crate) pipeline_short_info_and_version: String,
}

#[derive(Subcommand, Debug)]
pub(crate) enum NewType {
  /// Add new Action to Deployer's registry
  Action(NewActionArgs),
  /// Add new Pipeline to Deployer's registry
  Pipeline(NewPipelineArgs),
}

#[derive(Args, Debug)]
pub(crate) struct NewActionArgs {
  /// From description in JSON/YAML
  #[arg(short, long)]
  pub(crate) from: Option<String>,
}

pub(crate) type NewPipelineArgs = NewActionArgs;

#[derive(Args, Debug)]
pub(crate) struct ExportArgs {
  #[command(subcommand)]
  pub(crate) export_type: ExportType,
  /// {short-name}@{version}
  pub(crate) tag: String,
}

#[derive(Subcommand, Debug)]
pub(crate) enum ExportType {
  Action,
  Pipeline,
  Artifact,
}

#[derive(Args, Debug)]
pub(crate) struct InitArgs {
  
}

#[derive(Args, Debug)]
pub(crate) struct WithPipelineArgs {
  /// {short-name}@{version}
  pub(crate) tag: String,
  /// {short-name}
  #[arg(short, long)]
  pub(crate) r#as: Option<String>,
}

#[derive(Args, Debug)]
pub(crate) struct CleanArgs {
  /// Clean current project artifacts
  #[arg(short, long)]
  pub(crate) include_artifacts: bool,
}

#[derive(Args, Debug)]
pub(crate) struct BuildArgs {
  /// {short-name}
  pub(crate) pipeline_tag: Option<String>,
  /// Fresh build
  #[arg(short, long)]
  pub(crate) fresh: bool,
  /// With cache
  #[arg(short, long)]
  pub(crate) with_cache: bool,
  /// With cache too, but copy cache files and folders instead of symlinking
  #[arg(long)]
  pub(crate) copy_cache: bool,
  /// Disable output from Actions
  #[arg(short, long)]
  pub(crate) silent: bool,
}
