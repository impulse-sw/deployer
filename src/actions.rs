use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::exit;

use crate::cmd::{NewActionArgs, CatActionArgs};
use crate::configs::DeployerGlobalConfig;
use crate::hmap;
use crate::rw::read_checked;
use crate::utils::{tags_custom_type, regex2str, str2regex, info2str, str2info, info2str_simple, target2str_simple};

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

pub(crate) type ActionInfo = DependencyInfo;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum Action {
  /// Действие прерывания. Используется, когда пользователю необходимо выполнить действия самостоятельно.
  Interrupt,
  
  /// Кастомные команды сборки
  Custom(CustomCommand),
  /// Команда проверки состояния (может прерывать пайплайн при проверке вывода)
  Check(CheckAction),
  
  /// Инициализация проекта из шаблона
  InitWithTemplate(TemplateInitializationAction),
  
  /// Действие сохранения состояния проекта в реестре в качестве зависимости
  RegisterDependency(DependencyRegistrationAction),
  /// Удаление состояния зависимости из реестра
  DeleteDependency(DependencyRemovalAction),
  
  /// Действие добавления зависимости
  AddDependency(DependencyAdditionAction),
  /// Действие автопатча зависимости
  PatchDependency(DependencyPatchAction),
  
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
  
  /// Действие сохранения артефакта в реестре Деплойера
  RegisterArtifact(RegisterArtifactAction),
  /// Действие использования артефакта Деплойера в пайплайне
  UseArtifact(UseArtifactAction),
  /// Действие запуска артефакта Деплойера в пайплайне
  UseArtifactWith(CustomCommand),
}

impl DescribedAction {
  fn setup_buildlike_action(
    &self,
    action: &BuildAction,
    langs: &Vec<ProgrammingLanguage>,
    artifacts: &Vec<String>,
  ) -> anyhow::Result<BuildAction> {
    let mut action = action.clone();
    if !langs.iter().position(|l| action.supported_langs.contains(l)).is_some() {
      if !inquire::Confirm::new(
        &format!(
          "Action `{}` may be not fully compatible with your project due to requirements (Action's supported langs: {:?}, your project's: {:?}). Use this Action anyway? If no, Action will be skipped. (y/n)",
          info2str_simple(&self.info),
          action.supported_langs,
          langs,
        )
      ).prompt()? {
        return Ok(BuildAction::default())
      }
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, artifacts)?; }
    
    Ok(action)
  }
  
  fn setup_projectclean_action(
    &self,
    action: &ProjectCleanAction,
    artifacts: &Vec<String>,
  ) -> anyhow::Result<ProjectCleanAction> {
    let mut action = action.clone();
    for cmd in &mut action.additional_commands { *cmd = cmd.prompt_setup_for_project(&self.info, artifacts)?; }
    Ok(action)
  }
  
  fn setup_packlike_action(
    &self,
    action: &PackAction,
    targets: &Vec<TargetDescription>,
    artifacts: &Vec<String>,
  ) -> anyhow::Result<PackAction> {
    let mut action = action.clone();
    
    if action.target.as_ref().is_some_and(|t| !targets.contains(&t)) {
      if !inquire::Confirm::new(
        &format!(
          "Action `{}` may be not fully compatible with your project due to requirements (Action's target: {}, your project's: {:?}). Use this Action anyway? If no, Action will be skipped. (y/n)",
          info2str_simple(&self.info),
          target2str_simple(action.target.as_ref().unwrap()),
          targets.iter().map(|t| target2str_simple(&t)).collect::<Vec<_>>(),
        )
      ).prompt()? {
        return Ok(PackAction::default())
      }
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, artifacts)?; }
    Ok(action)
  }
  
  fn setup_deploylike_action(
    &self,
    action: &DeployAction,
    deploy_toolkit: &Option<String>,
    artifacts: &Vec<String>,
  ) -> anyhow::Result<DeployAction> {
    let mut action = action.clone();
    if action.deploy_toolkit.as_ref().is_some_and(|l| deploy_toolkit.as_ref().is_some_and(|r| l.as_str() != r.as_str())) {
      if !inquire::Confirm::new(
        &format!(
          "Action `{}` may be not fully compatible with your project due to requirements (Action's deploy toolkit: {}, your project's: {}). Use this Action anyway? If no, Action will be skipped. (y/n)",
          info2str_simple(&self.info),
          action.deploy_toolkit.as_ref().unwrap(),
          deploy_toolkit.as_ref().unwrap(),
        )
      ).prompt()? {
        return Ok(DeployAction::default())
      }
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, artifacts)?; }
    
    Ok(action)
  }
  
  pub(crate) fn prompt_setup_for_project(
    &self,
    langs: &Vec<ProgrammingLanguage>,
    deploy_toolkit: &Option<String>,
    targets: &Vec<TargetDescription>,
    artifacts: &Vec<String>,
  ) -> anyhow::Result<Self> {
    let action = match &self.action {
      Action::Custom(cmd) => Action::Custom(cmd.prompt_setup_for_project(&self.info, artifacts)?),
      Action::Build(b_action) => Action::Build(self.setup_buildlike_action(b_action, langs, artifacts)?),
      Action::PostBuild(pb_action) => Action::PostBuild(self.setup_buildlike_action(pb_action, langs, artifacts)?),
      Action::Test(t_action) => Action::Test(self.setup_buildlike_action(t_action, langs, artifacts)?),
      Action::ProjectClean(pc_action) => Action::ProjectClean(self.setup_projectclean_action(pc_action, artifacts)?),
      Action::Pack(p_action) => Action::Pack(self.setup_packlike_action(p_action, targets, artifacts)?),
      Action::Deliver(p_action) => Action::Deliver(self.setup_packlike_action(p_action, targets, artifacts)?),
      Action::Install(p_action) => Action::Install(self.setup_packlike_action(p_action, targets, artifacts)?),
      Action::ConfigureDeploy(cd_action) => Action::ConfigureDeploy(self.setup_deploylike_action(cd_action, deploy_toolkit, artifacts)?),
      Action::Deploy(d_action) => Action::Deploy(self.setup_deploylike_action(d_action, deploy_toolkit, artifacts)?),
      Action::PostDeploy(pd_action) => Action::PostDeploy(self.setup_deploylike_action(pd_action, deploy_toolkit, artifacts)?),
      
      another => another.clone(),
    };
    
    let mut described_action = self.clone();
    described_action.action = action;
    
    Ok(described_action)
  }
}

/// Команда, исполняемая в командной строке `bash`.
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub(crate) struct CustomCommand {
  /// Команда.
  pub(crate) bash_c: String,
  /// Если истинно, то ошибки выполнения команды будут игнорированы.
  pub(crate) ignore_fails: bool,
  /// Содержит плейсхолдер для артефактов, если есть.
  pub(crate) af_placeholder: Option<String>,
  /// Список файлов, которые нужно подставлять вместо плейсхолдера. Команда будет выполнена столько раз, сколько указано артефактов.
  pub(crate) replace_af_with: Vec<String>,
}

impl CustomCommand {
  pub(crate) fn prompt_setup_for_project(
    &self,
    info: &ActionInfo,
    targets: &Vec<String>,
  ) -> anyhow::Result<Self> {
    use inquire::MultiSelect;
    
    if self.af_placeholder.is_none() { return Ok(self.clone()) }
    
    let targets = targets.clone();
    let selected = MultiSelect::new(&format!("Select artifacts to use with `{}` Action:", info2str_simple(info)), targets).prompt()?;
    
    let mut r = self.clone();
    r.replace_af_with = selected;
    Ok(r)
  }
}

/// Команда, проверяющая вывод на определённое условие.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct CheckAction {
  pub(crate) command: CustomCommand,
  #[serde(serialize_with = "regex2str", deserialize_with = "str2regex")]
  pub(crate) fails_when_found: Regex,
  #[serde(serialize_with = "regex2str", deserialize_with = "str2regex")]
  pub(crate) fails_when_not_found: Regex,
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

/// Параметры инициализации проекта из шаблона.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct TemplateInitializationAction {
  /// Папка с шаблоном проекта.
  pub(crate) template_folder: String,
  /// Языки проекта.
  pub(crate) langs: Vec<ProgrammingLanguage>,
  /// Плейсхолдер, который будет заменён во всех указанных файлах.
  pub(crate) project_name_placeholder: (String, Vec<String>),
  /// Дополнительные команды по инициализации проекта.
  pub(crate) additional_commands: Vec<CustomCommand>,
}

/// Зависимость, которую можно переиспользовать в проектах.
#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct DescribedDependency {
  pub(crate) title: String,
  pub(crate) desc: String,
  pub(crate) supported_langs: Vec<ProgrammingLanguage>,
  pub(crate) tags: Vec<String>,
  pub(crate) add_action: DependencyAdditionAction,
  pub(crate) patch_action: DependencyPatchAction,
}

/// Добавление текущего проекта в качестве зависимости
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DependencyRegistrationAction {
  pub(crate) title: String,
  pub(crate) desc: String,
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) dep_info: DependencyInfo,
  pub(crate) supported_langs: Vec<ProgrammingLanguage>,
  pub(crate) tags: Vec<String>,
}

#[derive(Debug, Clone, Hash)]
pub(crate) struct DependencyInfo {
  pub(crate) short_name: String,
  pub(crate) version: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DependencyUse {
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) info: DependencyInfo,
  pub(crate) inplace_type: SimpleInplaceType,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub(crate) enum SimpleInplaceType {
  #[default]
  Symlink,
  Copy,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DependencyRemovalAction {
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) info: DependencyInfo,
}

/// Параметры добавления зависимости для проекта.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum DependencyAdditionAction {
  GitLink(GitLinkOptions),
  Folder(String),
  Symlink(String),
  DeployerDependency(DependencyUse),
  DeployerArtifact(UseArtifactAction),
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct GitLinkOptions {
  pub(crate) link: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) branch: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) tag: Option<String>,
  pub(crate) init_with_submodules: SubmoduleInitializationRules,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub(crate) enum SubmoduleInitializationRules {
  #[default]
  All,
  OnlyThese(Vec<String>),
  None,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct DependencyPatchAction {
  pub(crate) apply_patches: Vec<PatchRules>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum PatchRules {
  GitPatch(String),
  CustomCommandPatch(CustomCommand),
}

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
  pub(crate) version: OsVersion,
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
pub(crate) enum OsVersion {
  #[default]
  NotSpecified,
  /// Даже если указана версия, при несоответствии версий может заработать.
  WeakSpecified(String),
  StrongSpecified(String),
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
  pub(crate) commands: Vec<CustomCommand>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct RegisterArtifactAction {
  pub(crate) name: String,
  pub(crate) desc: String,
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) artifact_info: ArtifactInfo,
  pub(crate) tags: Vec<String>,
  pub(crate) inplace: HashMap<String, String>,
}

pub(crate) type ArtifactInfo = DependencyInfo;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct UseArtifactAction {
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) artifact_info: ArtifactInfo,
  pub(crate) inplace: HashMap<String, String>,
}

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
    println!("• {} {} (tags: {})", action_info.blue().bold(), action_title.green().bold(), action.tags.join(", ").as_str().blue().italic());
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
  use inquire::{Text, Select, Confirm};
  
  let actions = &mut globals.actions_registry;
  
  if let Some(from_file) = &args.from {
    let action = read_checked::<DescribedAction>(from_file).map_err(|e| {
      println!("Can't read provided Action file due to: {}", e.to_string());
      exit(1);
    }).unwrap();
    actions.insert(info2str_simple(&action.info), action.clone());
    return Ok(action)
  }
  
  let short_name = Text::new("Write the Action's short name:").prompt()?;
  let version = Text::new("Specify the Action's version:").prompt()?;
  
  let info = ActionInfo { short_name, version };
  
  if actions.contains_key(&info2str_simple(&info)) {
    if !Confirm::new(&format!("Actions Registry already have `{}` Action. Do you want to override it? (y/n)", info2str_simple(&info))).prompt()? {
      exit(0);
    }
  }
  
  let name = Text::new("Write the Action's full name:").prompt()?;
  let desc = Text::new("Write the Action's description:").prompt()?;
  
  let tags: Vec<String> = tags_custom_type("Write Action's tags, if any:").prompt()?;
  
  let action_types: Vec<&str> = vec![
    "Interrupt",
    "Custom",
    "Check",
    "Init with template",
    "Register dependency",
    "Delete dependency",
    "Add dependency",
    "Patch dependency",
    "Pre-build",
    "Build",
    "Post-build",
    "Test",
    "ProjectClean",
    "Pack",
    "Deliver",
    "Install",
    "Configure deploy",
    "Deploy",
    "Post-deploy",
    "Observe",
    "Register artifact",
    "Use artifact",
    "Use artifact with command",
  ];
  
  let selected_action_type = Select::new("Select Action's type (read the docs for details):", action_types).prompt()?;
  
  let action = match selected_action_type {
    "Interrupt" => Action::Interrupt,
    "Custom" => {
      let command = new_custom_command(true)?;
      Action::Custom(command)
    },
    "Check" => {
      let command = new_custom_command(false)?;
      let fails_when_found = Text::new("Enter regex pattern that indicates failure when found:")
        .prompt()
        .map(|s| Regex::new(&s).unwrap())?;
      let fails_when_not_found = Text::new("Enter regex pattern that indicates failure when not found:")
        .prompt()
        .map(|s| Regex::new(&s).unwrap())?;
      Action::Check(CheckAction {
        command,
        fails_when_found,
        fails_when_not_found,
      })
    },
    "Init with template" => {
      let template_folder = Text::new("Enter template folder path:").prompt()?;
      let langs = select_programming_languages()?;
      let placeholder = Text::new("Enter project name placeholder:").prompt()?;
      let files = Text::new("Enter comma-separated list of files to apply placeholder to:")
        .prompt()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())?;
      let additional_commands = collect_multiple_commands(false)?;
      
      Action::InitWithTemplate(TemplateInitializationAction {
        template_folder,
        langs,
        project_name_placeholder: (placeholder, files),
        additional_commands,
      })
    },
    "Register dependency" => {
      let title = Text::new("Enter dependency title:").prompt()?;
      let desc = Text::new("Enter dependency description:").prompt()?;
      let dep_short_name = Text::new("Enter dependency short name:").prompt()?;
      let dep_version = Text::new("Enter dependency version:").prompt()?;
      let supported_langs = select_programming_languages()?;
      let dep_tags = tags_custom_type("Enter dependency tags:").prompt()?;
      
      Action::RegisterDependency(DependencyRegistrationAction {
        title,
        desc,
        dep_info: DependencyInfo {
          short_name: dep_short_name,
          version: dep_version,
        },
        supported_langs,
        tags: dep_tags,
      })
    },
    "Delete dependency" => {
      let short_name = Text::new("Enter dependency short name:").prompt()?;
      let version = Text::new("Enter dependency version:").prompt()?;
      
      Action::DeleteDependency(DependencyRemovalAction {
        info: DependencyInfo { short_name, version },
      })
    },
    "Add dependency" => {
      let options = vec!["Git Link", "Folder", "Symlink", "Deployer Dependency", "Deployer Artifact"];
      let selected = Select::new("Select dependency type:", options).prompt()?;
      
      match selected {
        "Git Link" => {
          let link = Text::new("Enter git repository URL:").prompt()?;
          let branch = if Confirm::new("Specify branch?").prompt()? {
            Some(Text::new("Enter branch name:").prompt()?)
          } else {
            None
          };
          let tag = if Confirm::new("Specify tag?").prompt()? {
            Some(Text::new("Enter tag name:").prompt()?)
          } else {
            None
          };
          
          let init_rules = Select::new(
            "Select submodule initialization rules:",
            vec!["All", "Only These", "None"]
          ).prompt()?;
          
          let init_with_submodules = match init_rules {
            "All" => SubmoduleInitializationRules::All,
            "Only These" => {
              let submodules = Text::new("Enter comma-separated list of submodules:")
                .prompt()
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())?;
              SubmoduleInitializationRules::OnlyThese(submodules)
            },
            "None" => SubmoduleInitializationRules::None,
            _ => unreachable!(),
          };
          
          Action::AddDependency(DependencyAdditionAction::GitLink(GitLinkOptions {
            link,
            branch,
            tag,
            init_with_submodules,
          }))
        },
        "Folder" => {
          let path = Text::new("Enter folder path:").prompt()?;
          Action::AddDependency(DependencyAdditionAction::Folder(path))
        },
        "Symlink" => {
          let path = Text::new("Enter symlink path:").prompt()?;
          Action::AddDependency(DependencyAdditionAction::Symlink(path))
        },
        "Deployer Dependency" => {
          let short_name = Text::new("Enter dependency short name:").prompt()?;
          let version = Text::new("Enter dependency version:").prompt()?;
          let inplace_type = Select::new(
            "Select inplace type:",
            vec!["Symlink", "Copy"]
          ).prompt()?;
          
          Action::AddDependency(DependencyAdditionAction::DeployerDependency(DependencyUse {
            info: DependencyInfo { short_name, version },
            inplace_type: match inplace_type {
              "Symlink" => SimpleInplaceType::Symlink,
              "Copy" => SimpleInplaceType::Copy,
              _ => unreachable!(),
            },
          }))
        },
        "Deployer Artifact" => {
          let short_name = Text::new("Enter artifact short name:").prompt()?;
          let version = Text::new("Enter artifact version:").prompt()?;
          // let inplace = collect_key_value_pairs("Enter inplace mappings (empty to finish):")?;
          
          Action::AddDependency(DependencyAdditionAction::DeployerArtifact(UseArtifactAction {
            artifact_info: ArtifactInfo { short_name, version },
            inplace: hmap!(),
          }))
        },
        _ => unreachable!(),
      }
    },
    "Patch dependency" => {
      let mut patches = Vec::new();
      while Confirm::new("Add patch?").prompt()? {
        let patch_type = Select::new(
          "Select patch type:",
          vec!["Git Patch", "Custom Command"]
        ).prompt()?;
        
        let patch = match patch_type {
          "Git Patch" => {
            let path = Text::new("Enter patch file path:").prompt()?;
            PatchRules::GitPatch(path)
          },
          "Custom Command" => {
            let command = new_custom_command(false)?;
            PatchRules::CustomCommandPatch(command)
          },
          _ => unreachable!(),
        };
        patches.push(patch);
      }
      
      Action::PatchDependency(DependencyPatchAction {
        apply_patches: patches,
      })
    },
    action_type @ ("Pre-build" | "Build" | "Post-build" | "Test") => {
      let supported_langs = select_programming_languages()?;
      let commands = collect_multiple_commands(if action_type == "Pre-build" { false } else { true })?;
      
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
    "ProjectClean" => {
      let to_remove = Text::new("Enter comma-separated list of paths to remove:")
        .prompt()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())?;
      let additional_commands = collect_multiple_commands(true)?;
      
      Action::ProjectClean(ProjectCleanAction {
        to_remove,
        additional_commands,
      })
    },
    action_type @ ("Pack" | "Deliver" | "Install") => {
      let target = collect_target()?;
      let commands = collect_multiple_commands(true)?;
      
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
      let commands = collect_multiple_commands(true)?;
      
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
      let commands = collect_multiple_commands(true)?;
      
      Action::Observe(ObserveAction { tags, commands })
    },
    "Register artifact" => {
      let name = Text::new("Enter artifact name:").prompt()?;
      let desc = Text::new("Enter artifact description:").prompt()?;
      let short_name = Text::new("Enter artifact short name:").prompt()?;
      let version = Text::new("Enter artifact version:").prompt()?;
      let tags = tags_custom_type("Enter artifact tags:").prompt()?;
      // let inplace = collect_key_value_pairs("Enter inplace mappings (empty to finish):")?;
      
      Action::RegisterArtifact(RegisterArtifactAction {
        name,
        desc,
        artifact_info: ArtifactInfo { short_name, version },
        tags,
        inplace: hmap!(),
      })
    },
    "Use artifact" => {
      let short_name = Text::new("Enter artifact short name:").prompt()?;
      let version = Text::new("Enter artifact version:").prompt()?;
      let inplace = collect_key_value_pairs("Enter inplace mappings (empty to finish):")?;
      
      Action::UseArtifact(UseArtifactAction {
        artifact_info: ArtifactInfo { short_name, version },
        inplace,
      })
    },
    "Use artifact with command" => {
      let command = new_custom_command(true)?;
      Action::UseArtifactWith(command)
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
  
  actions.insert(info2str_simple(&described_action.info), described_action.clone());
  
  Ok(described_action)
}

/// Создаёт новую команду.
pub(crate) fn new_custom_command(with_af: bool) -> anyhow::Result<CustomCommand> {
  use inquire::{Text, Confirm};
  
  let af_placeholder = if with_af {
    Text::new("Specify a placeholder (any unique text) in place of which artifacts will be substituted during assembly: (or hit `esc`)").prompt_skippable()?
  } else { None };
  let bash_c = Text::new("Enter bash command:").prompt()?;
  let ignore_fails = Confirm::new("Ignore command failures? (y/n)").prompt()?;
  
  Ok(CustomCommand { bash_c, ignore_fails, af_placeholder, ..Default::default() })
}

/// Создаёт несколько новых команд.
fn collect_multiple_commands(with_af: bool) -> anyhow::Result<Vec<CustomCommand>> {
  use inquire::Confirm;
  
  let mut commands = Vec::new();
  while Confirm::new("Add command? (y/n)").prompt()? {
    commands.push(new_custom_command(with_af)?);
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

pub(crate) fn collect_target() -> anyhow::Result<TargetDescription> {
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
    "Not Specified" => OsVersion::NotSpecified,
    "Weak Specified" => {
      let ver = Text::new("Enter version:").prompt()?;
      OsVersion::WeakSpecified(ver)
    },
    "Strong Specified" => {
      let ver = Text::new("Enter version:").prompt()?;
      OsVersion::StrongSpecified(ver)
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

fn collect_key_value_pairs(prompt: &str) -> anyhow::Result<HashMap<String, String>> {
  use inquire::Text;
  
  let mut map = HashMap::new();
  loop {
    let key = Text::new(prompt).prompt()?;
    if key.is_empty() {
      break;
    }
    let value = Text::new("Enter value:").prompt()?;
    map.insert(key, value);
  }
  Ok(map)
}

pub(crate) fn cat_action(
  globals: &DeployerGlobalConfig,
  args: &CatActionArgs,
) -> anyhow::Result<()> {
  let action = match globals.actions_registry.get(&args.action_short_info_and_version) {
    None => exit(1),
    Some(action) => action,
  };
  
  let action_yaml = serde_yaml::to_string(&action).unwrap();
  println!("{}", action_yaml);
  
  Ok(())
}
