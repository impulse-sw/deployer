use crate::actions::{ProgrammingLanguage, select_programming_languages, TargetDescription, collect_target};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};

fn collect_targets() -> anyhow::Result<Vec<TargetDescription>> {
  let mut v = vec![];
  
  while inquire::Confirm::new("Add new build target? (y/n)").prompt()? {
    v.push(collect_target()?);
  }
  
  Ok(v)
}

fn collect_artifact() -> anyhow::Result<String> {
  Ok(inquire::Text::new("Enter the artifact's relative path:").prompt()?)
}

fn collect_artifacts() -> anyhow::Result<Vec<String>> {
  let mut v = vec![];
  
  while inquire::Confirm::new("Add new build/deploy artifact? (y/n)").prompt()? {
    v.push(collect_artifact()?);
  }
  
  Ok(v)
}

pub(crate) fn init(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
) -> anyhow::Result<()> {
  use inquire::Text;
  
  let curr_dir = std::env::current_dir().expect("Can't get current dir!").to_str().expect("Can't convert current dir's path to string!").to_owned();
  if !globals.projects.contains(&curr_dir) { globals.projects.push(curr_dir.to_owned()); }
  
  config.project_name = Text::new("Enter the project's name (or hit `esc`):").prompt_skippable()?;
  println!("Please, specify the project's programming languages to setup default cache folders.");
  config.langs = select_programming_languages()?;
  for lang in &config.langs {
    match lang {
      ProgrammingLanguage::Rust => config.cache_files.extend_from_slice(&["Cargo.lock".to_string(), "target".to_string()]),
      ProgrammingLanguage::Go => config.cache_files.extend_from_slice(&["go.sum".to_string(), "vendor".to_string()]),
      ProgrammingLanguage::Python => config.cache_files.extend_from_slice(&["__pycache__".to_string(), "dist".to_string()]),
      ProgrammingLanguage::C | ProgrammingLanguage::Cpp => config.cache_files.extend_from_slice(&["CMakeFiles".to_string(), "CMakeCache.txt".to_string()]),
      _ => {},
    }
  }
  
  config.deploy_toolkit = Text::new("Specify your deploy toolkit (`docker`, `docker-compose`, `podman`, `k8s`, etc.) (or hit `esc`):").prompt_skippable()?;
  config.targets = collect_targets()?;
  config.artifacts = collect_artifacts()?;
  
  println!("Setup is completed. Don't forget to assign at least one Pipeline to the project to build/deploy!");
  
  Ok(())
}
