use colored::Colorize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{DEPLOY_CACHE_SUBDIR, DEPLOY_ARTIFACTS_SUBDIR, DEPLOY_CONF_FILE};
use crate::actions::*;
use crate::entities::traits::Execute;
use crate::cmd::{BuildArgs, CleanArgs};
use crate::configs::DeployerProjectOptions;
use crate::pipelines::DescribedPipeline;
use crate::rw::{copy_all, write, symlink, log};
use crate::utils::get_current_working_dir;

fn enplace_artifacts(
  config: &DeployerProjectOptions,
  build_path: &Path,
  artifacts_dir: &Path,
  panic_when_not_found: bool,
) -> anyhow::Result<()> {
  let mut ignore = vec![DEPLOY_ARTIFACTS_SUBDIR];
  ignore.extend_from_slice(&(config.cache_files.iter().map(|c| c.as_str()).collect::<Vec<_>>()));
  
  for (from, to) in &config.inplace_artifacts_into_project_root {
    let artifact_path = build_path.join(from);
    if !std::fs::exists(artifact_path.clone())? {
      if panic_when_not_found { panic!("There is no `{:?}` artifact!", artifact_path); }
    } else if artifact_path.as_path().is_dir() || artifact_path.as_path().is_file() {
      copy_all(artifact_path.as_path(), artifacts_dir.join(to).as_path(), &ignore)?;
    }
  }
  
  Ok(())
}

pub(crate) fn build(
  config: &mut DeployerProjectOptions,
  cache_dir: &str,
  args: &mut BuildArgs,
) -> anyhow::Result<()> {
  if
    let Some(pipeline_tag) = &args.pipeline_tag &&
    !config.pipelines.iter().any(|p| p.title.as_str() == pipeline_tag.as_str())
  {
    panic!("There is no such Pipeline set up for this project. Maybe, you've forgotten `deployer with {{pipeline-short-name-and-ver}}`?");
  }
  
  let mut build_path = PathBuf::new();
  build_path.push(cache_dir);
  build_path.push(DEPLOY_CACHE_SUBDIR);
  std::fs::create_dir_all(build_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", build_path));
  
  let curr_dir = std::env::current_dir().expect("Can't get current dir!");
  let artifacts_dir = curr_dir.join(DEPLOY_ARTIFACTS_SUBDIR);
  std::fs::create_dir_all(artifacts_dir.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", build_path));
  
  if config.last_build.is_none() { args.fresh = true; }
  
  let uuid = match args.fresh {
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
  
  let mut ignore = vec![DEPLOY_ARTIFACTS_SUBDIR, &uuid];
  ignore.extend_from_slice(&config.cache_files.iter().map(|v| v.as_str()).collect::<Vec<_>>());
  
  copy_all(get_current_working_dir().unwrap(), build_path.as_path(), &ignore)?;
  write(get_current_working_dir().unwrap(), DEPLOY_CONF_FILE, &config);
  
  if args.with_cache {
    match args.copy_cache {
      false => {
        for cache_item in &config.cache_files {
          symlink(curr_dir.join(cache_item.as_str()), build_path.join(cache_item.as_str()));
          log(format!("-> {}", cache_item.as_str()));
        }
      },
      true => {
        for cache_item in &config.cache_files {
          copy_all(
            curr_dir.join(cache_item.as_str()),
            build_path.join(cache_item.as_str()),
            &[]
          )?;
          log(format!("-> {}", cache_item.as_str()));
        }
      },
    }
  }
  
  if
    let Some(pipeline_tag) = &args.pipeline_tag &&
    let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str() == pipeline_tag)
  {
    execute_pipeline(config, args.silent, pipeline, &build_path, &artifacts_dir)?;
  } else {
    if config.pipelines.is_empty() {
      anyhow::bail!("The pipelines' list is empty! Check the config file for errors.")
    }
    
    if let Some(pipeline) = &config.pipelines.iter().find(|p| p.default.is_some_and(|v| v)) {
      execute_pipeline(config, args.silent, pipeline, &build_path, &artifacts_dir)?;
    } else {
      for pipeline in &config.pipelines {
        execute_pipeline(config, args.silent, pipeline, &build_path, &artifacts_dir)?;
      }
    }
  }
  
  enplace_artifacts(config, &build_path, &artifacts_dir, true)?;
  
  println!("Build path: {}", build_path.to_str().expect("Can't convert `Path` to string!"));
  
  Ok(())
}

fn execute_pipeline(
  config: &DeployerProjectOptions,
  silent: bool,
  pipeline: &DescribedPipeline,
  build_dir: &Path,
  artifacts_dir: &Path,
) -> anyhow::Result<()> {
  use std::io::{stdout, Write};
  use std::time::Instant;
  
  println!("Starting the `{}` Pipeline...", pipeline.title);
  let mut cntr = 1usize;
  let total = pipeline.actions.len();
  for action in &pipeline.actions {
    print!("[{}/{}] `{}` Action... ", cntr, total, action.title.blue().italic());
    stdout().flush()?;
    let now = Instant::now();
    
    let (status, output) = match &action.action {
      Action::Custom(cmd) => cmd.execute(build_dir)?,
      Action::Check(check) => check.execute(build_dir)?,
      Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => a.execute(build_dir)?,
      Action::ProjectClean(pc_action) => pc_action.execute(build_dir)?,
      Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => a.execute(build_dir)?,
      Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => a.execute(build_dir)?,
      Action::Observe(o_action) => o_action.execute(build_dir)?,
      Action::ForceArtifactsEnplace => {
        enplace_artifacts(config, build_dir, artifacts_dir, false)?;
        enplace_artifacts(config, build_dir, &build_dir.to_path_buf().join(DEPLOY_ARTIFACTS_SUBDIR), false)?;
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
    
    let elapsed = now.elapsed();
    println!("{} ({}).", status_str, format!("{:.2?}", elapsed).green());
    cntr += 1;
    
    if !silent {
      for line in output { println!("{}", line); }
    }
    
    if !status {
      return Ok(())
    }
  }
  
  Ok(())
}

pub(crate) fn clean_builds(
  config: &mut DeployerProjectOptions,
  cache_dir: &str,
  args: &CleanArgs,
) -> anyhow::Result<()> {
  let mut path = PathBuf::new();
  path.push(cache_dir);
  path.push(DEPLOY_CACHE_SUBDIR);
  
  config.last_build = None;
  for build in &config.builds {
    let mut build_path = path.clone();
    build_path.push(build);
    let _ = std::fs::remove_dir_all(build_path);
  }
  config.builds.clear();
  
  if args.include_artifacts {
    let curr_dir = std::env::current_dir()?;
    let artifacts_dir = curr_dir.join(DEPLOY_ARTIFACTS_SUBDIR);
    if artifacts_dir.as_path().exists() {
      let _ = std::fs::remove_dir_all(artifacts_dir);
    }
  }
  
  Ok(())
}
