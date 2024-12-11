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

static DEPLOY_CONF_FILE: &str = "deploy-config.json";
static DEPLOY_GLOBAL_CONF_FILE: &str = "deploy-global.json";
pub(crate) static DEPLOY_CACHE_SUBDIR: &str = "deploy-cache";
pub(crate) static DEPLOY_ARTIFACTS_SUBDIR: &str = "artifacts";

#[cfg(not(unix))]
compile_error!("`deployer` can't work with non-Unix systems.");

fn main() {
  std::panic::set_hook(Box::new(|e| {
    eprintln!();
    eprintln!("An error occured: {}", e);
    std::process::exit(1);
  }));
  
  let args = Cli::parse();
  
  if args.verbose {
    VERBOSE.set(true).unwrap();
  } else {
    VERBOSE.set(false).unwrap();
  }
  
  // Определение рабочих директорий
  let cache_folder = if args.cache_folder.is_none() {
    cache_dir()
      .expect("Can't get `cache` directory's location automatically, please specify one.")
      .to_str()
      .expect("Can't convert `PathBuf` to `str` for `cache` folder!")
      .to_string()
  } else {
    args.cache_folder.as_ref().unwrap().to_string()
  };
  let config_folder = if args.config_folder.is_none() {
    config_dir()
      .expect("Can't get `config` directory's location automatically, please specify one.")
      .to_str()
      .expect("Can't convert `PathBuf` to `str` for `config` folder!")
      .to_string()
  } else {
    args.config_folder.as_ref().unwrap().to_string()
  };
  
  // Чтение конфигов
  let mut globals = read::<DeployerGlobalConfig>(&config_folder, DEPLOY_GLOBAL_CONF_FILE);
  let mut config = read::<DeployerProjectOptions>(&get_current_working_dir().unwrap(), DEPLOY_CONF_FILE);
  
  match args.r#type {
    DeployerExecType::Ls(ListType::Actions) => list_actions(&globals),
    DeployerExecType::New(NewType::Action(args)) => {
      let _ = new_action(&mut globals, &args).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
    },
    DeployerExecType::Cat(CatType::Action(args)) => cat_action(&globals, &args).unwrap(),
    DeployerExecType::Edit(EditType::Action(args)) => {
      edit_action(&mut globals, &args).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
    },
    DeployerExecType::Rm(RemoveType::Action) => {
      remove_action(&mut globals).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
    },
    
    DeployerExecType::Ls(ListType::Pipelines) => list_pipelines(&globals).unwrap(),
    DeployerExecType::New(NewType::Pipeline(args)) => {
      new_pipeline(&mut globals, &args).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
    },
    DeployerExecType::Cat(CatType::Pipeline(args)) => cat_pipeline(&globals, &args).unwrap(),
    DeployerExecType::Edit(EditType::Pipeline(args)) => {
      edit_pipeline(&mut globals, &args).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
    },
    DeployerExecType::Rm(RemoveType::Pipeline) => {
      remove_pipeline(&mut globals).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
    },
    
    DeployerExecType::Init(args) => {
      init(&mut globals, &mut config, &args).unwrap();
      write(get_current_working_dir().unwrap(), DEPLOY_CONF_FILE, &config);
    },
    DeployerExecType::With(args) => {
      assign_pipeline_to_project(&mut globals, &mut config, &args).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
      write(get_current_working_dir().unwrap(), DEPLOY_CONF_FILE, &config);
    },
    DeployerExecType::Cat(CatType::Project) => cat_project_pipelines(&config).unwrap(),
    DeployerExecType::Edit(EditType::Project) => {
      edit_project(&mut globals, &mut config).unwrap();
      write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
      write(get_current_working_dir().unwrap(), DEPLOY_CONF_FILE, &config);
    },
    DeployerExecType::Build(mut args) => build(&mut config, &cache_folder, &mut args).unwrap(),
    DeployerExecType::Clean(args) => {
      clean_builds(&mut config, &cache_folder, &args).unwrap();
      write(get_current_working_dir().unwrap(), DEPLOY_CONF_FILE, &config);
    },
    
    #[cfg(feature = "tests")]
    DeployerExecType::Tests => tests().unwrap(),
  }
}
