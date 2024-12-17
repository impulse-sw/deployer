use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;
use crate::utils::{regexopt2str, str2regexopt, str2regex_simple};

/// Команда, проверяющая вывод на определённое условие.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct CheckAction {
  pub(crate) command: CustomCommand,
  #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt")]
  pub(crate) success_when_found: Option<Regex>,
  #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt")]
  pub(crate) success_when_not_found: Option<Regex>,
}

impl CheckAction {
  pub(crate) fn change_regexes_from_prompt(&mut self) -> anyhow::Result<()> {
    println!("Current regexes are:");
    println!("`success_when_found` = {:?}", self.success_when_found);
    println!("`success_when_not_found` = {:?}", self.success_when_not_found);
    
    loop {
      if inquire::Confirm::new("Specify success when found some regex?").with_default(true).prompt()? {
        self.success_when_found = Some(specify_regex("for success on match")?);
      }
      
      if inquire::Confirm::new("Specify success when NOT found some regex?").with_default(true).prompt()? {
        self.success_when_not_found = Some(specify_regex("for success on mismatch")?);
      }
      
      if self.success_when_found.is_some() || self.success_when_not_found.is_some() { break }
      else { println!("For `Check` Action you need to specify at least one regex check!"); }
    }
    
    Ok(())
  }
}

impl Execute for CheckAction {
  fn execute(&self, curr_dir: &std::path::Path) -> anyhow::Result<(bool, Vec<String>)> {
    let mut output = vec![];
    
    let (status, command_out) = self.command.execute(curr_dir)?;
    if !status && !self.command.ignore_fails {
      return Ok((false, command_out))
    }
    
    if let Some(re) = &self.success_when_found {
      let text = command_out.join("\n");
      if re.is_match(text.as_str()) { output.push(format!("Pattern `{}` found!", re.as_str().green())); }
      else {
        output.push(format!("Pattern `{}` not found!", re.as_str().green()));
        return Ok((false, output))
      }
    }
    
    if let Some(re) = &self.success_when_not_found {
      let text = command_out.join("\n");
      if !re.is_match(text.as_str()) { output.push(format!("Pattern `{}` not found!", re.as_str().green())); }
      else {
        output.push(format!("Pattern `{}` found!", re.as_str().green()));
        return Ok((false, output))
      }
    }
    
    Ok((true, output))
  }
}

pub(crate) fn specify_regex(for_what: &str) -> anyhow::Result<Regex> {
  let mut regex_str;
  
  loop {
    regex_str = inquire::Text::new(
      &format!("Enter regex {} (or enter '/h' for help):", for_what)
    ).prompt()?;
    
    if let Err(e) = Regex::new(&regex_str) {
      println!("The regex you've written is invalid due to: {:?}.", e);
      continue
    }
    
    if regex_str.as_str() != "/h" { break }
    println!("Guide: `{}`", "Regex Checks for Deployer".blue());
    println!(">>> The usage of regex checks in Deployer is simple enough.");
    println!(">>> If you want to specify some text that needed to be found, you simply write this text.");
    println!(">>> ");
    println!(">>> For any supported regex read this: {}", "https://docs.rs/regex/latest/regex/".blue());
    println!(">>> For checks use this: {} (select `Rust` flavor at left side panel)", "https://regex101.com/".blue());
  }
  
  str2regex_simple(regex_str.as_str())
}
