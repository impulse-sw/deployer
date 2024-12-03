use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process::exit;

use crate::actions::{DescribedAction, DependencyInfo, new_action};
use crate::cmd::{NewActionArgs, NewPipelineArgs, CatPipelineArgs, WithPipelineArgs};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::hmap;
use crate::rw::read_checked;
use crate::utils::{info2str_simple, info2str, str2info, tags_custom_type};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DescribedPipeline {
  pub(crate) title: String,
  pub(crate) desc: String,
  /// Короткое имя и версия
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) info: PipelineInfo,
  /// Список меток для фильтрации действий при выборе из реестра
  pub(crate) tags: Vec<String>,
  pub(crate) actions: Vec<DescribedAction>,
}

pub(crate) type PipelineInfo = DependencyInfo;

/// Перечисляет все доступные пайплайны.
pub(crate) fn list_pipelines(
  globals: &DeployerGlobalConfig,
) -> anyhow::Result<()> {
  println!("Available Pipelines in Deployer's Registry:");
  
  let mut pipelines = globals.pipelines_registry.values().collect::<Vec<_>>();
  pipelines.sort_by_key(|a| info2str_simple(&a.info));
  
  for pipeline in pipelines {
    let pipeline_info = format!("{}@{}", pipeline.info.short_name, pipeline.info.version);
    let pipeline_title = format!("[{}]", pipeline.title);
    println!("• {} {} (tags: {})", pipeline_info.blue().bold(), pipeline_title.green().bold(), pipeline.tags.join(", ").as_str().blue().italic());
    println!("\t> {}", pipeline.desc.green().italic());
  }
  
  Ok(())
}

pub(crate) fn new_pipeline(
  globals: &mut DeployerGlobalConfig,
  args: &NewPipelineArgs,
) -> anyhow::Result<()> {
  use inquire::{Text, Confirm};
  
  let pipelines = &mut globals.pipelines_registry;
  
  if let Some(from_file) = &args.from {
    let pipeline = read_checked::<DescribedPipeline>(from_file).map_err(|e| {
      panic!("Can't read provided Pipeline file due to: {}", e);
    }).unwrap();
    pipelines.insert(info2str_simple(&pipeline.info), pipeline);
    return Ok(())
  }
  
  let short_name = Text::new("Write the Pipeline's short name:").prompt()?;
  let version = Text::new("Specify the Pipeline's version:").prompt()?;
  
  let info = PipelineInfo { short_name, version };
  
  if
    pipelines.contains_key(&info2str_simple(&info)) &&
    !Confirm::new(&format!("Pipelines Registry already have `{}` Pipeline. Do you want to override it? (y/n)", info2str_simple(&info))).prompt()?
  {
    return Ok(())
  }
  
  let name = Text::new("Write the Pipeline's full name:").prompt()?;
  let desc = Text::new("Write the Pipeline's description:").prompt()?;
  
  let tags: Vec<String> = tags_custom_type("Write Pipeline's tags, if any:").prompt()?;
  
  let selected_actions_unordered = collect_multiple_actions(globals)?;
  let selected_actions_ordered = reorder_actions(selected_actions_unordered)?;
  
  let described_pipeline = DescribedPipeline {
    title: name,
    desc,
    info,
    tags,
    actions: selected_actions_ordered,
  };
  
  let pipelines = &mut globals.pipelines_registry;
  pipelines.insert(info2str_simple(&described_pipeline.info), described_pipeline);
  
  Ok(())
}

fn reorder_actions(
  selected_actions_unordered: Vec<DescribedAction>,
) -> anyhow::Result<Vec<DescribedAction>> {
  use inquire::ReorderableList;
  
  let mut h = hmap!();
  let mut k = vec![];
  
  for selected_action in selected_actions_unordered {
    let key = format!("{} - {}", info2str_simple(&selected_action.info), selected_action.title);
    k.push(key.clone());
    h.insert(key, selected_action);
  }
  
  let reordered = ReorderableList::new("Reorder Pipeline's Actions:", k).prompt()?;
  
  let mut selected_actions_ordered = vec![];
  for key in reordered {
    selected_actions_ordered.push((*h.get(&key).unwrap()).clone());
  }
  
  Ok(selected_actions_ordered)
}

fn select_action(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<DescribedAction> {
  use inquire::{Select, Text};
  
  const NEW_ACTION: &str = "Create new Action";
  
  let (actions, keys) = {
    let mut h = hmap!();
    let mut k = vec![];
    
    for key in globals.actions_registry.keys() {
      let action = globals.actions_registry.get(key).unwrap();
      let new_key = format!("{} - {}", info2str_simple(&action.info), action.title);
      h.insert(new_key.clone(), action);
      k.push(new_key);
    }
    
    k.sort();
    k.push(NEW_ACTION.to_string());
    
    (h, k)
  };
  
  let selected_action = Select::new("Select Action for adding to Pipeline:", keys).prompt()?;
  
  if selected_action.as_str().eq(NEW_ACTION) {
    let mut action = new_action(globals, &NewActionArgs { from: None })?;
    let new_title = Text::new("Describe this Action inside your Pipeline:").prompt()?;
    action.desc = format!(r#"Got from `{}`. {}"#, action.title, action.desc);
    action.title = new_title;
    
    return Ok(action)
  }
  
  let mut action = (*actions.get(&selected_action).unwrap()).clone();
  
  let new_title = Text::new("Describe this Action inside your Pipeline:").prompt()?;
  action.desc = format!(r#"Got from `{}`. {}"#, action.title, action.desc);
  action.title = new_title;
  
  Ok(action)
}

// Helper function to collect multiple custom commands
fn collect_multiple_actions(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<Vec<DescribedAction>> {
  use inquire::Confirm;
  
  let mut actions = Vec::new();
  while Confirm::new("Add Action? (y/n)").prompt()? {
    actions.push(select_action(globals)?);
  }
  Ok(actions)
}

pub(crate) fn remove_pipeline(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<()> {
  use inquire::{Select, Confirm};
  
  if globals.pipelines_registry.is_empty() {
    println!("There is no Pipelines in Registry.");
    return Ok(())
  }
  
  let (pipelines, keys) = {
    let mut h = hmap!();
    let mut k = vec![];
    
    for key in globals.pipelines_registry.keys() {
      let pipeline = globals.pipelines_registry.get(key).unwrap();
      let new_key = format!("{} - {}", info2str_simple(&pipeline.info), pipeline.title);
      h.insert(new_key.clone(), pipeline);
      k.push(new_key);
    }
    
    k.sort();
    
    (h, k)
  };
  
  let selected_pipeline = Select::new("Select Pipeline for removing from Pipeline's Registry:", keys).prompt()?;
  let pipeline = *pipelines.get(&selected_pipeline).unwrap();
  
  if !Confirm::new("Are you sure? (y/n)").prompt()? { return Ok(()) }
  
  globals.pipelines_registry.remove(&info2str_simple(&pipeline.info));
  
  Ok(())
}

pub(crate) fn cat_pipeline(
  globals: &DeployerGlobalConfig,
  args: &CatPipelineArgs,
) -> anyhow::Result<()> {
  let pipeline = match globals.pipelines_registry.get(&args.pipeline_short_info_and_version) {
    None => exit(1),
    Some(pipeline) => pipeline,
  };
  
  let pipeline_yaml = serde_yaml::to_string(&pipeline).unwrap();
  println!("{}", pipeline_yaml);
  
  Ok(())
}

pub(crate) fn cat_project_pipelines(
  config: &DeployerProjectOptions,
) -> anyhow::Result<()> {
  for pipeline in &config.pipelines {
    let pipeline_yaml = serde_yaml::to_string(&pipeline).unwrap();
    println!("{}", pipeline_yaml);
  }
  
  Ok(())
}

fn reorder_pipelines_in_project(
  pipelines_unordered: Vec<DescribedPipeline>,
) -> anyhow::Result<Vec<DescribedPipeline>> {
  use inquire::ReorderableList;
  
  let mut h = hmap!();
  let mut k = vec![];
  
  for pipeline in pipelines_unordered {
    let key = format!("{} - {}", info2str_simple(&pipeline.info), pipeline.title);
    k.push(key.clone());
    h.insert(key, pipeline);
  }
  
  println!("The `build` action without specifying Pipeline's short name will execute all Pipelines. Make sure that your Pipelines are sorted the way you need them.");
  let reordered = ReorderableList::new("Reorder Pipelines inside your project:", k).prompt()?;
  
  let mut pipelines_ordered = vec![];
  for key in reordered {
    pipelines_ordered.push((*h.get(&key).unwrap()).clone());
  }
  
  Ok(pipelines_ordered)
}

pub(crate) fn assign_pipeline_to_project(
  globals: &DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  args: &WithPipelineArgs,
) -> anyhow::Result<()> {
  let mut pipeline = globals
    .pipelines_registry
    .get(&args.tag)
    .ok_or_else(|| anyhow::anyhow!("There is no such Pipeline in Registry. See available Pipelines with `deployer ls pipelines`."))?
    .clone();
  
  for action in &mut pipeline.actions {
    *action = action.prompt_setup_for_project(&config.langs, &config.deploy_toolkit, &config.targets, &config.artifacts)?;
  }
  
  let short_name = if let Some(short_name) = args.r#as.as_ref() {
    short_name.to_owned()
  } else {
    inquire::Text::new("Write the Pipeline's short name (only for this project):").prompt()?
  };
  
  pipeline.desc = format!(r#"Got from `{}`. {}"#, pipeline.title, pipeline.desc);
  pipeline.title = short_name.clone();
  
  if specify_short_name(config, &mut pipeline.title).is_err() { return Ok(()) };
  
  remove_old_pipeline(config, &short_name);
  config.pipelines.push(pipeline);
  
  if config.pipelines.len() >= 2 {
    config.pipelines = reorder_pipelines_in_project(config.pipelines.clone())?;
  }
  
  println!("Pipeline is successfully set up for this project.");
  
  Ok(())
}

fn specify_short_name(
  config: &mut DeployerProjectOptions,
  short_name: &mut String,
) -> anyhow::Result<()> {
  while
    config.pipelines.iter().any(|p| p.title.as_str() == short_name) &&
    !inquire::Confirm::new(&format!("Do you want to overwrite an existing pipeline `{}` for this project? (y/n)", short_name.as_str())).prompt()?
  {
    *short_name = inquire::Text::new("Write the Pipeline's short name (only for this project) (or hit `esc` to exit):")
      .prompt_skippable()?
      .ok_or_else(|| anyhow::anyhow!("Hitted Escape."))?;
  }
  
  Ok(())
}

fn remove_old_pipeline(
  config: &mut DeployerProjectOptions,
  short_name: &str,
) {
  if let Some(i) = config.pipelines.iter().position(|p| p.title.as_str() == short_name) {
    config.pipelines.remove(i);
  }
}
