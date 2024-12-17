use crate::entities::environment::BuildEnvironment;

pub(crate) trait Edit {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()>;
  fn reorder(&mut self) -> anyhow::Result<()>;
  fn add_item(&mut self) -> anyhow::Result<()>;
  fn remove_item(&mut self) -> anyhow::Result<()>;
}

pub(crate) trait EditExtended<T> {
  fn edit_from_prompt(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn reorder(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn add_item(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn remove_item(&mut self, opts: &mut T) -> anyhow::Result<()>;
}

pub(crate) trait Execute {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)>;
}
