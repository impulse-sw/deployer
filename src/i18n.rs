#[macro_export]
macro_rules! tr {
  ($k:ident, $v:expr) => {
    pub(crate) const $k: &str = $v;
  };
}

#[cfg(not(feature = "i18n-ru"))]
mod en;
#[cfg(not(feature = "i18n-ru"))]
use en as translations;

#[cfg(feature = "i18n-ru")]
mod ru;
#[cfg(feature = "i18n-ru")]
use ru as translations;

pub(crate) use translations::*;
