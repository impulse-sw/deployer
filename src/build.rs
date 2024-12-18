use colored::Colorize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{CACHE_DIR, ARTIFACTS_DIR, PROJECT_CONF};
use crate::actions::*;
use crate::entities::{
  traits::Execute,
  environment::BuildEnvironment,
};
use crate::cmd::{BuildArgs, CleanArgs};
use crate::configs::DeployerProjectOptions;
use crate::pipelines::DescribedPipeline;
use crate::rw::{copy_all, write, symlink, log, generate_build_log_filepath, build_log};
use crate::utils::get_current_working_dir;

fn enplace_artifacts(
  config: &DeployerProjectOptions,
  env: BuildEnvironment,
  panic_when_not_found: bool,
) -> anyhow::Result<()> {
  let mut ignore = vec![ARTIFACTS_DIR];
  ignore.extend_from_slice(&(config.cache_files.iter().map(|c| c.as_str()).collect::<Vec<_>>()));
  
  for (from, to) in &config.inplace_artifacts_into_project_root {
    let artifact_path = env.build_dir.join(from);
    if !std::fs::exists(artifact_path.clone())? {
      if panic_when_not_found { panic!("There is no `{:?}` artifact!", artifact_path); }
    } else if artifact_path.as_path().is_dir() || artifact_path.as_path().is_file() {
      copy_all(artifact_path.as_path(), env.artifacts_dir.join(to).as_path(), &ignore)?;
    }
  }
  
  Ok(())
}

fn prepare_artifacts_folder(
  current_dir: &std::path::Path,
) -> anyhow::Result<PathBuf> {
  let artifacts_dir = current_dir.join(ARTIFACTS_DIR);
  std::fs::create_dir_all(artifacts_dir.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", artifacts_dir));
  
  Ok(artifacts_dir)
}

fn prepare_build_folder(
  config: &mut DeployerProjectOptions,
  current_dir: &std::path::Path,
  cache_dir: &Path,
  mut fresh: bool,
  link_cache: bool,
  copy_cache: bool,
  new_build: &mut bool,
) -> anyhow::Result<PathBuf> {
  let mut build_path = PathBuf::new();
  build_path.push(cache_dir);
  build_path.push(CACHE_DIR);
  if !build_path.exists() { *new_build = true; }
  std::fs::create_dir_all(build_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", build_path));
  
  if config.last_build.is_none() { fresh = true; }
  
  let uuid = match fresh {
    true => {
      let uuid = format!("deploy-build-{}", Uuid::new_v4());
      config.builds.push(uuid.clone());
      config.last_build = Some(uuid.clone());
      uuid
    },
    false => {
      config.last_build.as_ref().unwrap().to_owned()
    },
  };
  
  build_path.push(uuid.clone());
  
  let mut ignore = vec![ARTIFACTS_DIR, &uuid];
  ignore.extend_from_slice(&config.cache_files.iter().map(|v| v.as_str()).collect::<Vec<_>>());
  
  copy_all(get_current_working_dir().unwrap(), build_path.as_path(), &ignore)?;
  write(get_current_working_dir().unwrap(), PROJECT_CONF, &config);
  
  if link_cache {
    for cache_item in &config.cache_files {
      symlink(current_dir.join(cache_item.as_str()), build_path.join(cache_item.as_str()));
      log(format!("-> {}", cache_item.as_str()));
    }
  }
  
  if copy_cache {
    for cache_item in &config.cache_files {
      copy_all(
        current_dir.join(cache_item.as_str()),
        build_path.join(cache_item.as_str()),
        &[]
      )?;
      log(format!("-> {}", cache_item.as_str()));
    }
  }
  
  Ok(build_path)
}

pub(crate) fn build(
  config: &mut DeployerProjectOptions,
  cache_dir: &Path,
  args: &BuildArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("Config is invalid!"); }
  
  if args.link_cache && args.copy_cache { panic!(
    "Select only one option from `{}` and `{}`. See help via `{}`.", "c".green(), "C".green(), "deployer build -h".green()
  ); }
  if (args.fresh || args.link_cache || args.copy_cache) && args.current { panic!(
    "Select either `{}` or `{}`/`{}`/`{}` options. See help via `{}`.", "j".green(), "f".green(), "c".green(), "C".green(), "deployer build -h".green()
  ); }
  if args.silent && args.no_pipe { panic!(
    "Select only one option from `{}` and `{}`. See help via `{}`.", "s".green(), "t".green(), "deployer build -h".green()
  ); }
  
  let mut new_build = false;
  
  let curr_dir = std::env::current_dir().expect("Can't get current dir!");
  let artifacts_dir = prepare_artifacts_folder(&curr_dir)?;
  let build_path = if args.current { curr_dir } else {
    prepare_build_folder(config, &curr_dir, cache_dir, args.fresh, args.link_cache, args.copy_cache, &mut new_build)?
  };
  
  let env = BuildEnvironment {
    build_dir: &build_path,
    cache_dir,
    artifacts_dir: &artifacts_dir,
    new_build,
    silent_build: args.silent,
    no_pipe: args.no_pipe,
  };
  
  if args.pipeline_tags.is_empty() {
    if config.pipelines.is_empty() {
      panic!("The pipelines' list is empty! Check the config file for errors.");
    }
    if let Some(pipeline) = &config.pipelines.iter().find(|p| p.default.is_some_and(|v| v)) {
      execute_pipeline(config, env, pipeline)?;
    }
  } else {
    for pipeline_tag in &args.pipeline_tags {
      if let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
        execute_pipeline(config, env, pipeline)?;
      } else {
        panic!(
          "There is no such Pipeline `{}` set up for this project. Maybe, you've forgotten set up this Pipeline for project via `{}`?",
          pipeline_tag.green(),
          "deployer with {pipeline-short-name-and-ver}".green(),
        );
      }
    }
  }
  
  enplace_artifacts(config, env, true)?;
  
  if !args.silent { println!("Build path: {}", build_path.to_str().expect("Can't convert `Path` to string!")); }
  
  Ok(())
}

fn execute_pipeline(
  config: &DeployerProjectOptions,
  env: BuildEnvironment,
  pipeline: &DescribedPipeline,
) -> anyhow::Result<()> {
  use std::io::{stdout, Write};
  use std::time::Instant;
  
  let log_file = generate_build_log_filepath(
    &config.project_name,
    &pipeline.title,
    env.cache_dir,
  );
  
  if !env.silent_build { println!("Starting the `{}` Pipeline...", pipeline.title); }
  build_log(&log_file, &[format!("Starting the `{}` Pipeline...", pipeline.title)])?;
  
  let mut cntr = 1usize;
  let total = pipeline.actions.len();
  for action in &pipeline.actions {
    if !env.silent_build {
      if !env.no_pipe {
        print!("[{}/{}] `{}` Action... ", cntr, total, action.title.blue().italic());
      } else {
        println!("[{}/{}] `{}` Action... ", cntr, total, action.title.blue().italic());
      }
      build_log(&log_file, &[format!("[{}/{}] `{}` Action... ", cntr, total, action.title)])?;
    }
    stdout().flush()?;
    let now = Instant::now();
    
    let (status, output) = match &action.action {
      Action::Custom(cmd) => cmd.execute(env)?,
      Action::Check(check) => check.execute(env)?,
      Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => a.execute(env)?,
      Action::ProjectClean(pc_action) => pc_action.execute(env)?,
      Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => a.execute(env)?,
      Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => a.execute(env)?,
      Action::Observe(o_action) => o_action.execute(env)?,
      Action::ForceArtifactsEnplace => {
        enplace_artifacts(config, env, false)?;
        
        let mut modified_env = env;
        let artifacts_dir = modified_env.build_dir.to_path_buf().join(ARTIFACTS_DIR);
        modified_env.artifacts_dir = &artifacts_dir;
        enplace_artifacts(config, modified_env, false)?;
        
        (true, vec!["Artifacts are enplaced successfully.".into()])
      },
      Action::Interrupt => {
        println!();
        inquire::Confirm::new("The Pipeline is interrupted. Click `Enter` to continue").with_default(true).prompt()?;
        (true, vec![])
      },
      
    };
    
    let status_str = match status {
      true => "done".to_string(),
      false => "got an error!".red().bold().to_string(),
    };
    
    if !env.silent_build {
      let elapsed = now.elapsed();
      
      if !env.no_pipe {
        println!("{} ({}).", status_str, format!("{:.2?}", elapsed).green());
        build_log(&log_file, &output)?;
        for line in output { println!("{}", line); }
      } else {
        println!("[{}/{}] `{}` Action - {} ({}).", cntr, total, action.title.blue().italic(), status_str, format!("{:.2?}", elapsed).green());
      }
      build_log(&log_file, &[
        format!("[{}/{}] `{}` Action - {} ({:.2?}).", cntr, total, action.title, if status { "done" } else { "got an error!" }, elapsed),
      ])?;
    }
    
    cntr += 1;
    
    if !status { return Ok(()) }
  }
  
  Ok(())
}

pub(crate) fn clean_builds(
  config: &mut DeployerProjectOptions,
  cache_dir: &Path,
  args: &CleanArgs,
) -> anyhow::Result<()> {
  let mut path = PathBuf::new();
  path.push(cache_dir);
  path.push(CACHE_DIR);
  
  config.last_build = None;
  for build in &config.builds {
    let mut build_path = path.clone();
    build_path.push(build);
    let _ = std::fs::remove_dir_all(build_path);
  }
  config.builds.clear();
  
  if args.include_artifacts {
    let curr_dir = std::env::current_dir()?;
    let artifacts_dir = curr_dir.join(ARTIFACTS_DIR);
    if artifacts_dir.as_path().exists() {
      let _ = std::fs::remove_dir_all(artifacts_dir);
    }
  }
  
  Ok(())
}
