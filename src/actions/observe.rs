use serde::{Deserialize, Serialize};

use crate::entities::environment::BuildEnvironment;
use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) struct ObserveAction {
  pub(crate) tags: Vec<String>,
  pub(crate) command: CustomCommand,
}

impl Execute for ObserveAction {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut observe_env = env;
    observe_env.no_pipe = true;
    self.command.execute(observe_env)
  }
}
