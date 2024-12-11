use serde::{Deserialize, Serialize};

use crate::entities::{
  custom_command::CustomCommand,
  targets::TargetDescription,
  traits::Execute,
};

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub(crate) struct PackAction {
  pub(crate) target: Option<TargetDescription>,
  pub(crate) commands: Vec<CustomCommand>,
}

pub(crate) type DeliveryAction = PackAction;
pub(crate) type InstallAction = PackAction;

impl Execute for PackAction {
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
