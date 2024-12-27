use serde::{Deserialize, Serialize};

use crate::i18n;

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
    
    let arch = Text::new(i18n::TARGET_ARCH).prompt()?;
    
    let os = Select::new(
      i18n::TARGET_OS_SELECT,
      vec!["Android", "iOS", "Linux", "Unix-like", "Windows", "macOS", "Other"]
    ).prompt()?;
    
    let os_variant = match os {
      "Android" => OsVariant::Android,
      "iOS" => OsVariant::iOS,
      "Linux" => OsVariant::Linux,
      "Unix-like" => {
        let name = Text::new(i18n::TARGET_OS_UNIX_LIKE).prompt()?;
        OsVariant::UnixLike(name)
      },
      "Windows" => OsVariant::Windows,
      "macOS" => OsVariant::macOS,
      "Other" => {
        let name = Text::new(i18n::TARGET_OS_OTHER).prompt()?;
        OsVariant::Other(name)
      },
      _ => unreachable!(),
    };
    
    let derivative = Text::new(i18n::TARGET_OS_DER).prompt()?;
    
    let version_type = Select::new(
      i18n::TARGET_OS_VER_S,
      vec![i18n::TARGET_OS_VER_NS, i18n::TARGET_OS_VER_WS, i18n::TARGET_OS_VER_SS]
    ).prompt()?;
    
    let version = match version_type {
      i18n::TARGET_OS_VER_NS => OsVersionSpecification::No,
      i18n::TARGET_OS_VER_WS => {
        let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
        OsVersionSpecification::Weak(ver)
      },
      i18n::TARGET_OS_VER_SS => {
        let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
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
      i18n::EDIT_ARCH,
      i18n::EDIT_OS,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      use inquire::{Select, Text};
      
      match action {
        i18n::EDIT_ARCH => self.arch = Text::new(i18n::TARGET_ARCH).prompt()?,
        i18n::EDIT_OS => {
          let os = Select::new(
            i18n::TARGET_OS_SELECT,
            vec!["Android", "iOS", "Linux", "Unix-like", "Windows", "macOS", "Other"]
          ).prompt()?;
          
          self.os = match os {
            "Android" => OsVariant::Android,
            "iOS" => OsVariant::iOS,
            "Linux" => OsVariant::Linux,
            "Unix-like" => {
              let name = Text::new(i18n::TARGET_OS_UNIX_LIKE).prompt()?;
              OsVariant::UnixLike(name)
            },
            "Windows" => OsVariant::Windows,
            "macOS" => OsVariant::macOS,
            "Other" => {
              let name = Text::new(i18n::TARGET_OS_OTHER).prompt()?;
              OsVariant::Other(name)
            },
            _ => unreachable!(),
          };
          
          self.derivative = Text::new(i18n::TARGET_OS_DER).prompt()?;
          
          let version_type = Select::new(
            i18n::TARGET_OS_VER_S,
            vec![i18n::TARGET_OS_VER_NS, i18n::TARGET_OS_VER_WS, i18n::TARGET_OS_VER_SS]
          ).prompt()?;
          
          self.version = match version_type {
            i18n::TARGET_OS_VER_NS => OsVersionSpecification::No,
            i18n::TARGET_OS_VER_WS => {
              let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
              OsVersionSpecification::Weak(ver)
            },
            i18n::TARGET_OS_VER_SS => {
              let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
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
