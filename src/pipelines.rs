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
  traits::{EditExtended, Execute},
};
use crate::hmap;
use crate::i18n;
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
  /// Информация для проекта: должен ли пайплайн выполняться в определённой среде (например, в отдельных папках сборки).
  /// 
  /// Если зависит, то пайплайн будет выполняться в папках с указанным тегом сборки.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) exclusive_exec_tag: Option<String>,
}

impl DescribedPipeline {
  pub(crate) fn new_from_prompt(globals: &mut DeployerGlobalConfig) -> anyhow::Result<Self> {
    use inquire::Text;
    
    let short_name = Text::new(i18n::PIPELINE_SHORT_NAME).prompt()?;
    let version = Text::new(i18n::PIPELINE_VERSION).prompt()?;
    
    let info = PipelineInfo { short_name, version };
    
    let name = Text::new(i18n::PIPELINE_FULL_NAME).prompt()?;
    let desc = Text::new(i18n::PIPELINE_DESC).prompt()?;
    
    let tags: Vec<String> = tags_custom_type(i18n::PIPELINE_TAGS, None).prompt()?;
    
    let selected_actions_unordered = collect_multiple_actions(globals)?;
    let selected_actions_ordered = reorder_actions(selected_actions_unordered)?;
    
    let exclusive_exec_tag = Text::new(&format!("{} {}:", i18n::PIPELINE_SPECIFY_EXCL_TAG, i18n::OR_HIT_ESC)).prompt_skippable()?;
    
    let described_pipeline = DescribedPipeline {
      title: name,
      desc,
      info,
      tags,
      actions: selected_actions_ordered,
      default: None,
      exclusive_exec_tag,
    };
    
    Ok(described_pipeline)
  }
  
  pub(crate) fn edit_pipeline_from_prompt(&mut self, globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let actions = vec![
      i18n::EDIT_PIPELINE_ACTIONS,
      i18n::EDIT_TITLE,
      i18n::EDIT_DESC,
      i18n::EDIT_TAGS,
      i18n::EDIT_EXCL_TAG,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        i18n::EDIT_TITLE => self.title = inquire::Text::new(i18n::PIPELINE_FULL_NAME).with_initial_value(self.title.as_str()).prompt()?,
        i18n::EDIT_DESC => self.desc = inquire::Text::new(i18n::PIPELINE_DESC).with_initial_value(self.desc.as_str()).prompt()?,
        i18n::EDIT_TAGS => {
          let joined = self.tags.join(", ");
          self.tags = tags_custom_type(i18n::PIPELINE_TAGS, if joined.is_empty() { None } else { Some(joined.as_str()) }).prompt()?
        },
        i18n::EDIT_PIPELINE_ACTIONS => self.actions.edit_from_prompt(globals)?,
        i18n::EDIT_EXCL_TAG => self.exclusive_exec_tag = if self.exclusive_exec_tag.is_none() {
          inquire::Text::new(&format!("{} {}:", i18n::PIPELINE_SPECIFY_EXCL_TAG, i18n::OR_HIT_ESC)).prompt_skippable()?
        } else {
          inquire::Text::new(
            &format!("{} {}:", i18n::PIPELINE_SPECIFY_EXCL_TAG, i18n::OR_HIT_ESC)
          ).with_initial_value(self.exclusive_exec_tag.as_ref().unwrap()).prompt_skippable()?
        },
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
  println!("{}", i18n::PIPELINES_AVAILABLE);
  
  let mut pipelines = globals.pipelines_registry.values().collect::<Vec<_>>();
  pipelines.sort_by_key(|a| info2str_simple(&a.info));
  
  for pipeline in pipelines {
    let pipeline_info = format!("{}@{}", pipeline.info.short_name, pipeline.info.version);
    let pipeline_title = format!("[{}]", pipeline.title);
    let tags = if pipeline.tags.is_empty() { String::new() } else { format!(" ({}: {})", i18n::TAGS, pipeline.tags.join(", ").as_str().blue().italic()) };
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
    !inquire::Confirm::new(&i18n::PIPELINE_REG_ALREADY_HAVE.replace("{}", &info2str_simple(&described_pipeline.info))).prompt()?
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
  
  let reordered = ReorderableList::new(i18n::REORDER_PIPELINE_ACTIONS, k).prompt()?;
  
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
  
  const NEW_ACTION: &str = i18n::ACTION_SPECIFY_ANOTHER;
  
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
  
  let selected_action = Select::new(i18n::SELECT_ACTION_TO_ADD_TO, keys).prompt()?;
  
  if selected_action.as_str().eq(NEW_ACTION) {
    let mut action = new_action(globals, &NewActionArgs { from: None })?;
    let new_title = Text::new(i18n::PIPELINE_DESCRIBE_ACTION_IN).prompt()?;
    action.desc = format!(r#"{} `{}`.{}{}"#, i18n::GOT_FROM, action.title, if action.desc.is_empty() { "" } else { " " }, action.desc);
    action.title = new_title;
    
    return Ok(action)
  }
  
  let mut action = (*actions.get(&selected_action).unwrap()).clone();
  
  let new_title = Text::new(i18n::PIPELINE_DESCRIBE_ACTION_IN).prompt()?;
  action.desc = format!(r#"{} `{}`.{}{}"#, i18n::GOT_FROM, action.title, if action.desc.is_empty() { "" } else { " " }, action.desc);
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
  
  while Confirm::new(i18n::ACTION_ADD).with_default(first).prompt()? {
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
    println!("{}", i18n::NO_PIPELINES);
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
  
  let selected_pipeline = Select::new(i18n::PIPELINE_REGISTRY_CHOOSE_TO_REMOVE, keys).prompt()?;
  let pipeline = *pipelines.get(&selected_pipeline).unwrap();
  
  if !Confirm::new(i18n::ARE_YOU_SURE).prompt()? { return Ok(()) }
  
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
    let pipeline_json = serde_json::to_string_pretty(&pipeline).unwrap();
    println!("{}", pipeline_json);
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
  
  let reordered = ReorderableList::new(i18n::PIPELINES_REORDER, k).prompt()?;
  
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
  if *config == Default::default() { panic!("{}", i18n::CFG_INVALID); }
  
  let mut pipeline = if let Some(tag) = &args.tag {
    globals
      .pipelines_registry
      .get(tag)
      .ok_or_else(|| anyhow::anyhow!(i18n::NO_SUCH_PIPELINE))?
      .clone()
  } else if !globals.pipelines_registry.is_empty() {
    const NEW_PIPELINE: &str = i18n::PIPELINE_SPECIFY_ANOTHER;
    
    let mut ptags = hmap!();
    let mut tags = vec![];
    
    globals
      .pipelines_registry
      .iter()
      .map(|(k, v)| (format!("`{}` - {}", k.blue().bold(), v.title.green().bold()), v))
      .for_each(|(t, p)| { tags.push(t.clone()); ptags.insert(t, p); });
    tags.push(NEW_PIPELINE.to_string());
    
    let selected = inquire::Select::new(i18n::PIPELINE_SELECT_FOR_PROJECT, tags).prompt()?;
    
    if selected.as_str() == NEW_PIPELINE {
      DescribedPipeline::new_from_prompt(globals)?
    } else {
      let pipeline = ptags
        .get(&selected)
        .ok_or(anyhow::anyhow!(i18n::NO_SUCH_PIPELINE))?;
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
    inquire::Text::new(i18n::PIPELINE_SHORT_NAME_FOR_PROJECT).prompt()?
  };
  
  pipeline.desc = format!(r#"{} `{}`.{}{}"#, i18n::GOT_FROM, pipeline.title, if pipeline.desc.is_empty() { "" } else { " " }, pipeline.desc);
  pipeline.title = short_name.clone();
  
  if specify_short_name(config, &mut pipeline.title).is_err() { return Ok(()) };
  
  if let Some(old_default) = config.pipelines.iter_mut().find(|p| p.default.is_some_and(|v| v)) {
    if inquire::Confirm::new(&i18n::PIPELINE_NEW_DEFAULT_REPLACE.replace("{}", old_default.title.as_str())).prompt()? {
      old_default.default = None;
      pipeline.default = Some(true);
    }
  } else if inquire::Confirm::new(i18n::PIPELINE_NEW_DEFAULT).prompt()? {
    pipeline.default = Some(true);
  }
  
  remove_old_pipeline(config, &short_name);
  config.pipelines.push(pipeline);
  
  if config.pipelines.len() >= 2 {
    config.pipelines = reorder_pipelines_in_project(config.pipelines.clone())?;
  }
  
  println!("{}", i18n::PIPELINE_DEFAULT_SET);
  
  Ok(())
}

fn specify_short_name(
  config: &mut DeployerProjectOptions,
  short_name: &mut String,
) -> anyhow::Result<()> {
  while
    config.pipelines.iter().any(|p| p.title.as_str() == short_name) &&
    !inquire::Confirm::new(&i18n::PIPELINE_SHORT_NAME_FOR_PROJECT_OVERRIDE.replace("{}", short_name.as_str())).prompt()?
  {
    *short_name = inquire::Text::new(&format!("{} {}:", i18n::PIPELINE_SHORT_NAME_FOR_PROJECT, i18n::HIT_ESC))
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
        let s = i18n::PIPELINE_EDIT.replace("{1}", &c.title).replace("{2}", &info2str_simple(&c.info));
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::REORDER.to_string(), i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::SELECT_PIPELINE_TO_CHANGE, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::REORDER => self.reorder(opts)?,
          i18n::ADD => self.add_item(opts)?,
          i18n::REMOVE => self.remove_item(opts)?,
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
      let key = i18n::PIPELINE.replace("{1}", &selected.title).replace("{2}", &info2str_simple(&selected.info));
      k.push(key.clone());
      h.insert(key, selected);
    }
    
    let reordered = ReorderableList::new(i18n::PIPELINE_REORDER_ACTIONS, k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self, opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    use inquire::Select;
    
    const USE_ANOTHER: &str = i18n::PIPELINE_SPECIFY_ANOTHER;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for pipeline in opts.pipelines_registry.values() {
      let key = i18n::PIPELINE.replace("{1}", &pipeline.title).replace("{2}", &info2str_simple(&pipeline.info));
      k.push(key.clone());
      h.insert(key, pipeline);
    }
    
    k.push(USE_ANOTHER.to_string());
    
    let selected = Select::new(i18n::PIPELINE_CHOOSE_TO_ADD, k).prompt()?;
    
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
      let s = i18n::PIPELINE_REMOVE.replace("{1}", &c.title).replace("{2}", &info2str_simple(&c.info));
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::PIPELINE_CHOOSE_TO_REMOVE, cs.clone()).prompt()?;
    
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
  
  if !env.silent_build { println!("{}", i18n::STARTING_PIPELINE.replace("{}", &pipeline.title)); }
  build_log(&log_file, &[format!("Starting the `{}` Pipeline...", pipeline.title)])?;
  
  let mut cntr = 1usize;
  let total = pipeline.actions.len();
  for action in &pipeline.actions {
    if !env.silent_build {
      if !env.no_pipe {
        print!("[{}/{}] {} `{}`...", cntr, total, i18n::STARTING_ACTION, action.title.blue().italic());
      } else {
        println!("[{}/{}] {} `{}`...", cntr, total, i18n::STARTING_ACTION, action.title.blue().italic());
      }
      build_log(&log_file, &[format!("[{}/{}] {} `{}`...", cntr, total, i18n::STARTING_ACTION, action.title)])?;
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
        
        (true, vec![i18n::ARTIFACTS_ENPLACED.into()])
      },
      Action::Interrupt => {
        println!();
        inquire::Confirm::new(i18n::INTERRUPT).with_default(true).prompt()?;
        (true, vec![])
      },
      
    };
    
    let status_str = match status {
      true => i18n::DONE.to_string(),
      false => i18n::GOT_ERROR.red().bold().to_string(),
    };
    
    let elapsed = now.elapsed();
    if !env.no_pipe { build_log(&log_file, &output)?; }
    build_log(&log_file, &[
      format!(
        "[{}/{}] {} -{} ({:.2?}).",
        cntr,
        total,
        i18n::STARTING_ACTION.replace("{}", &action.title),
        if status { i18n::DONE } else { i18n::GOT_ERROR },
        elapsed,
      ),
    ])?;
    
    if !env.silent_build {
      if !env.no_pipe {
        println!("{} ({}).", status_str, format!("{:.2?}", elapsed).green());
        for line in output { println!("{}", line); }
      } else {
        println!("[{}/{}] {} -{} ({}).", cntr, total, i18n::STARTING_ACTION.replace("{}", &action.title.blue().italic()), status_str, format!("{:.2?}", elapsed).green());
      }
    }
    
    cntr += 1;
    
    if !status { return Ok(()) }
  }
  
  let canonicalized = env.build_dir.canonicalize()?;
  let canonicalized = canonicalized.to_str().expect("Can't convert `Path` to string!");
  if !env.silent_build { println!("{}: {}", i18n::BUILD_PATH, canonicalized); }
  build_log(&log_file, &[format!("{}: {}", i18n::BUILD_PATH, canonicalized)])?;
  
  Ok(())
}
