use colored::Colorize;

use crate::entities::programming_languages::{ProgrammingLanguage, specify_programming_languages};
use crate::configs::{DeployerProjectOptions, DeployerGlobalConfig};
use crate::entities::{
  targets::TargetDescription,
  traits::{Edit, EditExtended},
  variables::Variable,
};
use crate::hmap;
use crate::i18n;

impl DeployerProjectOptions {
  pub(crate) fn init_from_prompt(&mut self, curr_dir: String) -> anyhow::Result<()> {
    use inquire::Text;
    
    #[cfg(unix)]
    let curr_dir = curr_dir.split('/').last().unwrap();
    let project_name_proposal = if self.project_name.is_empty() {
      curr_dir.to_owned()
    } else {
      self.project_name.to_owned()
    };
    self.project_name = Text::new(i18n::PROJECT_NAME).with_initial_value(project_name_proposal.as_str()).prompt()?;
    
    self.cache_files.push(".git".to_string());
    println!("{}", i18n::PROJECT_SPECIFY_PLS);
    self.langs = specify_programming_languages()?;
    for lang in &self.langs {
      match lang {
        ProgrammingLanguage::Rust => self.cache_files.extend_from_slice(&["Cargo.lock".to_string(), "target".to_string()]),
        ProgrammingLanguage::Go => self.cache_files.extend_from_slice(&["go.sum".to_string(), "vendor".to_string()]),
        ProgrammingLanguage::Python => self.cache_files.extend_from_slice(&["__pycache__".to_string(), "dist".to_string()]),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => self.cache_files.extend_from_slice(&["CMakeFiles".to_string(), "CMakeCache.txt".to_string()]),
        _ => {},
      }
    }
    
    self.deploy_toolkit = Text::new(&format!("{} {}:", i18n::PROJECT_DEPL_TOOLKIT, i18n::OR_HIT_ESC)).prompt_skippable()?;
    self.targets = collect_targets()?;
    self.variables = collect_variables()?;
    self.artifacts = collect_artifacts()?;
    self.inplace_artifacts_into_project_root = collect_af_inplacements(&self.artifacts)?;
    
    Ok(())
  }
  
  pub(crate) fn edit_project_from_prompt(&mut self, globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let actions = vec![
      i18n::EDIT_PROJECT_PIPELINES,
      i18n::EDIT_PROJECT_NAME,
      i18n::EDIT_PROJECT_REASSIGN,
      i18n::EDIT_CACHE,
      i18n::EDIT_PLS,
      i18n::EDIT_TARGETS,
      i18n::EDIT_DEPL_TOOLKIT,
      i18n::EDIT_PROJECT_VARS,
      i18n::EDIT_ARTIFACTS,
      i18n::EDIT_AF_INPLACE,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        i18n::EDIT_PROJECT_NAME => self.project_name = inquire::Text::new(i18n::PROJECT_NAME).prompt()?,
        i18n::EDIT_CACHE => self.cache_files.edit_from_prompt()?,
        i18n::EDIT_PLS => self.langs.edit_from_prompt()?,
        i18n::EDIT_TARGETS => self.targets.edit_from_prompt()?,
        i18n::EDIT_DEPL_TOOLKIT => self.deploy_toolkit = inquire::Text::new(
          &format!("{} {}:", i18n::DEPL_TOOLKIT, i18n::OR_HIT_ESC)
        ).prompt_skippable()?,
        i18n::EDIT_PROJECT_VARS => self.variables.edit_from_prompt()?,
        i18n::EDIT_ARTIFACTS => self.artifacts.edit_from_prompt()?,
        i18n::EDIT_AF_INPLACE => self.inplace_artifacts_into_project_root.edit_from_prompt(&mut self.artifacts)?,
        i18n::EDIT_PROJECT_PIPELINES => self.pipelines.edit_from_prompt(globals)?,
        i18n::EDIT_PROJECT_REASSIGN => for pipeline in &mut self.pipelines {
          for action in &mut pipeline.actions {
            *action = action.prompt_setup_for_project(&self.langs, &self.deploy_toolkit, &self.targets, &self.variables, &self.artifacts)?;
          }
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

impl Edit for Vec<String> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::ENTITY, c.as_str());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(inquire::Text::new(i18n::NEW_VALUE).prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("{} `{}`", i18n::ENTITY, c.as_str());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::VALUE_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

impl EditExtended<Vec<String>> for Vec<(String, String)> {
  fn edit_from_prompt(&mut self, opts: &mut Vec<String>) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}` -> `{}`", i18n::INPLACEMENT, c.0, c.1);
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item(opts)?,
          i18n::REMOVE => self.remove_item(opts)?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self, _opts: &mut Vec<String>) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self, opts: &mut Vec<String>) -> anyhow::Result<()> {
    self.push({
      let from = inquire::Select::new("Select project's artifact:", opts.to_owned()).prompt()?;
      let to = inquire::Text::new("Enter relative path of artifact inplacement (inside `artifacts` subfolder):").prompt()?;
      
      (from, to)
    });
    Ok(())
  }
  
  fn remove_item(&mut self, _opts: &mut Vec<String>) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("{} `{}` -> `{}`", i18n::INPLACEMENT, c.0, c.1);
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new("Select an inplacement to remove:", cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

impl Edit for Vec<TargetDescription> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("Edit target `{}`", c.to_string().green());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new("Select a concrete target to change (hit `esc` when done):", cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_target_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(TargetDescription::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("Target `{}`", c.to_string().green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new("Select a target to remove:", cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

fn collect_targets() -> anyhow::Result<Vec<TargetDescription>> {
  let mut v = vec![];
  let mut first = true;
  
  while inquire::Confirm::new("Add new build target?").with_default(first).prompt()? {
    v.push(TargetDescription::new_from_prompt()?);
    first = false;
  }
  
  Ok(v)
}

fn collect_artifact() -> anyhow::Result<String> {
  Ok(inquire::Text::new("Enter the artifact's relative path:").prompt()?)
}

fn collect_artifacts() -> anyhow::Result<Vec<String>> {
  let mut v = vec![];
  let mut first = true;
  
  while inquire::Confirm::new("Add new build/deploy artifact?").with_default(first).prompt()? {
    v.push(collect_artifact()?);
    first = false;
  }
  
  Ok(v)
}

fn collect_variables() -> anyhow::Result<Vec<Variable>> {
  let mut v = vec![];
  let mut first = true;
  
  while inquire::Confirm::new("Add new project-related variable or secret?").with_default(first).prompt()? {
    v.push(Variable::new_from_prompt()?);
    first = false;
  }
  
  Ok(v)
}

fn collect_af_inplacements(artifacts: &[String]) -> anyhow::Result<Vec<(String, String)>> {
  use inquire::{Confirm, Select, Text};
  
  const FIRST_PROMPT: &str = "Do you want to create artifact inplacement from build directory to your project's location (inside `artifacts` subfolder)?";
  const ANOTHER_PROMPT: &str = "Add one more artifact inplacement?";
  
  let mut v = vec![];
  let mut prompt = FIRST_PROMPT;
  let mut first = true;
  
  if artifacts.is_empty() { first = false; }
  
  while Confirm::new(prompt).with_default(first).prompt()? {
    let from = Select::new("Select project's artifact:", artifacts.to_owned()).prompt()?;
    let to = Text::new("Enter relative path of artifact inplacement (inside `artifacts` subfolder):").prompt()?;
    v.push((from, to));
    prompt = ANOTHER_PROMPT;
    first = false;
  }
  
  Ok(v)
}

pub(crate) fn edit_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("{}", i18n::CFG_INVALID); }
  
  config.edit_project_from_prompt(globals)?;
  Ok(())
}
