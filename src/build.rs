use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{CACHE_DIR, ARTIFACTS_DIR, BUILD_CACHE_LIST};
use crate::entities::environment::BuildEnvironment;
use crate::cmd::{BuildArgs, CleanArgs};
use crate::configs::DeployerProjectOptions;
use crate::pipelines::{execute_pipeline, DescribedPipeline};
use crate::rw::{copy_all, write, symlink, log};
use crate::utils::get_current_working_dir;

/// Список всех билдов в системе
#[derive(Deserialize, Serialize, Default)]
pub(crate) struct Builds {
  pub(crate) projects: Vec<ProjectBuilds>,
}

/// Билды определённого проекта
#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct ProjectBuilds {
  pub(crate) name: String,
  pub(crate) builds: Vec<BuildStats>,
}

/// Информация о сборке
#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct BuildStats {
  /// Этот билд будет использоваться только для определённых пайплайнов
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) exclusive_tag: Option<String>,
  /// Путь сборки
  pub(crate) folder: PathBuf,
}

impl BuildStats {
  pub(crate) fn works_with(&self, pipeline: &DescribedPipeline) -> bool {
    self.exclusive_tag.as_ref().is_some_and(|a| pipeline.exclusive_exec_tag.as_ref().is_some_and(|b| a.as_str().eq(b.as_str()))) ||
    (self.exclusive_tag.is_none() && pipeline.exclusive_exec_tag.is_none())
  }
}

pub(crate) fn enplace_artifacts(
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
  config: &DeployerProjectOptions,
  builds: &mut Builds,
  selected_pipeline: &DescribedPipeline,
  current_dir: &std::path::Path,
  cache_dir: &Path,
  args: &BuildArgs,
) -> anyhow::Result<(PathBuf, bool)> {
  let build_path = if let Some(build_at) = args.build_at.as_ref() {
    build_at.to_owned()
  } else {
    let mut build_path = PathBuf::new();
    build_path.push(cache_dir);
    build_path.push(CACHE_DIR);
    
    let mut project_builds = match builds.projects.iter().position(|p| p.name.as_str().eq(config.project_name.as_str())) {
      None => ProjectBuilds { name: config.project_name.to_owned(), builds: vec![] },
      Some(project_builds) => {
        let copy = builds.projects.get(project_builds).unwrap().clone();
        builds.projects.remove(project_builds);
        copy
      },
    };
    
    let folder = match project_builds.builds.iter().rev().find(|b| b.works_with(selected_pipeline)) {
      Some(b_stats) if !args.fresh => {
        b_stats.folder.to_owned()
      },
      _ => {
        let uuid = format!("deploy-build-{}", Uuid::new_v4());
        let folder = build_path.join(uuid);
        let b_stats = BuildStats { exclusive_tag: selected_pipeline.exclusive_exec_tag.clone(), folder: folder.to_owned(), };
        project_builds.builds.push(b_stats);
        folder.to_owned()
      },
    };
    
    builds.projects.push(project_builds);
    
    folder
  };
  
  let fresh = !build_path.exists() || args.fresh;
  std::fs::create_dir_all(build_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", build_path));
  
  let mut ignore = vec![ARTIFACTS_DIR, build_path.file_name().unwrap().to_str().unwrap()];
  ignore.extend_from_slice(&config.cache_files.iter().map(|v| v.as_str()).collect::<Vec<_>>());
  
  copy_all(get_current_working_dir().unwrap(), build_path.as_path(), &ignore)?;
  write(cache_dir, BUILD_CACHE_LIST, &builds);
  
  if args.link_cache {
    for cache_item in &config.cache_files {
      symlink(current_dir.join(cache_item.as_str()), build_path.join(cache_item.as_str()));
      log(format!("-> {}", cache_item.as_str()));
    }
  }
  
  if args.copy_cache {
    for cache_item in &config.cache_files {
      copy_all(
        current_dir.join(cache_item.as_str()),
        build_path.join(cache_item.as_str()),
        &[]
      )?;
      log(format!("-> {}", cache_item.as_str()));
    }
  }
  
  Ok((build_path, fresh))
}

pub(crate) fn build(
  config: &mut DeployerProjectOptions,
  builds: &mut Builds,
  cache_dir: &Path,
  args: &BuildArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("Config is invalid! Reinit the project."); }
  
  if args.link_cache && args.copy_cache { panic!(
    "Select only one option from `{}` and `{}`. See help via `{}`.", "c".green(), "C".green(), "deployer build -h".green()
  ); }
  if (args.fresh || args.link_cache || args.copy_cache || args.build_at.is_some()) && args.current { panic!(
    "Select either `{}` or `{}`/{}`/`{}`/`{}` options. See help via `{}`.",
    "o".green(),
    "j".green(),
    "f".green(),
    "c".green(),
    "C".green(),
    "deployer build -h".green(),
  ); }
  if args.silent && args.no_pipe { panic!(
    "Select only one option from `{}` and `{}`. See help via `{}`.", "s".green(), "t".green(), "deployer build -h".green()
  ); }
  
  let curr_dir = std::env::current_dir().expect("Can't get current dir!");
  let artifacts_dir = prepare_artifacts_folder(&curr_dir)?;
  
  if args.pipeline_tags.is_empty() {
    if config.pipelines.is_empty() {
      panic!("The pipelines' list is empty! Check the config file for errors.");
    }
    if let Some(pipeline) = &config.pipelines.iter().find(|p| p.default.is_some_and(|v| v)) {
      let (build_path, new_build) = if args.current {
        (curr_dir.clone(), false)
      } else {
        prepare_build_folder(config, builds, pipeline, &curr_dir, cache_dir, args)?
      };
      
      let env = BuildEnvironment {
        build_dir: &build_path,
        cache_dir,
        artifacts_dir: &artifacts_dir,
        new_build,
        silent_build: args.silent,
        no_pipe: args.no_pipe,
      };
      
      execute_pipeline(config, env, pipeline)?;
      
      enplace_artifacts(config, env, true)?;
    }
  } else {
    for pipeline_tag in &args.pipeline_tags {
      if let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
        let (build_path, new_build) = if args.current {
          (curr_dir.clone(), false)
        } else {
          prepare_build_folder(config, builds, pipeline, &curr_dir, cache_dir, args)?
        };
        
        let env = BuildEnvironment {
          build_dir: &build_path,
          cache_dir,
          artifacts_dir: &artifacts_dir,
          new_build,
          silent_build: args.silent,
          no_pipe: args.no_pipe,
        };
        
        execute_pipeline(config, env, pipeline)?;
        
        enplace_artifacts(config, env, true)?;
      } else {
        panic!(
          "There is no such Pipeline `{}` set up for this project. Maybe, you've forgotten set up this Pipeline for project via `{}`?",
          pipeline_tag.green(),
          "deployer with {pipeline-short-name-and-ver}".green(),
        );
      }
    }
  }
  
  Ok(())
}

pub(crate) fn clean_builds(
  config: &DeployerProjectOptions,
  builds: &mut Builds,
  cache_dir: &Path,
  args: &CleanArgs,
) -> anyhow::Result<()> {
  let mut path = PathBuf::new();
  path.push(cache_dir);
  path.push(CACHE_DIR);
  
  if let Some(project_builds) = builds.projects.iter_mut().find(|p| p.name.as_str().eq(config.project_name.as_str())) {
    for folder in project_builds.builds.iter().map(|b| b.folder.clone()) {
      let _ = std::fs::remove_dir_all(folder);
    }
    project_builds.builds.clear();
  }
  
  if args.include_artifacts {
    let curr_dir = std::env::current_dir()?;
    let artifacts_dir = curr_dir.join(ARTIFACTS_DIR);
    if artifacts_dir.as_path().exists() {
      let _ = std::fs::remove_dir_all(artifacts_dir);
    }
  }
  
  Ok(())
}
