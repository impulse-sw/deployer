use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::exit;

use crate::cmd::{NewActionArgs, CatActionArgs};
use crate::configs::DeployerGlobalConfig;
use crate::hmap;
use crate::rw::read_checked;
use crate::utils::{tags_custom_type, regexopt2str, str2regexopt, str2regex_simple, info2str, str2info, info2str_simple};
use crate::variables::{Variable, VarTraits};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DescribedAction {
  pub(crate) title: String,
  pub(crate) desc: String,
  /// Короткое имя и версия
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) info: ActionInfo,
  /// Список меток для фильтрации действий при выборе из реестра
  pub(crate) tags: Vec<String>,
  pub(crate) action: Action,
}

#[derive(Debug, Clone, Hash)]
pub(crate) struct ActionInfo {
  pub(crate) short_name: String,
  pub(crate) version: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum Action {
  /// Действие прерывания. Используется, когда пользователю необходимо выполнить действия самостоятельно.
  Interrupt,
  
  /// Кастомные команды сборки
  Custom(CustomCommand),
  /// Команда проверки состояния (может прерывать пайплайн при проверке вывода)
  Check(CheckAction),
  
  /// Принудительно размещает доступные артефакты
  ForceArtifactsEnplace,
  
  /// Действие перед сборкой
  PreBuild(PreBuildAction),
  /// Действие сборки
  Build(BuildAction),
  /// Действие после сборки
  PostBuild(PostBuildAction),
  
  /// Тесты
  Test(TestAction),
  
  /// Очистка проекта от следов взаимодействия
  ProjectClean(ProjectCleanAction),
  
  /// Упаковка артефактов
  Pack(PackAction),
  /// Доставка артефактов
  Deliver(DeliveryAction),
  /// Установка артефактов
  Install(InstallAction),
  
  /// Действие перед развёртыванием
  ConfigureDeploy(ConfigureDeployAction),
  /// Развёртывание
  Deploy(DeployAction),
  /// Действие после развёртывания
  PostDeploy(PostDeployAction),
  
  /// Действие наблюдения за состоянием
  Observe(ObserveAction),
}

impl DescribedAction {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    use inquire::{Select, Text};
    
    let short_name = Text::new("Write the Action's short name:").prompt()?;
    let version = Text::new("Specify the Action's version:").prompt()?;
    
    let info = ActionInfo { short_name, version };
    
    let name = Text::new("Write the Action's full name:").prompt()?;
    let desc = Text::new("Write the Action's description:").prompt()?;
    
    let tags: Vec<String> = tags_custom_type("Write Action's tags, if any:").prompt()?;
    
    let action_types: Vec<&str> = vec![
      "Interrupt",
      "Custom",
      "Check",
      "Force artifacts enplace",
      "Pre-build",
      "Build",
      "Post-build",
      "Test",
      "Project clean",
      "Pack",
      "Deliver",
      "Install",
      "Configure deploy",
      "Deploy",
      "Post-deploy",
      "Observe",
    ];
    
    let selected_action_type = Select::new("Select Action's type (read the docs for details):", action_types).prompt()?;
    
    let action = match selected_action_type {
      "Interrupt" => Action::Interrupt,
      "Force artifacts enplace" => Action::ForceArtifactsEnplace,
      "Custom" => {
        let command = CustomCommand::new_from_prompt()?;
        Action::Custom(command)
      },
      "Check" => {
        let bash_c = specify_bash_c()?;
        
        let placeholders = tags_custom_type("Enter command placeholders, if any:").prompt()?;
        let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
        
        let ignore_fails = !inquire::Confirm::new("Does the command failure also means check failure?").with_default(true).prompt()?;
        
        let mut success_when_found = None;
        let mut success_when_not_found = None;
        loop {
          if inquire::Confirm::new("Specify success when found some regex?").with_default(true).prompt()? {
            success_when_found = Some(specify_regex("for success on match")?);
          }
          
          if inquire::Confirm::new("Specify success when NOT found some regex?").with_default(true).prompt()? {
            success_when_not_found = Some(specify_regex("for success on mismatch")?);
          }
          
          if success_when_found.is_some() || success_when_not_found.is_some() { break }
          else { println!("For `Check` Action you need to specify at least one regex check!"); }
        }
        
        Action::Check(CheckAction {
          success_when_found,
          success_when_not_found,
          command: CustomCommand {
            bash_c,
            placeholders,
            replacements: None,
            ignore_fails,
            show_success_output: true,
            show_bash_c: false,
          },
        })
      },
      action_type @ ("Pre-build" | "Build" | "Post-build" | "Test") => {
        let supported_langs = select_programming_languages()?;
        let commands = collect_multiple_commands()?;
        
        let action = BuildAction {
          supported_langs,
          commands,
        };
        
        match action_type {
          "Pre-build" => Action::PreBuild(action),
          "Build" => Action::Build(action),
          "Post-build" => Action::PostBuild(action),
          "Test" => Action::Test(action),
          _ => unreachable!(),
        }
      },
      "Project clean" => {
        let to_remove = Text::new("Enter comma-separated list of paths to remove:")
          .prompt()
          .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())?;
        let additional_commands = collect_multiple_commands()?;
        
        Action::ProjectClean(ProjectCleanAction {
          to_remove,
          additional_commands,
        })
      },
      action_type @ ("Pack" | "Deliver" | "Install") => {
        let target = TargetDescription::new_from_prompt()?;
        let commands = collect_multiple_commands()?;
        
        let action = PackAction {
          target: Some(target),
          commands,
        };
        
        match action_type {
          "Pack" => Action::Pack(action),
          "Deliver" => Action::Deliver(action),
          "Install" => Action::Install(action),
          _ => unreachable!(),
        }
      },
      action_type @ ("Configure deploy" | "Deploy" | "Post-deploy") => {
        let deploy_toolkit = Text::new("Enter deploy toolkit name (or hit `esc`):").prompt_skippable()?;
        let tags = tags_custom_type("Enter deploy tags:").prompt()?;
        let commands = collect_multiple_commands()?;
        
        let action = DeployAction {
          deploy_toolkit,
          tags,
          commands,
        };
        
        match action_type {
          "Configure deploy" => Action::ConfigureDeploy(action),
          "Deploy" => Action::Deploy(action),
          "Post-deploy" => Action::PostDeploy(action),
          _ => unreachable!(),
        }
      },
      "Observe" => {
        let tags = tags_custom_type("Enter observe tags:").prompt()?;
        let command = CustomCommand::new_from_prompt()?;
        
        Action::Observe(ObserveAction { tags, command })
      },
      _ => unreachable!(),
    };
    
    let described_action = DescribedAction {
      title: name,
      desc,
      info,
      tags,
      action,
    };
    
    Ok(described_action)
  }
  
  fn setup_buildlike_action(
    &self,
    action: &BuildAction,
    langs: &Vec<ProgrammingLanguage>,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<BuildAction> {
    let mut action = action.clone();
    if 
      !langs.iter().any(|l| action.supported_langs.contains(l)) && 
      !inquire::Confirm::new(
        &format!(
          "Action `{}` may be not fully compatible with your project due to requirements (Action's supported langs: {:?}, your project's: {:?}). Use this Action anyway? If no, Action will be skipped. (y/n)",
          info2str_simple(&self.info),
          action.supported_langs,
          langs,
        )
      ).prompt()?
    {
      return Ok(BuildAction::default())
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?; }
    
    Ok(action)
  }
  
  fn setup_projectclean_action(
    &self,
    action: &ProjectCleanAction,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<ProjectCleanAction> {
    let mut action = action.clone();
    for cmd in &mut action.additional_commands { *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?; }
    Ok(action)
  }
  
  fn setup_packlike_action(
    &self,
    action: &PackAction,
    targets: &[TargetDescription],
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<PackAction> {
    let mut action = action.clone();
    
    if
      action.target.as_ref().is_some_and(|t| !targets.contains(t)) &&
      !inquire::Confirm::new(
        &format!(
          "Action `{}` may be not fully compatible with your project due to requirements (Action's target: {}, your project's: {:?}). Use this Action anyway? If no, Action will be skipped. (y/n)",
          info2str_simple(&self.info),
          action.target.as_ref().unwrap(),
          targets.iter().map(TargetDescription::to_string).collect::<Vec<_>>(),
        )
      ).prompt()?
    {
      return Ok(PackAction::default())
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?; }
    Ok(action)
  }
  
  fn setup_deploylike_action(
    &self,
    action: &DeployAction,
    deploy_toolkit: &Option<String>,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<DeployAction> {
    let mut action = action.clone();
    if
      action.deploy_toolkit.as_ref().is_some_and(|l| deploy_toolkit.as_ref().is_some_and(|r| l.as_str() != r.as_str())) &&
      !inquire::Confirm::new(
        &format!(
          "Action `{}` may be not fully compatible with your project due to requirements (Action's deploy toolkit: {}, your project's: {}). Use this Action anyway? If no, Action will be skipped. (y/n)",
          info2str_simple(&self.info),
          action.deploy_toolkit.as_ref().unwrap(),
          deploy_toolkit.as_ref().unwrap(),
        )
      ).prompt()?
    {
      return Ok(DeployAction::default())
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?; }
    
    Ok(action)
  }
  
  fn setup_observe_action(
    &self,
    action: &ObserveAction,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<ObserveAction> {
    let mut action = action.clone();
    
    action.command = action.command.prompt_setup_for_project(&self.info, variables, artifacts)?;
    
    Ok(action)
  }
  
  pub(crate) fn prompt_setup_for_project(
    &self,
    langs: &Vec<ProgrammingLanguage>,
    deploy_toolkit: &Option<String>,
    targets: &[TargetDescription],
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<Self> {
    let action = match &self.action {
      Action::Custom(cmd) => Action::Custom(cmd.prompt_setup_for_project(&self.info, variables, artifacts)?),
      Action::PreBuild(pb_action) => Action::PreBuild(self.setup_buildlike_action(pb_action, langs, variables, artifacts)?),
      Action::Build(b_action) => Action::Build(self.setup_buildlike_action(b_action, langs, variables, artifacts)?),
      Action::PostBuild(pb_action) => Action::PostBuild(self.setup_buildlike_action(pb_action, langs, variables, artifacts)?),
      Action::Test(t_action) => Action::Test(self.setup_buildlike_action(t_action, langs, variables, artifacts)?),
      Action::ProjectClean(pc_action) => Action::ProjectClean(self.setup_projectclean_action(pc_action, variables, artifacts)?),
      Action::Pack(p_action) => Action::Pack(self.setup_packlike_action(p_action, targets, variables, artifacts)?),
      Action::Deliver(p_action) => Action::Deliver(self.setup_packlike_action(p_action, targets, variables, artifacts)?),
      Action::Install(p_action) => Action::Install(self.setup_packlike_action(p_action, targets, variables, artifacts)?),
      Action::ConfigureDeploy(cd_action) => Action::ConfigureDeploy(self.setup_deploylike_action(cd_action, deploy_toolkit, variables, artifacts)?),
      Action::Deploy(d_action) => Action::Deploy(self.setup_deploylike_action(d_action, deploy_toolkit, variables, artifacts)?),
      Action::PostDeploy(pd_action) => Action::PostDeploy(self.setup_deploylike_action(pd_action, deploy_toolkit, variables, artifacts)?),
      Action::Observe(o_action) => Action::Observe(self.setup_observe_action(o_action, variables, artifacts)?),
      Action::Interrupt | Action::Check(_) | Action::ForceArtifactsEnplace => self.action.clone(),
    };
    
    let mut described_action = self.clone();
    described_action.action = action;
    
    Ok(described_action)
  }
  
  pub(crate) fn edit_action_from_prompt(&mut self) -> anyhow::Result<()> {
    let mut actions = vec![
      "Edit title",
      "Edit description",
      "Edit tags",
    ];
    match &self.action {
      Action::Custom(_) | Action::Observe(_) => { actions.push("Edit command"); },
      Action::Check(_) => { actions.extend_from_slice(&["Edit regexes", "Edit command"]); }
      Action::ProjectClean(_) => { actions.extend_from_slice(&["Edit files and folders to remove", "Edit commands"]); },
      Action::PreBuild(_) | Action::Build(_) | Action::PostBuild(_) | Action::Test(_) => {
        actions.extend_from_slice(&["Edit programming languages", "Edit commands"]);
      },
      Action::Pack(_) | Action::Deliver(_) | Action::Install(_) => {
        actions.extend_from_slice(&["Edit targets", "Edit commands"]);
      },
      Action::ConfigureDeploy(_) | Action::Deploy(_) | Action::PostDeploy(_) => {
        actions.extend_from_slice(&["Edit deploy toolkit", "Edit commands"]);
      },
      Action::Interrupt | Action::ForceArtifactsEnplace => {},
    }
    
    while let Some(action) = inquire::Select::new(
      "Select an edit action (hit `esc` when done):",
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        "Edit title" => self.title = inquire::Text::new("Write the Action's full name:").prompt()?,
        "Edit description" => self.desc = inquire::Text::new("Write the Action's description:").prompt()?,
        "Edit tags" => self.tags = tags_custom_type("Write Action's tags, if any:").prompt()?,
        "Edit command" => {
          if let Action::Custom(cmd) = &mut self.action {
            cmd.edit_command_from_prompt()?;
          } else if let Action::Observe(o_command) = &mut self.action {
            o_command.command.edit_command_from_prompt()?;
          } else if let Action::Check(c_command) = &mut self.action {
            c_command.command.edit_command_from_prompt()?;
          }
        },
        "Edit commands" => {
          match &mut self.action {
            Action::ProjectClean(a) => a.additional_commands.edit_from_prompt()?,
            Action::PreBuild(a) => a.commands.edit_from_prompt()?,
            Action::Build(a) => a.commands.edit_from_prompt()?,
            Action::PostBuild(a) => a.commands.edit_from_prompt()?,
            Action::Test(a) => a.commands.edit_from_prompt()?,
            Action::Pack(a) => a.commands.edit_from_prompt()?,
            Action::Deliver(a) => a.commands.edit_from_prompt()?,
            Action::Install(a) => a.commands.edit_from_prompt()?,
            Action::ConfigureDeploy(a) => a.commands.edit_from_prompt()?,
            Action::Deploy(a) => a.commands.edit_from_prompt()?,
            Action::PostDeploy(a) => a.commands.edit_from_prompt()?,
            Action::Interrupt | Action::Custom(_) | Action::Check(_) | Action::ForceArtifactsEnplace | Action::Observe(_) => {},
          }
        },
        "Edit regexes" if let Action::Check(c_action) = &mut self.action => c_action.change_regexes_from_prompt()?,
        "Edit files and folders to remove" if let Action::ProjectClean(pc_action) = &mut self.action => {
          pc_action.to_remove = inquire::Text::new("Enter comma-separated list of paths to remove:")
            .prompt()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())?;
        }
        "Edit programming languages" => {
          match &mut self.action {
            Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => {
              a.supported_langs = select_programming_languages()?;
            },
            _ => {},
          }
        },
        "Edit targets" => {
          match &mut self.action {
            Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => {
              a.target = Some(TargetDescription::new_from_prompt()?);
            },
            _ => {},
          }
        },
        "Edit deploy toolkit" => {
          match &mut self.action {
            Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => {
              a.deploy_toolkit = inquire::Text::new("Enter deploy toolkit name (or hit `esc`):").prompt_skippable()?;
            },
            _ => {},
          }
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

/// Команда, исполняемая в командной строке `bash`.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct CustomCommand {
  /// Команда.
  pub(crate) bash_c: String,
  
  /// Плейсхолдеры команды. Используются для подстановки значений при выполнении действия.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) placeholders: Option<Vec<String>>,
  /// Список переменных для подстановки вместо плейсхолдеров.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) replacements: Option<Vec<Vec<(String, Variable)>>>,
  
  /// Игнорировать ли ошибки команды.
  pub(crate) ignore_fails: bool,
  /// Отображать ли вывод команды, если не возникла ошибка.
  pub(crate) show_success_output: bool,
  /// Отображать ли команду.
  /// 
  /// Потенциально команда может содержать уязвимые переменные, такие как: ключи, пароли, пути к чувствительным файлам и т.д.
  /// Их можно скрыть при сборке, если указать `false`.
  pub(crate) show_bash_c: bool,
}

impl CustomCommand {
  /// Создаёт новую команду.
  pub(crate) fn new_from_prompt() -> anyhow::Result<CustomCommand> {
    let bash_c = specify_bash_c()?;
    
    let placeholders = tags_custom_type("Enter command placeholders, if any:").prompt()?;
    let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
    
    let ignore_fails = inquire::Confirm::new("Ignore command failures?").with_default(false).prompt()?;
    let show_bash_c = inquire::Confirm::new("Show an entire command at build stage?").with_default(true).prompt()?;
    let show_success_output = inquire::Confirm::new("Show an output of command if it executed successfully?").with_default(false).prompt()?;
    
    Ok(CustomCommand {
      bash_c,
      placeholders,
      ignore_fails,
      show_bash_c,
      show_success_output,
      replacements: None,
    })
  }
  
  pub(crate) fn prompt_setup_for_project(
    &self,
    info: &ActionInfo,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<Self> {
    use inquire::{Confirm, Select, Text};
    
    const USE_ANOTHER: &str = "· Specify another variable";
    
    if self.placeholders.as_ref().is_none_or(|ps| ps.is_empty()) { return Ok(self.clone()) }
    
    println!("Specifying variables for `{}` Action:", info2str_simple(info).blue());
    
    let mut all_variables = variables.titles();
    all_variables.extend_from_slice(artifacts);
    all_variables.push(USE_ANOTHER.to_string());
    
    let mut replacements = vec![];
    let mut explicitly_show_bash_c = None;
    loop {
      let mut replacement = vec![];
      for placeholder in self.placeholders.as_ref().unwrap() {
        let mut selected = Select::new(
          &format!("Select variable to replace `{}` in `{}` bash command:", placeholder.green(), self.bash_c.green()),
          all_variables.clone(),
        ).prompt()?;
        
        if variables.is_secret(selected.as_str()) {
          println!("At build stage the command will be hidden due to usage of secret variable.");
          explicitly_show_bash_c = Some(false);
        }
        
        if selected.as_str() == USE_ANOTHER {
          selected = Text::new(&format!("Enter variable to replace `{}` in `{}` bash command:", placeholder.green(), self.bash_c.green())).prompt()?;
        }
        
        replacement.push(
          (
            placeholder.to_owned(),
            variables
              .find(&selected)
              .unwrap_or_else(|| Variable::new_plain(&selected, &selected)),
          )
        );
      }
      
      replacements.push(replacement);
      if !Confirm::new("Enter `y` if you need exec this command one more time with others variables.").with_default(false).prompt()? { break }
    }
    
    let mut r = self.clone();
    r.replacements = Some(replacements);
    r.show_bash_c = if let Some(show) = explicitly_show_bash_c { show } else { r.show_bash_c };
    Ok(r)
  }
  
  fn edit_command_from_prompt(&mut self) -> anyhow::Result<()> {
    while let Some(action) = inquire::Select::new(
      &format!("Select an option to change in `{}` command (hit `esc` when done):", self.bash_c.green()),
      vec![
        "Edit bash command",
        "Change command placeholders",
        "Change command failure ignorance",
        "Change whether command is displayed or not on build stage",
        "Change whether command output is displayed or not when it executed successfully",
      ],
    ).prompt_skippable()? {
      match action {
        "Edit bash command" => self.bash_c = specify_bash_c()?,
        "Change command placeholders" => {
          let placeholders = tags_custom_type("Enter command placeholders, if any:").prompt()?;
          self.placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
        },
        "Change command failure ignorance" => {
          self.ignore_fails = inquire::Confirm::new("Ignore command failures?").with_default(false).prompt()?;
        },
        "Change whether command is displayed or not on build stage" => {
          self.show_bash_c = inquire::Confirm::new("Show an entire command at build stage?").with_default(true).prompt()?;
        },
        "Change whether command output is displayed or not when it executed successfully" => {
          self.show_success_output = inquire::Confirm::new("Show an output of command if it executed successfully?").with_default(false).prompt()?;
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

pub(crate) trait Edit {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()>;
  fn reorder(&mut self) -> anyhow::Result<()>;
  fn add_item(&mut self) -> anyhow::Result<()>;
  fn remove_item(&mut self) -> anyhow::Result<()>;
}

pub(crate) trait EditExtended<T> {
  fn edit_from_prompt(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn reorder(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn add_item(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn remove_item(&mut self, opts: &mut T) -> anyhow::Result<()>;
}

impl Edit for Vec<CustomCommand> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("Edit command `{}`", c.bash_c.green());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&["Reorder commands".to_string(), "Add command".to_string(), "Remove command".to_string()]);
      
      if let Some(action) = inquire::Select::new("Select a concrete command to change (hit `esc` when done):", cs).prompt_skippable()? {
        match action.as_str() {
          "Reorder commands" => self.reorder()?,
          "Add command" => self.add_item()?,
          "Remove command" => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_command_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> {
    use inquire::ReorderableList;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for selected_command in self.iter() {
      let key = format!("`{}`", selected_command.bash_c);
      k.push(key.clone());
      h.insert(key, selected_command);
    }
    
    let reordered = ReorderableList::new("Reorder Action's commands:", k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(CustomCommand::new_from_prompt()?);
    
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("`{}`", c.bash_c.green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new("Select a command to remove:", cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    
    Ok(())
  }
}

impl Edit for Vec<DescribedAction> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("Edit Action `{}` - `{}`", c.title, info2str_simple(&c.info));
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&["Reorder Actions".to_string(), "Add Action".to_string(), "Remove Action".to_string()]);
      
      if let Some(action) = inquire::Select::new("Select a concrete Action to change (hit `esc` when done):", cs).prompt_skippable()? {
        match action.as_str() {
          "Reorder Actions" => self.reorder()?,
          "Add Action" => self.add_item()?,
          "Remove Action" => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_action_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> {
    use inquire::ReorderableList;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for selected in self.iter() {
      let key = format!("Action `{}` - `{}`", selected.title, info2str_simple(&selected.info));
      k.push(key.clone());
      h.insert(key, selected);
    }
    
    let reordered = ReorderableList::new("Reorder Action's commands:", k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(DescribedAction::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("Remove Action `{}` - `{}`", c.title, info2str_simple(&c.info));
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new("Select an Action to remove:", cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    
    Ok(())
  }
}

/// Команда, проверяющая вывод на определённое условие.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct CheckAction {
  pub(crate) command: CustomCommand,
  #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt", skip_serializing_if = "Option::is_none")]
  pub(crate) success_when_found: Option<Regex>,
  #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt", skip_serializing_if = "Option::is_none")]
  pub(crate) success_when_not_found: Option<Regex>,
}

impl CheckAction {
  pub(crate) fn change_regexes_from_prompt(&mut self) -> anyhow::Result<()> {
    println!("Current regexes are:");
    println!("`success_when_found` = {:?}", self.success_when_found);
    println!("`success_when_not_found` = {:?}", self.success_when_not_found);
    
    loop {
      if inquire::Confirm::new("Specify success when found some regex?").with_default(true).prompt()? {
        self.success_when_found = Some(specify_regex("for success on match")?);
      }
      
      if inquire::Confirm::new("Specify success when NOT found some regex?").with_default(true).prompt()? {
        self.success_when_not_found = Some(specify_regex("for success on mismatch")?);
      }
      
      if self.success_when_found.is_some() || self.success_when_not_found.is_some() { break }
      else { println!("For `Check` Action you need to specify at least one regex check!"); }
    }
    
    Ok(())
  }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub(crate) enum ProgrammingLanguage {
  Rust,
  Go,
  C,
  Cpp,
  Python,
  Other(String),
}

impl ProgrammingLanguage {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let s = inquire::Text::new("Input the programming language name:").prompt()?;
    let pl = match s.as_str() {
      "Rust" => Self::Rust,
      "Go" => Self::Go,
      "C" => Self::C,
      "C++" => Self::Cpp,
      "Python" => Self::Python,
      s => Self::Other(s.to_owned()),
    };
    Ok(pl)
  }
}

impl std::fmt::Display for ProgrammingLanguage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let lang = match self {
      Self::Rust => "Rust".to_string(),
      Self::Go => "Go".to_string(),
      Self::C => "C".to_string(),
      Self::Cpp => "C++".to_string(),
      Self::Python => "Python".to_string(),
      Self::Other(s) => s.to_owned(),
    };
    
    f.write_str(&lang)
  }
}

impl Edit for Vec<ProgrammingLanguage> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("Language `{}`", c);
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&["Add".to_string(), "Remove".to_string()]);
      
      if let Some(action) = inquire::Select::new("Select a concrete language to change (hit `esc` when done):", cs).prompt_skippable()? {
        match action.as_str() {
          "Add" => self.add_item()?,
          "Remove" => self.remove_item()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(ProgrammingLanguage::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("`{}`", c);
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new("Select a language to remove:", cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

// /// Параметры инициализации проекта из шаблона.
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct TemplateInitializationAction {
//   /// Папка с шаблоном проекта.
//   pub(crate) template_folder: String,
//   /// Языки проекта.
//   pub(crate) langs: Vec<ProgrammingLanguage>,
//   /// Плейсхолдер, который будет заменён во всех указанных файлах.
//   pub(crate) project_name_placeholder: (String, Vec<String>),
//   /// Дополнительные команды по инициализации проекта.
//   pub(crate) additional_commands: Vec<CustomCommand>,
// }
// 
// /// Зависимость, которую можно переиспользовать в проектах.
// #[derive(Deserialize, Serialize, Debug)]
// pub(crate) struct DescribedDependency {
//   pub(crate) title: String,
//   pub(crate) desc: String,
//   pub(crate) supported_langs: Vec<ProgrammingLanguage>,
//   pub(crate) tags: Vec<String>,
//   pub(crate) add_action: DependencyAdditionAction,
//   pub(crate) patch_action: DependencyPatchAction,
// }
// 
// /// Добавление текущего проекта в качестве зависимости
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct DependencyRegistrationAction {
//   pub(crate) title: String,
//   pub(crate) desc: String,
//   #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
//   pub(crate) dep_info: DependencyInfo,
//   pub(crate) supported_langs: Vec<ProgrammingLanguage>,
//   pub(crate) tags: Vec<String>,
// }
// 
// pub(crate) type DependencyInfo = ActionInfo;
// 
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct DependencyUse {
//   #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
//   pub(crate) info: DependencyInfo,
//   pub(crate) inplace_type: SimpleInplaceType,
// }
// 
// #[derive(Deserialize, Serialize, Clone, Default, Debug)]
// pub(crate) enum SimpleInplaceType {
//   #[default]
//   Symlink,
//   Copy,
// }
// 
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct DependencyRemovalAction {
//   #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
//   pub(crate) info: DependencyInfo,
// }
// 
// /// Параметры добавления зависимости для проекта.
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) enum DependencyAdditionAction {
//   GitLink(GitLinkOptions),
//   Folder(String),
//   Symlink(String),
//   DeployerDependency(DependencyUse),
//   DeployerArtifact(UseArtifactAction),
// }
// 
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct GitLinkOptions {
//   pub(crate) link: String,
//   #[serde(skip_serializing_if = "Option::is_none")]
//   pub(crate) branch: Option<String>,
//   #[serde(skip_serializing_if = "Option::is_none")]
//   pub(crate) tag: Option<String>,
//   pub(crate) init_with_submodules: SubmoduleInitializationRules,
// }
// 
// #[derive(Deserialize, Serialize, Clone, Default, Debug)]
// pub(crate) enum SubmoduleInitializationRules {
//   #[default]
//   All,
//   OnlyThese(Vec<String>),
//   None,
// }
// 
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct DependencyPatchAction {
//   pub(crate) apply_patches: Vec<PatchRules>,
// }
// 
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) enum PatchRules {
//   GitPatch(String),
//   CustomCommandPatch(CustomCommand),
// }

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub(crate) struct BuildAction {
  pub(crate) supported_langs: Vec<ProgrammingLanguage>,
  pub(crate) commands: Vec<CustomCommand>,
}

pub(crate) type PreBuildAction = BuildAction;
pub(crate) type PostBuildAction = BuildAction;
pub(crate) type TestAction = BuildAction;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ProjectCleanAction {
  pub(crate) to_remove: Vec<String>,
  pub(crate) additional_commands: Vec<CustomCommand>,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub(crate) struct PackAction {
  pub(crate) target: Option<TargetDescription>,
  pub(crate) commands: Vec<CustomCommand>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) struct TargetDescription {
  pub(crate) arch: String,
  pub(crate) os: OsVariant,
  pub(crate) derivative: String,
  pub(crate) version: OsVersionSpecification,
}

impl std::fmt::Display for TargetDescription {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let os = match &self.os {
      OsVariant::Android => "android",
      OsVariant::iOS => "ios",
      OsVariant::Linux => "linux",
      OsVariant::UnixLike(nix) => &format!("unix-{}", nix),
      OsVariant::Windows => "windows",
      OsVariant::macOS => "macos",
      OsVariant::Other(other) => other,
    };
    
    let os_ver = match &self.version {
      OsVersionSpecification::No => "any",
      OsVersionSpecification::Weak(ver) => &format!("^{}", ver),
      OsVersionSpecification::Strong(ver) => ver,
    };
    
    f.write_str(&format!("{}/{}@{}@{}", self.arch, os, self.derivative, os_ver))
  }
}

impl TargetDescription {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    use inquire::{Select, Text};
    
    let arch = Text::new("Enter the target's architecture:").prompt()?;
    
    let os = Select::new(
      "Select OS:",
      vec!["Android", "iOS", "Linux", "Unix-like", "Windows", "macOS", "Other"]
    ).prompt()?;
    
    let os_variant = match os {
      "Android" => OsVariant::Android,
      "iOS" => OsVariant::iOS,
      "Linux" => OsVariant::Linux,
      "Unix-like" => {
        let name = Text::new("Enter Unix-like OS name:").prompt()?;
        OsVariant::UnixLike(name)
      },
      "Windows" => OsVariant::Windows,
      "macOS" => OsVariant::macOS,
      "Other" => {
        let name = Text::new("Enter OS name:").prompt()?;
        OsVariant::Other(name)
      },
      _ => unreachable!(),
    };
    
    let derivative = Text::new("Enter OS derivative:").prompt()?;
    
    let version_type = Select::new(
      "Select version specification type:",
      vec!["Not Specified", "Weak Specified", "Strong Specified"]
    ).prompt()?;
    
    let version = match version_type {
      "Not Specified" => OsVersionSpecification::No,
      "Weak Specified" => {
        let ver = Text::new("Enter version:").prompt()?;
        OsVersionSpecification::Weak(ver)
      },
      "Strong Specified" => {
        let ver = Text::new("Enter version:").prompt()?;
        OsVersionSpecification::Strong(ver)
      },
      _ => unreachable!(),
    };
    
    Ok(TargetDescription {
      arch,
      os: os_variant,
      derivative,
      version,
    })
  }
  
  pub(crate) fn edit_target_from_prompt(&mut self) -> anyhow::Result<()> {
    let actions = vec![
      "Edit arch",
      "Edit OS",
    ];
    
    while let Some(action) = inquire::Select::new(
      "Select an edit action (hit `esc` when done):",
      actions.clone(),
    ).prompt_skippable()? {
      use inquire::{Select, Text};
      
      match action {
        "Edit arch" => self.arch = Text::new("Enter the target's architecture:").prompt()?,
        "Edit OS" => {
          let os = Select::new(
            "Select OS:",
            vec!["Android", "iOS", "Linux", "Unix-like", "Windows", "macOS", "Other"]
          ).prompt()?;
          
          self.os = match os {
            "Android" => OsVariant::Android,
            "iOS" => OsVariant::iOS,
            "Linux" => OsVariant::Linux,
            "Unix-like" => {
              let name = Text::new("Enter Unix-like OS name:").prompt()?;
              OsVariant::UnixLike(name)
            },
            "Windows" => OsVariant::Windows,
            "macOS" => OsVariant::macOS,
            "Other" => {
              let name = Text::new("Enter OS name:").prompt()?;
              OsVariant::Other(name)
            },
            _ => unreachable!(),
          };
          
          self.derivative = Text::new("Enter OS derivative:").prompt()?;
          
          let version_type = Select::new(
            "Select version specification type:",
            vec!["Not Specified", "Weak Specified", "Strong Specified"]
          ).prompt()?;
          
          self.version = match version_type {
            "Not Specified" => OsVersionSpecification::No,
            "Weak Specified" => {
              let ver = Text::new("Enter version:").prompt()?;
              OsVersionSpecification::Weak(ver)
            },
            "Strong Specified" => {
              let ver = Text::new("Enter version:").prompt()?;
              OsVersionSpecification::Strong(ver)
            },
            _ => unreachable!(),
          };
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) enum OsVariant {
  Android,
  #[allow(non_camel_case_types)]
  iOS,
  Linux,
  UnixLike(String),
  Windows,
  #[allow(non_camel_case_types)]
  macOS,
  Other(String),
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub(crate) enum OsVersionSpecification {
  #[default]
  No,
  /// Даже если указана версия, при несоответствии версий может заработать.
  Weak(String),
  Strong(String),
}

pub(crate) type DeliveryAction = PackAction;
pub(crate) type InstallAction = PackAction;

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub(crate) struct DeployAction {
  pub(crate) deploy_toolkit: Option<String>,
  pub(crate) tags: Vec<String>,
  pub(crate) commands: Vec<CustomCommand>,
}

pub(crate) type ConfigureDeployAction = DeployAction;
pub(crate) type PostDeployAction = DeployAction;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ObserveAction {
  pub(crate) tags: Vec<String>,
  pub(crate) command: CustomCommand,
}

// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct RegisterArtifactAction {
//   pub(crate) name: String,
//   pub(crate) desc: String,
//   #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
//   pub(crate) artifact_info: ArtifactInfo,
//   pub(crate) tags: Vec<String>,
//   pub(crate) inplace: HashMap<String, String>,
// }
// 
// pub(crate) type ArtifactInfo = DependencyInfo;
// 
// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub(crate) struct UseArtifactAction {
//   #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
//   pub(crate) artifact_info: ArtifactInfo,
//   pub(crate) inplace: HashMap<String, String>,
// }

/// Перечисляет все доступные действия.
pub(crate) fn list_actions(
  globals: &DeployerGlobalConfig,
) {
  println!("Available Actions in Deployer's Registry:");
  
  let mut actions = globals.actions_registry.values().collect::<Vec<_>>();
  actions.sort_by_key(|a| info2str_simple(&a.info));
  
  for action in actions {
    let action_info = format!("{}@{}", action.info.short_name, action.info.version);
    let action_title = format!("[{}]", action.title);
    let tags = if action.tags.is_empty() { String::new() } else { format!(" (tags: {})", action.tags.join(", ").as_str().blue().italic()) };
    println!("• {} {}{}", action_info.blue().bold(), action_title.green().bold(), tags);
    println!("\t> {}", action.desc.green().italic());
  }
}

/// Удаляет выбранное действие.
pub(crate) fn remove_action(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<()> {
  use inquire::{Select, Confirm};
  
  if globals.actions_registry.is_empty() {
    println!("There is no Actions in Registry.");
    return Ok(())
  }
  
  let mut actions = globals.actions_registry.values().collect::<Vec<_>>();
  actions.sort_by_key(|a| info2str_simple(&a.info));
  
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
    
    (h, k)
  };
  
  let selected_action = Select::new("Select Action for removing from Actions' Registry:", keys).prompt()?;
  let action = *actions.get(&selected_action).unwrap();
  
  if !Confirm::new("Are you sure? (y/n)").prompt()? { return Ok(()) }
  
  globals.actions_registry.remove(&info2str_simple(&action.info));
  
  Ok(())
}

/// Добавляет новое действие.
pub(crate) fn new_action(
  globals: &mut DeployerGlobalConfig,
  args: &NewActionArgs,
  // data_dir: &str,
) -> anyhow::Result<DescribedAction> {
  let actions = &mut globals.actions_registry;
  
  if let Some(from_file) = &args.from {
    let action = read_checked::<DescribedAction>(from_file).map_err(|e| {
      panic!("Can't read provided Action file due to: {}", e);
    }).unwrap();
    actions.insert(info2str_simple(&action.info), action.clone());
    return Ok(action)
  }
  
  let described_action = DescribedAction::new_from_prompt()?;
  
  if
    actions.contains_key(&info2str_simple(&described_action.info)) &&
    !inquire::Confirm::new(&format!(
      "Actions Registry already have `{}` Action. Do you want to override it? (y/n)", info2str_simple(&described_action.info))
    ).prompt()?
  {
    exit(0);
  }
  
  actions.insert(info2str_simple(&described_action.info), described_action.clone());
  
  Ok(described_action)
}

/// Создаёт несколько новых команд.
fn collect_multiple_commands() -> anyhow::Result<Vec<CustomCommand>> {
  use inquire::Confirm;
  
  let mut commands = Vec::new();
  while Confirm::new("Add command? (y/n)").prompt()? {
    commands.push(CustomCommand::new_from_prompt()?);
  }
  Ok(commands)
}

fn collect_multiple_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  let langs = tags_custom_type("Enter the names of programming languages separated by commas:").prompt()?;
  let mut v = vec![];
  
  for lang in langs {
    match lang.as_str() {
      "Rust" | "Go" | "C" | "C++" | "Python" => continue,
      lang => v.push(ProgrammingLanguage::Other(lang.to_owned())),
    }
  }
  
  Ok(v)
}

/// Парсит вводимые языки программирования.
pub(crate) fn select_programming_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  use inquire::MultiSelect;
  
  let langs = vec!["Rust", "Go", "C", "C++", "Python", "Others"];
  let selected = MultiSelect::new("Select programming languages:", langs).prompt()?;
  
  let mut result = Vec::new();
  for lang in selected {
    let lang = match lang {
      "Rust" => ProgrammingLanguage::Rust,
      "Go" => ProgrammingLanguage::Go,
      "C" => ProgrammingLanguage::C,
      "C++" => ProgrammingLanguage::Cpp,
      "Python" => ProgrammingLanguage::Python,
      "Others" => {
        let langs = collect_multiple_languages()?;
        result.extend_from_slice(&langs);
        continue
      }
      _ => unreachable!(),
    };
    result.push(lang);
  }
  
  Ok(result)
}

// fn collect_key_value_pairs(prompt: &str) -> anyhow::Result<HashMap<String, String>> {
//   use inquire::Text;
//   
//   let mut map = HashMap::new();
//   loop {
//     let key = Text::new(prompt).prompt()?;
//     if key.is_empty() {
//       break;
//     }
//     let value = Text::new("Enter value:").prompt()?;
//     map.insert(key, value);
//   }
//   Ok(map)
// }

fn specify_bash_c() -> anyhow::Result<String> {
  let mut bash_c;
  loop {
    bash_c = inquire::Text::new("Enter typical bash command (or enter '/h' for help):").prompt()?;
    if bash_c.as_str() != "/h" { break }
    println!("Guide: `{}`", "Bash Commands for Deployer".blue());
    println!(">>> The usage of `bash` commands in Deployer is very simple.");
    println!(">>> You can use `{}` for home directories, your default `{}` variable and so on.", "~".green(), "PATH".green());
    println!(">>> ");
    println!(">>> Also you can write your commands even when there are some unspecified variables:");
    println!(">>> `{}`", "g++ <input-file> -o <output-file>".green());
    println!(">>> `{}{}`", "docker compose run -e DEPLOY_KEY=".green(), "{{my very secret key}}".red());
  }
  
  Ok(bash_c)
}

fn specify_regex(for_what: &str) -> anyhow::Result<Regex> {
  let mut regex_str;
  
  loop {
    regex_str = inquire::Text::new(
      &format!("Enter regex {} (or enter '/h' for help):", for_what)
    ).prompt()?;
    
    if regex_str.as_str() != "/h" { break }
    println!("Guide: `{}`", "Regex Checks for Deployer".blue());
    println!(">>> The usage of regex checks in Deployer is simple enough.");
    println!(">>> If you want to specify some text that needed to be found, you simply write this text.");
    println!(">>> ");
    println!(">>> For any supported regex read this: {}", "https://docs.rs/regex/latest/regex/".blue());
    println!(">>> For checks use this: {} (select `Rust` flavor at left side panel)", "https://regex101.com/".blue());
  }
  
  str2regex_simple(regex_str.as_str())
}

pub(crate) fn cat_action(
  globals: &DeployerGlobalConfig,
  args: &CatActionArgs,
) -> anyhow::Result<()> {
  let action = match globals.actions_registry.get(&args.action_short_info_and_version) {
    None => exit(1),
    Some(action) => action,
  };
  
  let action_yaml = serde_json::to_string_pretty(&action).unwrap();
  println!("{}", action_yaml);
  
  Ok(())
}

pub(crate) fn edit_action(
  globals: &mut DeployerGlobalConfig,
  args: &CatActionArgs,
) -> anyhow::Result<()> {
  let described_action = match globals.actions_registry.get_mut(&args.action_short_info_and_version) {
    None => exit(1),
    Some(action) => action,
  };
  
  described_action.edit_action_from_prompt()?;
  
  Ok(())
}
