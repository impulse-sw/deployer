use anyhow::anyhow;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uuid::Uuid;

use crate::{DEPLOY_CACHE_SUBDIR, DEPLOY_ARTIFACTS_SUBDIR};
use crate::actions::{Action, CustomCommand, ProjectCleanAction, BuildAction, PackAction, DeployAction};
use crate::cmd::{BuildArgs, CleanArgs};
use crate::configs::DeployerProjectOptions;
use crate::pipelines::DescribedPipeline;
use crate::rw::{copy_all, symlink, log};

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
  
  copy_all(".", build_path.as_path(), &["Cargo.lock", "target", ".git", "deploy-config.json", DEPLOY_ARTIFACTS_SUBDIR, &uuid])?;
  
  if args.with_cache {
    match args.copy_cache {
      false => {
        symlink(curr_dir.join("Cargo.lock"), build_path.join("Cargo.lock"));
        log("-> Cargo.lock");
        symlink(curr_dir.join("target"), build_path.join("target"));
        log("-> target/*");
      },
      true => {
        std::fs::copy(curr_dir.join("Cargo.lock"), build_path.join("Cargo.lock")).unwrap();
        log("-> Cargo.lock");
        copy_all(curr_dir.join("target"), build_path.as_path(), &[".git", "deploy-config.json", DEPLOY_ARTIFACTS_SUBDIR, &uuid])?;
      },
    }
  }
  
  if
    let Some(pipeline_tag) = &args.pipeline_tag &&
    let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str() == pipeline_tag)
  {
    execute_pipeline(args.silent, pipeline, &build_path)?;
  } else {
    for pipeline in &config.pipelines {
      execute_pipeline(args.silent, pipeline, &build_path)?;
    }
  }
  
  let mut ignore = vec![DEPLOY_ARTIFACTS_SUBDIR];
  ignore.extend_from_slice(&(config.cache_files.iter().map(|c| c.as_str()).collect::<Vec<_>>()));
  
  for (from, to) in &config.inplace_artifacts_into_project_root {
    let artifact_path = build_path.join(from);
    if !std::fs::exists(artifact_path.clone())? {
      panic!("There is no `{:?}` artifact!", artifact_path);
    } else if artifact_path.as_path().is_dir() {
      copy_all(artifact_path.as_path(), artifacts_dir.join(to).join(artifact_path.file_name().unwrap()), &ignore)?;
    } else if artifact_path.as_path().is_file() {
      copy_all(artifact_path.as_path(), artifacts_dir.join(to).as_path(), &ignore)?;
    }
  }
  
  println!("Build path: {}", build_path.to_str().expect("Can't convert `Path` to string!"));
  
  Ok(())
}

fn execute_pipeline(
  silent: bool,
  pipeline: &DescribedPipeline,
  build_dir: &Path,
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
      Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => a.execute(build_dir)?,
      Action::ProjectClean(pc_action) => pc_action.execute(build_dir)?,
      Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => a.execute(build_dir)?,
      Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => a.execute(build_dir)?,
      _ => {
        print!("{}", "not implemented! skip...".red());
        stdout().flush()?;
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

trait Execute {
  fn execute(&self, curr_dir: &Path) -> anyhow::Result<(bool, Vec<String>)>;
}

impl Execute for CustomCommand {
  fn execute(&self, curr_dir: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut output = vec![];
    
    if let Some(af_placeholder) = &self.af_placeholder {
      for artifact in &self.replace_af_with {
        let bash_c = self.bash_c.replace(af_placeholder, artifact);
        let bash_c_info = format!(r#"/bin/bash -c "{}""#, bash_c).green();
        
        let command_output = Command::new("/bin/bash")
          .current_dir(curr_dir)
          .arg("-c")
          .arg(&bash_c)
          .stdout(Stdio::piped())
          .spawn()
          .map_err(|e| anyhow!("Can't execute `{}` due to: {}", bash_c_info, e))?
          .wait_with_output()
          .map_err(|e| anyhow!("Can't wait for output `{}` due to: {}", bash_c_info, e))?;
        
        let out_strs = String::from_utf8_lossy_owned(command_output.stdout);
        if !out_strs.trim().is_empty() {
          output.push(format!("Executing `{}`:", bash_c_info));
          output.push(out_strs);
        }
        
        if !self.ignore_fails && !command_output.status.success() {
          return Ok((false, output))
        }
      }
    } else {
      let bash_c_info = format!(r#"/bin/bash -c "{}""#, self.bash_c.as_str()).green();
      
      let command_output = Command::new("/bin/bash")
        .current_dir(curr_dir)
        .arg("-c")
        .arg(self.bash_c.as_str())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("Can't execute `{}` due to: {}", bash_c_info, e))?
        .wait_with_output()
        .map_err(|e| anyhow!("Can't wait for output `{}` due to: {}", bash_c_info, e))?;
      
      let out_strs = String::from_utf8_lossy_owned(command_output.stdout);
      if !out_strs.trim().is_empty() {
        output.push(format!("Executing `{}`:", bash_c_info));
        output.push(out_strs);
      }
      
      if !self.ignore_fails && !command_output.status.success() {
        return Ok((false, output))
      }
    }
    
    Ok((true, output))
  }
}

impl Execute for ProjectCleanAction {
  fn execute(&self, curr_dir: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for cmd in &self.additional_commands {
      let (status, out) = cmd.execute(curr_dir)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}

impl Execute for BuildAction {
  fn execute(&self, curr_dir: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for cmd in &self.commands {
      let (status, out) = cmd.execute(curr_dir)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}

impl Execute for PackAction {
  fn execute(&self, curr_dir: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for cmd in &self.commands {
      let (status, out) = cmd.execute(curr_dir)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}


impl Execute for DeployAction {
  fn execute(&self, curr_dir: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for cmd in &self.commands {
      let (status, out) = cmd.execute(curr_dir)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
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
