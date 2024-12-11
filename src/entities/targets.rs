use serde::{Deserialize, Serialize};

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
