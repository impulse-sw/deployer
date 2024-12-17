use serde::{Deserialize, Serialize};

use crate::entities::{
  environment::BuildEnvironment,
  custom_command::CustomCommand,
  programming_languages::ProgrammingLanguage,
  traits::Execute,
};

#[derive(Deserialize, Serialize, PartialEq, Default, Clone, Debug)]
pub(crate) struct BuildAction {
  pub(crate) supported_langs: Vec<ProgrammingLanguage>,
  pub(crate) commands: Vec<CustomCommand>,
}

pub(crate) type PreBuildAction = BuildAction;
pub(crate) type PostBuildAction = BuildAction;
pub(crate) type TestAction = BuildAction;

impl Execute for BuildAction {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for cmd in &self.commands {
      let (status, out) = cmd.execute(env)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}
