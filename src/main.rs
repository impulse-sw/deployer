#![feature(let_chains, once_wait, string_from_utf8_lossy_owned)]
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

use crate::actions::{list_actions, new_action, remove_action, cat_action};
use crate::cmd::{Cli, DeployerExecType, ListType, NewType, RemoveType, CatType};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::pipelines::{list_pipelines, new_pipeline, remove_pipeline, cat_pipeline, assign_pipeline_to_project};
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
compile_error!("`cc-deploy` can't work with non-Unix systems.");

fn main() {
  std::panic::set_hook(Box::new(|e| {
    println!();
    println!("An error occured: {}", e);
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
    DeployerExecType::New(NewType::Action(args)) => { let _ = new_action(&mut globals, &args).unwrap(); },
    DeployerExecType::Cat(CatType::Action(args)) => cat_action(&globals, &args).unwrap(),
    DeployerExecType::Rm(RemoveType::Action) => remove_action(&mut globals).unwrap(),
    
    DeployerExecType::Ls(ListType::Pipelines) => list_pipelines(&globals).unwrap(),
    DeployerExecType::New(NewType::Pipeline(args)) => new_pipeline(&mut globals, &args).unwrap(),
    DeployerExecType::Cat(CatType::Pipeline(args)) => cat_pipeline(&globals, &args).unwrap(),
    DeployerExecType::Rm(RemoveType::Pipeline) => remove_pipeline(&mut globals).unwrap(),
    
    DeployerExecType::Init(_) => init(&mut globals, &mut config).unwrap(),
    DeployerExecType::With(args) => assign_pipeline_to_project(&globals, &mut config, &args).unwrap(),
    DeployerExecType::Build(mut args) => build(&mut config, &cache_folder, &mut args).unwrap(),
    DeployerExecType::Clean(args) => clean_builds(&mut config, &cache_folder, &args).unwrap(),
    
    #[cfg(feature = "tests")]
    DeployerExecType::Tests => tests().unwrap(),
    
    _ => unimplemented!(),
  }
  
  // Запись конфигов
  write(&config_folder, DEPLOY_GLOBAL_CONF_FILE, &globals);
  write(&get_current_working_dir().unwrap(), DEPLOY_CONF_FILE, &config);
}
