#![feature(let_chains, if_let_guard, once_wait, string_from_utf8_lossy_owned)]
#![warn(clippy::todo, clippy::unimplemented)]
#![deny(warnings)]

#[cfg(feature = "tests")]
mod tests;

mod cmd;
mod configs;
mod rw;
mod utils;

mod init;
mod build;

mod actions;
mod pipelines;
mod entities;
mod project;

use crate::actions::{list_actions, new_action, remove_action, cat_action, edit_action};
use crate::build::Builds;
use crate::cmd::{Cli, DeployerExecType, ListType, NewType, RemoveType, CatType, EditType};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::pipelines::{list_pipelines, new_pipeline, remove_pipeline, cat_pipeline, cat_project_pipelines, assign_pipeline_to_project, edit_pipeline};
use crate::project::edit_project;
use crate::rw::{read, write, VERBOSE};
use crate::utils::get_current_working_dir;

#[cfg(feature = "tests")]
use crate::tests::tests;

use crate::init::init;
use crate::build::{build, clean_builds};

use clap::Parser;
use dirs::{config_dir, cache_dir};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static PROJECT_CONF: &str = "deploy-config.json";
static GLOBAL_CONF: &str = "deploy-global.json";
static BUILD_CACHE_LIST: &str = "deploy-builds.json";

pub(crate) static CACHE_DIR: &str = "deploy-cache";
pub(crate) static LOGS_DIR: &str = "logs";

pub(crate) static ARTIFACTS_DIR: &str = "artifacts";

#[cfg(not(unix))]
compile_error!("`deployer` can't work with non-Unix systems.");

fn main() {
  std::panic::set_hook(Box::new(|e| {
    let err = e.to_string();
    if err.contains("called `Result::unwrap()` on an `Err` value: ") {
      eprintln!("{}", err.split("called `Result::unwrap()` on an `Err` value: ").last().unwrap());
    } else {
      eprintln!("{}", err.split('\n').last().unwrap());
    }
    std::process::exit(1);
  }));
  
  ctrlc::set_handler(move || {
    println!("\nInterrupted");
    std::process::exit(0);
  }).expect("Error setting Ctrl-C handler");
  
  let args = Cli::parse();
  
  if args.verbose {
    if let DeployerExecType::Build(build_args) = &args.r#type && build_args.silent { VERBOSE.set(false).unwrap(); }
    else { VERBOSE.set(true).unwrap(); }
  } else {
    VERBOSE.set(false).unwrap();
  }
  
  // Определение рабочих директорий
  let cache_folder = if args.cache_folder.is_none() {
    cache_dir().expect("Can't get `cache` directory's location automatically, please specify one.")
  } else {
    let path = std::path::PathBuf::new();
    path.join(args.cache_folder.as_ref().unwrap())
  };
  let config_folder = if args.config_folder.is_none() {
    config_dir().expect("Can't get `config` directory's location automatically, please specify one.")
  } else {
    let path = std::path::PathBuf::new();
    path.join(args.config_folder.as_ref().unwrap())
  };
  
  // Чтение конфигов
  let mut globals = read::<DeployerGlobalConfig>(&config_folder, GLOBAL_CONF);
  let mut config = read::<DeployerProjectOptions>(&get_current_working_dir().unwrap(), PROJECT_CONF);
  let mut builds = read::<Builds>(&cache_folder, BUILD_CACHE_LIST);
  
  match args.r#type {
    DeployerExecType::Ls(ListType::Actions) => list_actions(&globals),
    DeployerExecType::New(NewType::Action(args)) => {
      let _ = new_action(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    },
    DeployerExecType::Cat(CatType::Action(args)) => cat_action(&globals, &args).unwrap(),
    DeployerExecType::Edit(EditType::Action(args)) => {
      edit_action(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    },
    DeployerExecType::Rm(RemoveType::Action) => {
      remove_action(&mut globals).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    },
    
    DeployerExecType::Ls(ListType::Pipelines) => list_pipelines(&globals).unwrap(),
    DeployerExecType::New(NewType::Pipeline(args)) => {
      new_pipeline(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    },
    DeployerExecType::Cat(CatType::Pipeline(args)) => cat_pipeline(&globals, &args).unwrap(),
    DeployerExecType::Edit(EditType::Pipeline(args)) => {
      edit_pipeline(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    },
    DeployerExecType::Rm(RemoveType::Pipeline) => {
      remove_pipeline(&mut globals).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    },
    
    DeployerExecType::Init(args) => {
      init(&mut globals, &mut config, &args).unwrap();
      write(get_current_working_dir().unwrap(), PROJECT_CONF, &config);
    },
    DeployerExecType::With(args) => {
      assign_pipeline_to_project(&mut globals, &mut config, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
      write(get_current_working_dir().unwrap(), PROJECT_CONF, &config);
    },
    DeployerExecType::Cat(CatType::Project) => cat_project_pipelines(&config).unwrap(),
    DeployerExecType::Edit(EditType::Project) => {
      edit_project(&mut globals, &mut config).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
      write(get_current_working_dir().unwrap(), PROJECT_CONF, &config);
    },
    DeployerExecType::Build(args) => {
      build(&mut config, &mut builds, &cache_folder, &args).unwrap();
      write(&cache_folder, BUILD_CACHE_LIST, &builds);
    },
    DeployerExecType::Clean(args) => {
      clean_builds(&config, &mut builds, &cache_folder, &args).unwrap();
      write(&cache_folder, BUILD_CACHE_LIST, &builds);
    },
    
    #[cfg(feature = "tests")]
    DeployerExecType::Tests => tests().unwrap(),
  }
}
