use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::hmap;
use crate::i18n;
use crate::entities::environment::BuildEnvironment;
use crate::entities::variables::{Variable, VarTraits};
use crate::entities::info::{ActionInfo, info2str_simple};
use crate::entities::traits::{Edit, Execute};
use crate::utils::tags_custom_type;

/// Команда, исполняемая в командной строке `bash`.
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) struct CustomCommand {
  /// Команда.
  pub(crate) bash_c: String,
  
  /// Плейсхолдеры команды. Используются для подстановки значений при выполнении действия.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) placeholders: Option<Vec<String>>,
  /// Список переменных для подстановки вместо плейсхолдеров.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) replacements: Option<Vec<Vec<(String, Variable)>>>,
  
  /// Игнорировать ли ошибки команды.
  pub(crate) ignore_fails: bool,
  /// Отображать ли вывод команды, если не возникла ошибка.
  pub(crate) show_success_output: bool,
  /// Отображать ли команду.
  /// 
  /// Потенциально команда может содержать уязвимые переменные, такие как: ключи, пароли, пути к чувствительным файлам и т.д.
  /// Их можно скрыть при сборке, если указать `false`.
  pub(crate) show_bash_c: bool,
  /// Запускать ли действие только при новых сборках.
  pub(crate) only_when_fresh: Option<bool>,
}

impl CustomCommand {
  /// Создаёт новую команду.
  pub(crate) fn new_from_prompt() -> anyhow::Result<CustomCommand> {
    let bash_c = specify_bash_c(None)?;
    
    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
    
    let ignore_fails = inquire::Confirm::new(i18n::CMD_IGNORE_FAILS).with_default(false).prompt()?;
    let show_bash_c = inquire::Confirm::new(i18n::CMD_SHOW_BASH_C).with_default(true).prompt()?;
    let show_success_output = inquire::Confirm::new(i18n::CMD_SHOW_SUCC_OUT).with_default(false).prompt()?;
    let only_when_fresh = Some(inquire::Confirm::new(i18n::CMD_ONLY_WHEN_FRESH).with_default(false).prompt()?);
    
    Ok(CustomCommand {
      bash_c,
      placeholders,
      ignore_fails,
      show_bash_c,
      show_success_output,
      only_when_fresh,
      replacements: None,
    })
  }
  
  pub(crate) fn new_from_prompt_unspecified() -> anyhow::Result<CustomCommand> {
    let bash_c = specify_bash_c(None)?;
    
    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
    
    Ok(CustomCommand {
      bash_c,
      placeholders,
      ignore_fails: true,
      show_success_output: true,
      show_bash_c: false,
      only_when_fresh: Some(false),
      replacements: None,
    })
  }
  
  pub(crate) fn prompt_setup_for_project(
    &self,
    info: &ActionInfo,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<Self> {
    use inquire::{Confirm, Select, Text};
    
    const USE_ANOTHER: &str = i18n::VAR_SPECIFY_ANOTHER;
    
    if self.placeholders.as_ref().is_none_or(|ps| ps.is_empty()) { return Ok(self.clone()) }
    
    println!("{}", i18n::CMD_SPECIFY_VARS.replace("{}", &info2str_simple(info).blue()));
    
    let mut all_variables = variables.titles();
    all_variables.extend_from_slice(artifacts);
    all_variables.push(USE_ANOTHER.to_string());
    
    let mut replacements = vec![];
    let mut explicitly_show_bash_c = None;
    loop {
      let mut replacement = vec![];
      for placeholder in self.placeholders.as_ref().unwrap() {
        let mut selected = Select::new(
          &i18n::CMD_SELECT_TO_REPLACE.replace("{1}", &placeholder.green()).replace("{2}", &self.bash_c.green()),
          all_variables.clone(),
        ).prompt()?;
        
        if variables.is_secret(selected.as_str()) {
          println!("{}", i18n::CMD_HIDDEN_VAR);
          explicitly_show_bash_c = Some(false);
        }
        
        if selected.as_str() == USE_ANOTHER {
          selected = Text::new(&i18n::CMD_SELECT_TO_REPLACE.replace("{1}", &placeholder.green()).replace("{2}", &self.bash_c.green())).prompt()?;
        }
        
        replacement.push(
          (
            placeholder.to_owned(),
            variables
              .find(&selected)
              .unwrap_or_else(|| Variable::new_plain(&selected, &selected)),
          )
        );
      }
      
      replacements.push(replacement);
      if !Confirm::new(i18n::CMD_ONE_MORE_TIME).with_default(false).prompt()? { break }
    }
    
    let mut r = self.clone();
    r.replacements = Some(replacements);
    r.show_bash_c = if let Some(show) = explicitly_show_bash_c { show } else { r.show_bash_c };
    Ok(r)
  }
  
  pub(crate) fn edit_command_from_prompt(&mut self) -> anyhow::Result<()> {
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::CMD_SELECT_TO_CHANGE.replace("{}", &self.bash_c.green()), i18n::HIT_ESC),
      vec![
        i18n::CMD_EDIT_SHELL,
        i18n::CMD_CHANGE_PLACEHOLDERS,
        i18n::CMD_CHANGE_FAILURE_IGNORANCE,
        i18n::CMD_CHANGE_VISIBILITY_AT_BUILD,
        i18n::CMD_CHANGE_VISIBILITY_ON_SUCC,
        i18n::CMD_CHANGE_ON_FRESH,
      ],
    ).prompt_skippable()? {
      match action {
        i18n::CMD_EDIT_SHELL => self.bash_c = specify_bash_c(Some(self.bash_c.as_str()))?,
        i18n::CMD_CHANGE_PLACEHOLDERS => {
          let placeholders = if let Some(phs) = &self.placeholders {
            let joined = phs.join(", ");
            tags_custom_type(i18n::CMD_PLACEHOLDERS, Some(joined.as_str())).prompt()?
          } else {
            tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?
          };
          self.placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
        },
        i18n::CMD_CHANGE_FAILURE_IGNORANCE => {
          self.ignore_fails = inquire::Confirm::new(i18n::CMD_IGNORE_FAILS).with_default(false).prompt()?;
        },
        i18n::CMD_CHANGE_VISIBILITY_AT_BUILD => {
          self.show_bash_c = inquire::Confirm::new(i18n::CMD_SHOW_BASH_C).with_default(true).prompt()?;
        },
        i18n::CMD_CHANGE_VISIBILITY_ON_SUCC => {
          self.show_success_output = inquire::Confirm::new(i18n::CMD_SHOW_SUCC_OUT).with_default(false).prompt()?;
        },
        i18n::CMD_CHANGE_ON_FRESH => {
          self.only_when_fresh = if inquire::Confirm::new(i18n::CMD_ONLY_WHEN_FRESH).with_default(false).prompt()? {
            Some(true)
          } else {
            None
          };
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

pub(crate) fn specify_bash_c(default: Option<&str>) -> anyhow::Result<String> {
  let mut bash_c;
  loop {
    let prompt = format!("{} {}:", i18n::CMD_SPECIFY_BASH_C, i18n::CHECK_HELP);
    let mut text_prompt = inquire::Text::new(prompt.as_str());
    if let Some(default) = default { text_prompt = text_prompt.with_initial_value(default); }
    bash_c = text_prompt.prompt()?;
    if bash_c.as_str() != "/h" { break }
    println!("{}: `{}`", i18n::GUIDE, i18n::CUSTOM_CMD_GUIDE_TITLE.blue());
    println!(">>> {}", i18n::CUSTOM_CMD_GUIDE_1);
    println!(">>> {}", i18n::CUSTOM_CMD_GUIDE_2.replace("%1%", &"~".green()).replace("%2%", &"PATH".green()));
    println!(">>> ");
    println!(">>> {}", i18n::CUSTOM_CMD_GUIDE_3);
    println!(">>> `{}`", "g++ <input-file> -o <output-file>".green());
    println!(">>> `{}{}`", "docker compose run -e DEPLOY_KEY=".green(), "{{my very secret key}}".red());
    println!(">>> ");
    println!(">>> {}", i18n::CUSTOM_CMD_GUIDE_4);
    println!(">>> {} `{}`.", i18n::CUSTOM_CMD_GUIDE_5, "/bin/bash".green());
    
    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => format!("`{}`", path.green()),
      Err(_) => format!("\"\" (`{}`)", "/bin/bash".green()),
    };
    
    println!(">>> {} {}", i18n::CUSTOM_CMD_GUIDE_6, shell);
  }
  
  Ok(bash_c)
}

impl Edit for Vec<CustomCommand> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::CUSTOM_CMD_EDIT, c.bash_c.green());
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::CUSTOM_CMD_REORDER.to_string(), i18n::CUSTOM_CMD_ADD.to_string(), i18n::CUSTOM_CMD_RM.to_string()]);
      
      if let Some(action) = inquire::Select::new(
        &format!("{} {}:", i18n::CUSTOM_CMD_EDIT_PROMPT, i18n::HIT_ESC),
        cs,
      ).prompt_skippable()? {
        match action.as_str() {
          i18n::CUSTOM_CMD_REORDER => self.reorder()?,
          i18n::CUSTOM_CMD_ADD => self.add_item()?,
          i18n::CUSTOM_CMD_RM => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_command_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> {
    use inquire::ReorderableList;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for selected_command in self.iter() {
      let key = format!("`{}`", selected_command.bash_c);
      k.push(key.clone());
      h.insert(key, selected_command);
    }
    
    let reordered = ReorderableList::new(i18n::CMDS_REORDER, k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(CustomCommand::new_from_prompt()?);
    
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("`{}`", c.bash_c.green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::CMD_SELECT_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    
    Ok(())
  }
}

impl Execute for CustomCommand {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut output = vec![];
    
    if !env.new_build && self.only_when_fresh.is_some_and(|v| v) {
      if *crate::rw::VERBOSE.wait() {
        output.push(i18n::CMD_SKIP_DUE_TO_NOT_FRESH.to_string());
      }
      return Ok((true, output))
    }
    
    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => path,
      Err(_) => "/bin/bash".to_string(),
    };
    
    if self.placeholders.is_some() && let Some(replacements) = &self.replacements {
      for every_start in replacements {
        let mut bash_c = self.bash_c.to_owned();
        
        for (from, to) in every_start { bash_c = bash_c.replace(from, to.get_value()?); }
        
        let bash_c_info = format!(r#"{} -c "{}""#, shell, bash_c).green();
        let mut cmd = std::process::Command::new(&shell);
        cmd.current_dir(env.build_dir).arg("-c").arg(&bash_c);
        
        if !env.no_pipe { cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()); }
        
        let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("Can't execute command due to: {}", e))?;
        
        let success = if env.no_pipe {
          let res = child.wait().map_err(|e| anyhow::anyhow!("Can't wait for exit status due to: {}", e))?;
          res.success()
        } else {
          let command_output = child.wait_with_output().map_err(|e| anyhow::anyhow!("Can't wait for output due to: {}", e))?;
          
          let stdout_strs = String::from_utf8_lossy_owned(command_output.stdout);
          let stderr_strs = String::from_utf8_lossy_owned(command_output.stderr);
          output.extend_from_slice(&compose_output(
            bash_c_info.to_string(),
            stdout_strs,
            stderr_strs,
            command_output.status.success(),
            self.show_success_output,
            self.show_bash_c,
          ));
          
          command_output.status.success()
        };
        
        if !self.ignore_fails && !success {
          return Ok((false, output))
        }
      }
    } else {
      let bash_c_info = format!(r#"{} -c "{}""#, shell, self.bash_c.as_str()).green();
      let mut cmd = std::process::Command::new(&shell);
      cmd.current_dir(env.build_dir).arg("-c").arg(&self.bash_c);
      
      if !env.no_pipe { cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()); }
      
      let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("Can't execute command due to: {}", e))?;
      
      let success = if env.no_pipe {
        let res = child.wait().map_err(|e| anyhow::anyhow!("Can't wait for exit status due to: {}", e))?;
        res.success()
      } else {
        let command_output = child.wait_with_output().map_err(|e| anyhow::anyhow!("Can't wait for output due to: {}", e))?;
        
        let stdout_strs = String::from_utf8_lossy_owned(command_output.stdout);
        let stderr_strs = String::from_utf8_lossy_owned(command_output.stderr);
        output.extend_from_slice(&compose_output(
          bash_c_info.to_string(),
          stdout_strs,
          stderr_strs,
          command_output.status.success(),
          self.show_success_output,
          self.show_bash_c,
        ));
        
        command_output.status.success()
      };
      
      if !self.ignore_fails && !success {
        return Ok((false, output))
      }
    }
    
    Ok((true, output))
  }
}

fn compose_output(
  bash_c_info: String,
  stdout: String,
  stderr: String,
  success: bool,
  show_success_output: bool,
  show_bash_c: bool,
) -> Vec<String> {
  let mut output = vec![];
  
  if success && !show_success_output { return output }
  
  if !stdout.trim().is_empty() || !stderr.trim().is_empty() {
    if show_bash_c {
      output.push(format!("{} `{}`:", i18n::EXECUTING, bash_c_info));
    } else {
      output.push(i18n::EXECUTING_HIDDEN.to_string());
    }
  }
  if !stdout.trim().is_empty() {
    let total = stdout.chars().filter(|c| *c == '\n').count();
    
    for (i, line) in stdout.split('\n').enumerate() {
      if i == total && line.trim().is_empty() { break }
      output.push(format!(">>> {}", line));
    }
  }
  if !stderr.trim().is_empty() {
    let total = stderr.chars().filter(|c| *c == '\n').count();
    if total != 0 && !success { output.push(format!("{}", i18n::ERRORS.red().bold())); }
    
    for (i, line) in stderr.split('\n').enumerate() {
      if i == total && line.trim().is_empty() { break }
      output.push(format!(">>> {}", line));
    }
  }
  
  if
    let Ok(num) = std::env::var("DEPLOYER_TRIM_ERR_OUT_LINES") &&
    let Ok(num) = num.parse::<usize>() &&
    num <= output.len() &&
    !success
  {
    output[(output.len()-1-num)..(output.len()-1)].to_vec()
  } else {
    output
  }
}
