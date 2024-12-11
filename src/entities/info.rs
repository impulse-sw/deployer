use serde::Deserialize;

#[derive(Debug, Clone, Hash)]
pub(crate) struct Info {
  pub(crate) short_name: String,
  pub(crate) version: String,
}

pub(crate) type ActionInfo = Info;
pub(crate) type PipelineInfo = Info;

pub(crate) fn str2info<'de, D>(deserializer: D) -> Result<ActionInfo, D::Error>
where
  D: serde::Deserializer<'de>,
{
  use serde::de::Error;
  String::deserialize(deserializer).and_then(|string| {
    let vals = string.split('@').collect::<Vec<_>>();
    if let Some(short_name) = vals.first() && let Some(version) = vals.get(1) {
      Ok(ActionInfo { short_name: short_name.to_string(), version: version.to_string() })
    } else {
      Err(Error::custom("Can't deserialize information!"))
    }
  })
}

pub(crate) fn info2str<S>(v: &ActionInfo, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  serializer.serialize_str(info2str_simple(v).as_str())
}

pub(crate) fn info2str_simple(v: &ActionInfo) -> String {
  format!("{}@{}", v.short_name, v.version)
}
