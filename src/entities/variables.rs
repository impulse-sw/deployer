use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::entities::traits::Edit;
use crate::hmap;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct Variable {
  pub(crate) title: String,
  pub(crate) is_secret: bool,
  pub(crate) value: VarValue,
}

impl<'a> Variable {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let title = inquire::Text::new("Enter your variable's title:").prompt()?;
    println!("{} if variable is a secret, then no command containing this variable will be printed during the build stage.", "Note:".green().italic());
    let is_secret = inquire::Confirm::new("Is this variable a secret?").with_default(false).prompt()?;
    
    // TBD
    let plain = inquire::Text::new("Enter the variable's content:").prompt()?;
    
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
  
  pub(crate) fn get_value(&'a self) -> anyhow::Result<&'a str> {
    match &self.value {
      VarValue::Plain(val) => Ok(val.as_str()),
      // VarValue::FromEnvFile(_) => unimplemented!(),
    }
  }
  
  pub(crate) fn edit_variable_from_prompt(&mut self) -> anyhow::Result<()> {
    let actions = vec![
      "Edit title",
      "Change secret flag",
      "Edit value",
    ];
    
    while let Some(action) = inquire::Select::new(
      "Select an edit action (hit `esc` when done):",
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        "Edit title" => self.title = inquire::Text::new("Write the Action's full name:").prompt()?,
        "Change secret flag" => self.is_secret = inquire::Confirm::new("Is this variable a secret?").with_default(false).prompt()?,
        "Edit value" => self.value = VarValue::Plain(inquire::Text::new("Enter the variable's content:").prompt()?),
        _ => {},
      }
    }
    
    Ok(())
  }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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
        let s = format!("Edit variable `{}`", c.title.green());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&["Add variable".to_string(), "Remove variable".to_string()]);
      
      if let Some(action) = inquire::Select::new("Select a concrete variable to change (hit `esc` when done):", cs).prompt_skippable()? {
        match action.as_str() {
          "Add variable" => self.add_item()?,
          "Remove variable" => self.remove_item()?,
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
      let s = format!("Variable `{}`", c.title.green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new("Select a variable to remove:", cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}
