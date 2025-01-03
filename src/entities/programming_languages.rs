use serde::{Deserialize, Serialize};

use crate::hmap;
use crate::i18n;
use crate::entities::traits::Edit;
use crate::utils::tags_custom_type;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub(crate) enum ProgrammingLanguage {
  Rust,
  Go,
  C,
  Cpp,
  Python,
  Other(String),
}

impl ProgrammingLanguage {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let s = inquire::Text::new(i18n::PL_INPUT_PROMPT).prompt()?;
    let pl = match s.as_str() {
      "Rust" => Self::Rust,
      "Go" => Self::Go,
      "C" => Self::C,
      "C++" => Self::Cpp,
      "Python" => Self::Python,
      s => Self::Other(s.to_owned()),
    };
    Ok(pl)
  }
}

impl std::fmt::Display for ProgrammingLanguage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let lang = match self {
      Self::Rust => "Rust".to_string(),
      Self::Go => "Go".to_string(),
      Self::C => "C".to_string(),
      Self::Cpp => "C++".to_string(),
      Self::Python => "Python".to_string(),
      Self::Other(s) => s.to_owned(),
    };
    
    f.write_str(&lang)
  }
}

impl Edit for Vec<ProgrammingLanguage> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::LANGUAGE, c);
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::PL_ACTION_PROMPT, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(ProgrammingLanguage::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("`{}`", c);
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::PL_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

/// Парсит вводимые языки программирования.
pub(crate) fn specify_programming_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  use inquire::MultiSelect;
  
  let langs = vec!["Rust", "Go", "C", "C++", "Python", "Others"];
  let selected = MultiSelect::new(i18n::PL_SELECT, langs).prompt()?;
  
  let mut result = Vec::new();
  for lang in selected {
    let lang = match lang {
      "Rust" => ProgrammingLanguage::Rust,
      "Go" => ProgrammingLanguage::Go,
      "C" => ProgrammingLanguage::C,
      "C++" => ProgrammingLanguage::Cpp,
      "Python" => ProgrammingLanguage::Python,
      "Others" => {
        let langs = collect_multiple_languages()?;
        result.extend_from_slice(&langs);
        continue
      }
      _ => unreachable!(),
    };
    result.push(lang);
  }
  
  Ok(result)
}

fn collect_multiple_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  let langs = tags_custom_type(i18n::PL_COLLECT, None).prompt()?;
  let mut v = vec![];
  
  for lang in langs {
    match lang.as_str() {
      "Rust" | "Go" | "C" | "C++" | "Python" => continue,
      lang => v.push(ProgrammingLanguage::Other(lang.to_owned())),
    }
  }
  
  Ok(v)
}
