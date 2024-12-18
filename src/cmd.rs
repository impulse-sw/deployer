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
  /// Edit inner Deployer's object
  #[command(subcommand)]
  Edit(EditType),
  /// Remove the inner Deployer's object
  #[command(subcommand)]
  Rm(RemoveType),
  
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

#[derive(Subcommand, Debug)]
pub(crate) enum EditType {
  /// Edits an Action
  Action(CatActionArgs),
  /// Edits a Pipeline
  Pipeline(CatPipelineArgs),
  /// Edits Project settings
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
  /// From description in JSON
  #[arg(short, long)]
  pub(crate) from: Option<String>,
}

pub(crate) type NewPipelineArgs = NewActionArgs;

#[derive(Args, Debug)]
pub(crate) struct InitArgs {
  
}

#[derive(Args, Debug)]
pub(crate) struct WithPipelineArgs {
  /// {short-name}@{version}
  pub(crate) tag: Option<String>,
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
  /// {short-name} or {short-name1},{short-name2},..
  #[arg(required = false, value_delimiter(','))]
  pub(crate) pipeline_tags: Vec<String>,
  
  /// Build in current folder
  #[arg(short('j'), long)]
  pub(crate) current: bool,
  
  /// Fresh build
  #[arg(short('f'), long)]
  pub(crate) fresh: bool,
  /// With symlinking cache
  #[arg(short('c'), long)]
  pub(crate) link_cache: bool,
  /// With copying cache
  #[arg(short('C'), long)]
  pub(crate) copy_cache: bool,
  
  /// Force disable output from Actions
  #[arg(short('s'), long)]
  pub(crate) silent: bool,
  /// Don't pipe I/O channels
  #[arg(short('t'), long)]
  pub(crate) no_pipe: bool,
}
