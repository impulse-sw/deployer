use serde::{Deserialize, Serialize};

use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub(crate) struct DeployAction {
  pub(crate) deploy_toolkit: Option<String>,
  pub(crate) tags: Vec<String>,
  pub(crate) commands: Vec<CustomCommand>,
}

pub(crate) type ConfigureDeployAction = DeployAction;
pub(crate) type PostDeployAction = DeployAction;

impl Execute for DeployAction {
  fn execute(&self, curr_dir: &std::path::Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for cmd in &self.commands {
      let (status, out) = cmd.execute(curr_dir)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}
