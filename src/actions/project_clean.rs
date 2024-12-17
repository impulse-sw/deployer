use serde::{Deserialize, Serialize};

use crate::entities::environment::BuildEnvironment;
use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) struct ProjectCleanAction {
  pub(crate) to_remove: Vec<String>,
  pub(crate) additional_commands: Vec<CustomCommand>,
}

impl Execute for ProjectCleanAction {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for entity in &self.to_remove {
      crate::rw::remove_all(env.build_dir.join(entity))?;
    }
    
    for cmd in &self.additional_commands {
      let (status, out) = cmd.execute(env)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}
