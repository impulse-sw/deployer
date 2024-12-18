use std::path::Path;

#[derive(Clone, Copy)]
pub(crate) struct BuildEnvironment<'a> {
  pub(crate) build_dir: &'a Path,
  pub(crate) cache_dir: &'a Path,
  pub(crate) artifacts_dir: &'a Path,
  pub(crate) new_build: bool,
  pub(crate) silent_build: bool,
  pub(crate) no_pipe: bool,
}
