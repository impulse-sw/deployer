use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process::exit;

use crate::actions::{DescribedAction, Action, new_action};
use crate::build::enplace_artifacts;
use crate::cmd::{NewActionArgs, NewPipelineArgs, CatPipelineArgs, WithPipelineArgs};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::entities::{
  environment::BuildEnvironment,
  info::{PipelineInfo, info2str_simple, info2str, str2info},
  targets::TargetDescription,
  traits::{EditExtended, Execute},
};
use crate::hmap;
use crate::rw::{read_checked, generate_build_log_filepath, build_log};
use crate::utils::tags_custom_type;
use crate::ARTIFACTS_DIR;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) struct DescribedPipeline {
  /// Заголовок Пайплайна.
  pub(crate) title: String,
  /// Описание Пайплайна.
  pub(crate) desc: String,
  /// Короткое имя и версия.
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) info: PipelineInfo,
  /// Список меток для фильтрации Действий при выборе из Реестра.
  pub(crate) tags: Vec<String>,
  pub(crate) actions: Vec<DescribedAction>,
  /// Информация для проекта: запускать ли Пайплайн по умолчанию.
  /// 
  /// Если не установлен, считается как `false`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) default: Option<bool>,
  /// Информация для проекта: зависит ли пайплайн от какого-либо таргета.
  /// 
  /// Если зависит, то пайплайн будет выполняться в зависимости от 
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) target_dependable: Option<TargetDescription>,
}

impl DescribedPipeline {
  pub(crate) fn new_from_prompt(globals: &mut DeployerGlobalConfig) -> anyhow::Result<Self> {
    use inquire::Text;
    
    let short_name = Text::new("Write the Pipeline's short name:").prompt()?;
    let version = Text::new("Specify the Pipeline's version:").prompt()?;
    
    let info = PipelineInfo { short_name, version };
    
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
      default: None,
      target_dependable: None,
    };
    
    Ok(described_pipeline)
  }
  
  pub(crate) fn edit_pipeline_from_prompt(&mut self, globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let actions = vec![
      "Edit title",
      "Edit description",
      "Edit tags",
      "Edit Pipeline's Actions",
    ];
    
    while let Some(action) = inquire::Select::new(
      "Select an edit action (hit `esc` when done):",
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        "Edit title" => self.title = inquire::Text::new("Write the Action's full name:").prompt()?,
        "Edit description" => self.desc = inquire::Text::new("Write the Action's description:").prompt()?,
        "Edit tags" => self.tags = tags_custom_type("Write Action's tags, if any:").prompt()?,
        "Edit Pipeline's Actions" => self.actions.edit_from_prompt(globals)?,
        _ => {},
      }
    }
    
    Ok(())
  }
}

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
    let tags = if pipeline.tags.is_empty() { String::new() } else { format!(" (tags: {})", pipeline.tags.join(", ").as_str().blue().italic()) };
    println!("• {} {}{}", pipeline_info.blue().bold(), pipeline_title.green().bold(), tags);
    if !pipeline.desc.is_empty() { println!("\t> {}", pipeline.desc.green().italic()); }
  }
  
  Ok(())
}

/// Создаёт новый пайплайн.
pub(crate) fn new_pipeline(
  globals: &mut DeployerGlobalConfig,
  args: &NewPipelineArgs,
) -> anyhow::Result<()> {
  if let Some(from_file) = &args.from {
    let pipeline = read_checked::<DescribedPipeline>(from_file).map_err(|e| {
      panic!("Can't read provided Pipeline file due to: {}", e);
    }).unwrap();
    globals.pipelines_registry.insert(info2str_simple(&pipeline.info), pipeline);
    return Ok(())
  }
  
  let described_pipeline = DescribedPipeline::new_from_prompt(globals)?;
  
  if
    globals.pipelines_registry.contains_key(&info2str_simple(&described_pipeline.info)) &&
    !inquire::Confirm::new(&format!(
      "Pipelines Registry already have `{}` Pipeline. Do you want to override it? (y/n)", info2str_simple(&described_pipeline.info))
    ).prompt()?
  {
    return Ok(())
  }
  
  globals.pipelines_registry.insert(info2str_simple(&described_pipeline.info), described_pipeline);
  
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
  let mut first = true;
  
  while Confirm::new("Add Action?").with_default(first).prompt()? {
    actions.push(select_action(globals)?);
    first = false;
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
  
  let pipeline_json = serde_json::to_string_pretty(&pipeline).unwrap();
  println!("{}", pipeline_json);
  
  Ok(())
}

pub(crate) fn cat_project_pipelines(
  config: &DeployerProjectOptions,
) -> anyhow::Result<()> {
  for pipeline in &config.pipelines {
    let pipeline_yaml = serde_json::to_string_pretty(&pipeline).unwrap();
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
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  args: &WithPipelineArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("Config is invalid! Reinit the project."); }
  
  let mut pipeline = if let Some(tag) = &args.tag {
    globals
      .pipelines_registry
      .get(tag)
      .ok_or_else(|| anyhow::anyhow!("There is no such Pipeline in Registry. See available Pipelines with `deployer ls pipelines`."))?
      .clone()
  } else if !globals.pipelines_registry.is_empty() {
    const NEW_PIPELINE: &str = "· Specify another Pipeline";
    
    let mut ptags = hmap!();
    let mut tags = vec![];
    
    globals
      .pipelines_registry
      .iter()
      .map(|(k, v)| (format!("`{}` - {}", k.blue().bold(), v.title.green().bold()), v))
      .for_each(|(t, p)| { tags.push(t.clone()); ptags.insert(t, p); });
    tags.push(NEW_PIPELINE.to_string());
    
    let selected = inquire::Select::new("Select the Pipeline for this project:", tags).prompt()?;
    
    if selected.as_str() == NEW_PIPELINE {
      DescribedPipeline::new_from_prompt(globals)?
    } else {
      let pipeline = ptags
        .get(&selected)
        .ok_or(anyhow::anyhow!("There is no such Pipeline in Registry. See available Pipelines with `deployer ls pipelines`."))?;
      (*pipeline).clone()
    }
  } else {
    DescribedPipeline::new_from_prompt(globals)?
  };
  
  for action in &mut pipeline.actions {
    *action = action.prompt_setup_for_project(&config.langs, &config.deploy_toolkit, &config.targets, &config.variables, &config.artifacts)?;
  }
  
  let short_name = if let Some(short_name) = args.r#as.as_ref() {
    short_name.to_owned()
  } else {
    inquire::Text::new("Write the Pipeline's short name (only for this project):").prompt()?
  };
  
  pipeline.desc = format!(r#"Got from `{}`. {}"#, pipeline.title, pipeline.desc);
  pipeline.title = short_name.clone();
  
  if specify_short_name(config, &mut pipeline.title).is_err() { return Ok(()) };
  
  if let Some(old_default) = config.pipelines.iter_mut().find(|p| p.default.is_some_and(|v| v)) {
    if inquire::Confirm::new(&format!(
      "Pipeline `{}` is already set by default. Set this Pipeline running by default instead?",
      old_default.title.as_str()
    )).prompt()? {
      old_default.default = None;
      pipeline.default = Some(true);
    }
  } else if inquire::Confirm::new("Set this Pipeline running by default? (y/n)").prompt()? {
    pipeline.default = Some(true);
  }
  
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

pub(crate) fn edit_pipeline(
  globals: &mut DeployerGlobalConfig,
  args: &CatPipelineArgs,
) -> anyhow::Result<()> {
  let mut pipeline = match globals.pipelines_registry.contains_key(&args.pipeline_short_info_and_version) {
    false => panic!("There is no such Pipeline!"),
    true => {
      let pipeline = globals.pipelines_registry.get(&args.pipeline_short_info_and_version).unwrap().clone();
      globals.pipelines_registry.remove(&args.pipeline_short_info_and_version);
      pipeline
    },
  };
  
  pipeline.edit_pipeline_from_prompt(globals)?;
  globals.pipelines_registry.insert(info2str_simple(&pipeline.info), pipeline);
  
  Ok(())
}

impl EditExtended<DeployerGlobalConfig> for Vec<DescribedPipeline> {
  fn edit_from_prompt(&mut self, opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("Edit Pipeline `{}` - `{}`", c.title, info2str_simple(&c.info));
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&["Reorder Pipelines".to_string(), "Add Pipeline".to_string(), "Remove Pipeline".to_string()]);
      
      if let Some(action) = inquire::Select::new("Select a concrete Pipeline to change (hit `esc` when done):", cs).prompt_skippable()? {
        match action.as_str() {
          "Reorder Pipelines" => self.reorder(opts)?,
          "Add Pipeline" => self.add_item(opts)?,
          "Remove Pipeline" => self.remove_item(opts)?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_pipeline_from_prompt(opts)?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self, _opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    use inquire::ReorderableList;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for selected in self.iter() {
      let key = format!("Pipeline `{}` - `{}`", selected.title, info2str_simple(&selected.info));
      k.push(key.clone());
      h.insert(key, selected);
    }
    
    let reordered = ReorderableList::new("Reorder Pipeline's Actions:", k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self, opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    use inquire::Select;
    
    const USE_ANOTHER: &str = "· Specify another Pipeline";
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for pipeline in opts.pipelines_registry.values() {
      let key = format!("Pipeline `{}` - `{}`", pipeline.title, info2str_simple(&pipeline.info));
      k.push(key.clone());
      h.insert(key, pipeline);
    }
    
    k.push(USE_ANOTHER.to_string());
    
    let selected = Select::new("Choose a Pipeline to add:", k).prompt()?;
    
    if selected.as_str() == USE_ANOTHER {
      if let Ok(pipeline) = DescribedPipeline::new_from_prompt(opts) {
        self.push(pipeline);
      }
    } else {
      self.push((**h.get(&selected).ok_or(anyhow::anyhow!("Can't get specified Pipeline!"))?).clone());
    }
    Ok(())
  }
  
  fn remove_item(&mut self, _opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("Remove Pipeline `{}` - `{}`", c.title, info2str_simple(&c.info));
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new("Select an Pipeline to remove:", cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    
    Ok(())
  }
}

pub(crate) fn execute_pipeline(
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
    
    let elapsed = now.elapsed();
    if !env.no_pipe { build_log(&log_file, &output)?; }
    build_log(&log_file, &[
      format!("[{}/{}] `{}` Action - {} ({:.2?}).", cntr, total, action.title, if status { "done" } else { "got an error!" }, elapsed),
    ])?;
    
    if !env.silent_build {
      if !env.no_pipe {
        println!("{} ({}).", status_str, format!("{:.2?}", elapsed).green());
        for line in output { println!("{}", line); }
      } else {
        println!("[{}/{}] `{}` Action - {} ({}).", cntr, total, action.title.blue().italic(), status_str, format!("{:.2?}", elapsed).green());
      }
    }
    
    cntr += 1;
    
    if !status { return Ok(()) }
  }
  
  let canonicalized = env.build_dir.canonicalize()?;
  let canonicalized = canonicalized.to_str().expect("Can't convert `Path` to string!");
  if !env.silent_build { println!("Build path: {}", canonicalized); }
  build_log(&log_file, &[format!("Build path: {}", canonicalized)])?;
  
  Ok(())
}
