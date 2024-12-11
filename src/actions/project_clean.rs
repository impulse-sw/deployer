use serde::{Deserialize, Serialize};

use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ProjectCleanAction {
  pub(crate) to_remove: Vec<String>,
  pub(crate) additional_commands: Vec<CustomCommand>,
}

impl Execute for ProjectCleanAction {
  fn execute(&self, curr_dir: &std::path::Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for entity in &self.to_remove {
      crate::rw::remove_all(curr_dir.join(entity))?;
    }
    
    for cmd in &self.additional_commands {
      let (status, out) = cmd.execute(curr_dir)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}
