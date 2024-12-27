use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::entities::traits::Edit;
use crate::hmap;
use crate::i18n;

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub(crate) struct Variable {
  pub(crate) title: String,
  pub(crate) is_secret: bool,
  pub(crate) value: VarValue,
}

impl Variable {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let title = inquire::Text::new(i18n::VAR_TITLE).prompt()?;
    println!("{}: {}", i18n::NOTE.green().italic(), i18n::VAR_NOTE);
    let is_secret = inquire::Confirm::new(i18n::VAR_IS_SECRET).with_default(false).prompt()?;
    
    // TBD
    let plain = inquire::Text::new(i18n::VAR_CONTENT).prompt()?;
    
    Ok(Variable {
      title,
      is_secret,
      value: VarValue::Plain(plain),
    })
  }
  
  pub(crate) fn new_plain(title: &str, value: &str) -> Self {
    Self {
      title: title.to_string(),
      is_secret: false,
      value: VarValue::Plain(value.to_string()),
    }
  }
  
  pub(crate) fn get_value(&self) -> anyhow::Result<&str> {
    match &self.value {
      VarValue::Plain(val) => Ok(val.as_str()),
      // VarValue::FromEnvFile(_) => unimplemented!(),
    }
  }
  
  pub(crate) fn edit_variable_from_prompt(&mut self) -> anyhow::Result<()> {
    let actions = vec![
      i18n::EDIT_TITLE,
      i18n::EDIT_VAR_SECRET,
      i18n::EDIT_VALUE,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        i18n::EDIT_TITLE => self.title = inquire::Text::new(i18n::VAR_TITLE).prompt()?,
        i18n::EDIT_VAR_SECRET => self.is_secret = inquire::Confirm::new(i18n::VAR_IS_SECRET).with_default(false).prompt()?,
        i18n::EDIT_VALUE => self.value = VarValue::Plain(inquire::Text::new(i18n::VAR_CONTENT).prompt()?),
        _ => {},
      }
    }
    
    Ok(())
  }
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub(crate) enum VarValue {
  Plain(String),
  // FromEnvFile(FromEnvFile),
  // TBD
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct FromEnvFile {
  pub(crate) env_file_path: String,
  pub(crate) key: String,
}

pub(crate) trait VarTraits {
  fn is_secret(&self, title: &str) -> bool;
  fn titles(&self) -> Vec<String>;
  fn find(&self, title: &str) -> Option<Variable>;
}

impl VarTraits for [Variable] {
  fn is_secret(&self, title: &str) -> bool {
    self.iter().find(|v| v.title.as_str().eq(title)).is_some_and(|v| v.is_secret)
  }
  
  fn titles(&self) -> Vec<String> {
    self.iter().map(|v| v.title.to_owned()).collect::<Vec<_>>()
  }
  
  fn find(&self, title: &str) -> Option<Variable> {
    self.iter().find(|v| v.title.as_str().eq(title)).cloned()
  }
}

impl Edit for Vec<Variable> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::VAR_EDIT, c.title.green());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::VAR_SELECT_FC, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_variable_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(Variable::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("{} `{}`", i18n::VAR, c.title.green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::VAR_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}
