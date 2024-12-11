use serde::{Deserialize, Serialize};

use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ObserveAction {
  pub(crate) tags: Vec<String>,
  pub(crate) command: CustomCommand,
}

impl Execute for ObserveAction {
  fn execute(&self, curr_dir: &std::path::Path) -> anyhow::Result<(bool, Vec<String>)> {
    self.command.execute(curr_dir)
  }
}
