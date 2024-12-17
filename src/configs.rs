use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::actions::{DescribedAction, Action, buildlike::BuildAction};
use crate::pipelines::DescribedPipeline;
use crate::entities::{
  custom_command::CustomCommand,
  info::{ActionInfo, info2str_simple},
  targets::TargetDescription,
  programming_languages::ProgrammingLanguage,
  variables::Variable,
};
use crate::hmap;
use crate::utils::ordered_map;

/// Конфигурация проекта.
#[derive(Deserialize, Serialize, PartialEq, Default, Debug)]
pub(crate) struct DeployerProjectOptions {
  /// Название проекта.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) project_name: Option<String>,
  /// Языки
  pub(crate) langs: Vec<ProgrammingLanguage>,
  /// Таргеты
  pub(crate) targets: Vec<TargetDescription>,
  /// Тулкит для развёртывания
  pub(crate) deploy_toolkit: Option<String>,
  
  /// Сборки
  pub(crate) builds: Vec<String>,
  /// Последняя (текущая сборка)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) last_build: Option<String>,
  
  /// Метки кэша
  pub(crate) cache_files: Vec<String>,
  
  /// Пайплайны
  pub(crate) pipelines: Vec<DescribedPipeline>,
  
  /// Артефакты
  pub(crate) artifacts: Vec<String>,
  /// Переменные
  pub(crate) variables: Vec<Variable>,
  /// Правила размещения артефактов
  pub(crate) inplace_artifacts_into_project_root: Vec<(String, String)>,
}

/// Глобальная конфигурация Деплойера.
#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct DeployerGlobalConfig {
  /// Список ведомых проектов.
  pub(crate) projects: Vec<String>,
  /// Список доступных шаблонов проектов.
  pub(crate) templates: Vec<String>,
  /// Реестр доступных действий.
  #[serde(serialize_with = "ordered_map")]
  pub(crate) actions_registry: HashMap<String, DescribedAction>,
  /// Реестр доступных пайплайнов.
  #[serde(serialize_with = "ordered_map")]
  pub(crate) pipelines_registry: HashMap<String, DescribedPipeline>,
  // /// Реестр доступных зависимостей.
  // #[serde(serialize_with = "ordered_map")]
  // pub(crate) dependencies_registry: HashMap<String, DescribedDependency>,
}

impl Default for DeployerGlobalConfig {
  fn default() -> Self {
    let mut actions_registry = hmap!();
    
    let info = ActionInfo { short_name: "cargo-rel".into(), version: "0.1".into() };
    actions_registry.insert(info2str_simple(&info), DescribedAction {
      title: "Cargo Build (Release)".into(),
      desc: "Build the Rust project with Cargo default settings in release mode".into(),
      info,
      tags: vec!["rust".into(), "cargo".into()],
      action: Action::Build(BuildAction {
        supported_langs: vec![ProgrammingLanguage::Rust],
        commands: vec![CustomCommand {
          bash_c: "cargo build --release".into(),
          placeholders: None,
          replacements: None,
          ignore_fails: false,
          show_success_output: false,
          show_bash_c: true,
        }],
      })
    });
    
    let pipelines_registry = hmap!();
    
    Self {
      // dependencies_registry: hmap!(),
      projects: vec![],
      templates: vec![],
      actions_registry,
      pipelines_registry,
    }
  }
}
