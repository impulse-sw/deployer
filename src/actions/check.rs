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
use crate::i18n;
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
    println!("{}", i18n::CHECK_CURR_REGEX);
    println!("`success_when_found` = {:?}", self.success_when_found);
    println!("`success_when_not_found` = {:?}", self.success_when_not_found);
    
    loop {
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_SUCC).with_default(true).prompt()? {
        self.success_when_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_SUCC)?);
      }
      
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_FAIL).with_default(true).prompt()? {
        self.success_when_not_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_FAIL)?);
      }
      
      if self.success_when_found.is_some() || self.success_when_not_found.is_some() { break }
      else { println!("{}", i18n::CHECK_NEED_TO_AT_LEAST); }
    }
    
    Ok(())
  }
  
  pub(crate) fn edit_check_from_prompt(&mut self) -> anyhow::Result<()> {
    while let Some(selected) = inquire::Select::new(
      i18n::CHECK_SPECIFY_WHAT,
      vec![i18n::CHECK_EDIT_CMD, i18n::CHECK_EDIT_REGEXES],
    ).prompt_skippable()? {
      match selected {
        i18n::CHECK_EDIT_CMD => self.command.edit_command_from_prompt()?,
        i18n::CHECK_EDIT_REGEXES => self.change_regexes_from_prompt()?,
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
      if re.is_match(text.as_str()) { output.push(format!("{} `{}` {}!", i18n::PATTERN, re.as_str().green(), i18n::FOUND)); }
      else {
        output.push(format!("{} `{}` {}!", i18n::PATTERN, re.as_str().green(), i18n::NOT_FOUND));
        return Ok((false, output))
      }
    }
    
    if let Some(re) = &self.success_when_not_found {
      let text = command_out.join("\n");
      if !re.is_match(text.as_str()) { output.push(format!("{} `{}` {}!", i18n::PATTERN, re.as_str().green(), i18n::NOT_FOUND)); }
      else {
        output.push(format!("{} `{}` {}!", i18n::PATTERN, re.as_str().green(), i18n::FOUND));
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
      &format!("{} {} {}:", i18n::CHECK_ENTER_REGEX, for_what, i18n::CHECK_HELP)
    ).prompt()?;
    
    if let Err(e) = Regex::new(&regex_str) {
      println!("{}: {:?}.", i18n::CHECK_REGEX_INVALID_DUE, e);
      continue
    }
    
    if regex_str.as_str() != "/h" { break }
    println!("{}: `{}`", i18n::GUIDE, i18n::CHECK_GUIDE_TITLE.blue());
    println!(">>> {}", i18n::CHECK_GUIDE_1);
    println!(">>> {}", i18n::CHECK_GUIDE_2);
    println!(">>> ");
    println!(">>> {}: {}", i18n::CHECK_GUIDE_3, "https://docs.rs/regex/latest/regex/".blue());
    println!(">>> {}: {} ({})", i18n::CHECK_GUIDE_4, "https://regex101.com/".blue(), i18n::CHECK_GUIDE_5);
  }
  
  str2regex_simple(regex_str.as_str())
}
