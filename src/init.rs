use crate::cmd::InitArgs;
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};

pub(crate) fn init(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  _args: &InitArgs,
) -> anyhow::Result<()> {
  let curr_dir = std::env::current_dir().expect("Can't get current dir!").to_str().expect("Can't convert current dir's path to string!").to_owned();
  if !globals.projects.contains(&curr_dir) { globals.projects.push(curr_dir.to_owned()); }
  
  config.init_from_prompt()?;
  
  println!("Setup is completed. Don't forget to assign at least one Pipeline to the project to build/deploy!");
  
  Ok(())
}
