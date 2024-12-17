use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::entities::{
  environment::BuildEnvironment,
  custom_command::CustomCommand,
  info::ActionInfo,
  traits::Execute,
  variables::Variable,
};
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

impl PartialEq for CheckAction {
  fn eq(&self, other: &Self) -> bool {
    self.command.eq(&other.command) &&
    (
      (self.success_when_found.is_none() && other.success_when_found.is_none()) ||
      (self.success_when_found.as_ref().is_some_and(|a| other.success_when_found.as_ref().is_some_and(|b| a.as_str().eq(b.as_str()))))
    ) &&
    (
      (self.success_when_not_found.is_none() && other.success_when_not_found.is_none()) ||
      (self.success_when_not_found.as_ref().is_some_and(|a| other.success_when_not_found.as_ref().is_some_and(|b| a.as_str().eq(b.as_str()))))
    )
  }
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
  
  pub(crate) fn edit_check_from_prompt(&mut self) -> anyhow::Result<()> {
    const EDIT_COMMAND: &str = "Edit check command";
    const EDIT_REGEXES: &str = "Edit regexes";
    
    while let Some(selected) = inquire::Select::new(
      "Specify an action for Check Action:",
      vec![EDIT_COMMAND, EDIT_REGEXES],
    ).prompt_skippable()? {
      match selected {
        EDIT_COMMAND => self.command.edit_command_from_prompt()?,
        EDIT_REGEXES => self.change_regexes_from_prompt()?,
        _ => {},
      }
    }
    
    Ok(())
  }
  
  pub(crate) fn prompt_setup_for_project(
    &self,
    info: &ActionInfo,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<Self> {
    let mut r = self.clone();
    r.command = r.command.prompt_setup_for_project(info, variables, artifacts)?;
    Ok(r)
  }
}

impl Execute for CheckAction {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut output = vec![];
    
    let (status, command_out) = self.command.execute(env)?;
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
