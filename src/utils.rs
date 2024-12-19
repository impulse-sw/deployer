use inquire::CustomType;
use regex::Regex;
use serde::Deserialize;

#[macro_export]
macro_rules! hmap {
  () => {
    std::collections::HashMap::new()
  };
}

pub(crate) fn get_current_working_dir() -> std::io::Result<std::path::PathBuf> {
  std::env::current_dir()
}

pub(crate) fn tags_custom_type<'a>(message: &'a str, default: Option<&'a str>) -> CustomType<'a, Vec<String>> {
  use inquire::ui::RenderConfig;
  
  CustomType {
    message,
    starting_input: if default.is_some() { default } else { None },
    default: Some(vec![]),
    placeholder: None,
    help_message: None,
    formatter: &|val| val.join(", ").to_string(),
    default_value_formatter: &|val| val.join(", ").to_string(),
    parser: &|a| Ok(a.split(',').map(|s| s.trim().to_owned()).collect::<Vec<_>>()),
    validators: CustomType::DEFAULT_VALIDATORS,
    error_message: "Invalid input".into(),
    render_config: RenderConfig::default(),
  }
}

// pub(crate) fn str2target_simple(t: impl AsRef<str>) -> Result<TargetDescription> {
//   
// }

// pub(crate) fn str2regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
// where
//   D: serde::Deserializer<'de>,
// {
//   use serde::de::Error;
//   String::deserialize(deserializer).and_then(|string| {
//     Regex::new(string.as_str()).map_err(|err| Error::custom(err.to_string()))
//   })
// }

pub(crate) fn str2regex_simple(s: &str) -> anyhow::Result<Regex> {
  Ok(Regex::new(s)?)
}

// pub(crate) fn regex2str<S>(v: &Regex, serializer: S) -> Result<S::Ok, S::Error>
// where
//   S: serde::Serializer,
// {
//   serializer.serialize_str(v.as_str())
// }

pub(crate) fn str2regexopt<'de, D>(deserializer: D) -> Result<Option<Regex>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  use serde::de::Error;
  Option::<String>::deserialize(deserializer).and_then(|option| {
    match option {
      Some(string) => Regex::new(string.as_str()).map(Some).map_err(|err| Error::custom(err.to_string())),
      None => Ok(None),
    }
  })
}

pub(crate) fn regexopt2str<S>(v: &Option<Regex>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  match v {
    Some(r) => serializer.serialize_str(r.as_str()),
    None => serializer.serialize_none(),
  }
}

pub(crate) fn ordered_map<S, K: Ord + serde::Serialize, V: serde::Serialize>(value: &std::collections::HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  use serde::Serialize;
  
  let ordered: std::collections::BTreeMap<_, _> = value.iter().collect();
  ordered.serialize(serializer)
}
