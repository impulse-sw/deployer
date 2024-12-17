use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::hmap;
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
    let bash_c = specify_bash_c()?;
    
    let placeholders = tags_custom_type("Enter command placeholders, if any:").prompt()?;
    let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
    
    let ignore_fails = inquire::Confirm::new("Ignore command failures?").with_default(false).prompt()?;
    let show_bash_c = inquire::Confirm::new("Show an entire command at build stage?").with_default(true).prompt()?;
    let show_success_output = inquire::Confirm::new("Show an output of command if it executed successfully?").with_default(false).prompt()?;
    let only_when_fresh = if inquire::Confirm::new("Start a command only in fresh builds?").with_default(false).prompt()? {
      Some(true)
    } else {
      None
    };
    
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
  
  pub(crate) fn prompt_setup_for_project(
    &self,
    info: &ActionInfo,
    variables: &[Variable],
    artifacts: &[String],
  ) -> anyhow::Result<Self> {
    use inquire::{Confirm, Select, Text};
    
    const USE_ANOTHER: &str = "· Specify another variable";
    
    if self.placeholders.as_ref().is_none_or(|ps| ps.is_empty()) { return Ok(self.clone()) }
    
    println!("Specifying variables for `{}` Action:", info2str_simple(info).blue());
    
    let mut all_variables = variables.titles();
    all_variables.extend_from_slice(artifacts);
    all_variables.push(USE_ANOTHER.to_string());
    
    let mut replacements = vec![];
    let mut explicitly_show_bash_c = None;
    loop {
      let mut replacement = vec![];
      for placeholder in self.placeholders.as_ref().unwrap() {
        let mut selected = Select::new(
          &format!("Select variable to replace `{}` in `{}` bash command:", placeholder.green(), self.bash_c.green()),
          all_variables.clone(),
        ).prompt()?;
        
        if variables.is_secret(selected.as_str()) {
          println!("At build stage the command will be hidden due to usage of secret variable.");
          explicitly_show_bash_c = Some(false);
        }
        
        if selected.as_str() == USE_ANOTHER {
          selected = Text::new(&format!("Enter variable to replace `{}` in `{}` bash command:", placeholder.green(), self.bash_c.green())).prompt()?;
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
      if !Confirm::new("Enter `y` if you need exec this command one more time with others variables.").with_default(false).prompt()? { break }
    }
    
    let mut r = self.clone();
    r.replacements = Some(replacements);
    r.show_bash_c = if let Some(show) = explicitly_show_bash_c { show } else { r.show_bash_c };
    Ok(r)
  }
  
  pub(crate) fn edit_command_from_prompt(&mut self) -> anyhow::Result<()> {
    while let Some(action) = inquire::Select::new(
      &format!("Select an option to change in `{}` command (hit `esc` when done):", self.bash_c.green()),
      vec![
        "Edit bash command",
        "Change command placeholders",
        "Change command failure ignorance",
        "Change whether command is displayed or not on build stage",
        "Change whether command output is displayed or not when it executed successfully",
      ],
    ).prompt_skippable()? {
      match action {
        "Edit bash command" => self.bash_c = specify_bash_c()?,
        "Change command placeholders" => {
          let placeholders = tags_custom_type("Enter command placeholders, if any:").prompt()?;
          self.placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
        },
        "Change command failure ignorance" => {
          self.ignore_fails = inquire::Confirm::new("Ignore command failures?").with_default(false).prompt()?;
        },
        "Change whether command is displayed or not on build stage" => {
          self.show_bash_c = inquire::Confirm::new("Show an entire command at build stage?").with_default(true).prompt()?;
        },
        "Change whether command output is displayed or not when it executed successfully" => {
          self.show_success_output = inquire::Confirm::new("Show an output of command if it executed successfully?").with_default(false).prompt()?;
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

pub(crate) fn specify_bash_c() -> anyhow::Result<String> {
  let mut bash_c;
  loop {
    bash_c = inquire::Text::new("Enter typical bash command (or enter '/h' for help):").prompt()?;
    if bash_c.as_str() != "/h" { break }
    println!("Guide: `{}`", "Bash Commands for Deployer".blue());
    println!(">>> The usage of `bash` commands in Deployer is very simple.");
    println!(">>> You can use `{}` for home directories, your default `{}` variable and so on.", "~".green(), "PATH".green());
    println!(">>> ");
    println!(">>> Also you can write your commands even when there are some unspecified variables:");
    println!(">>> `{}`", "g++ <input-file> -o <output-file>".green());
    println!(">>> `{}{}`", "docker compose run -e DEPLOY_KEY=".green(), "{{my very secret key}}".red());
  }
  
  Ok(bash_c)
}

impl Edit for Vec<CustomCommand> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("Edit command `{}`", c.bash_c.green());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&["Reorder commands".to_string(), "Add command".to_string(), "Remove command".to_string()]);
      
      if let Some(action) = inquire::Select::new("Select a concrete command to change (hit `esc` when done):", cs).prompt_skippable()? {
        match action.as_str() {
          "Reorder commands" => self.reorder()?,
          "Add command" => self.add_item()?,
          "Remove command" => self.remove_item()?,
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
    
    let reordered = ReorderableList::new("Reorder Action's commands:", k).prompt()?;
    
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
    
    let selected = inquire::Select::new("Select a command to remove:", cs.clone()).prompt()?;
    
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
      output.push("Skip a command due to not a fresh build...".to_string());
      return Ok((true, output))
    }
    
    if self.placeholders.is_some() && let Some(replacements) = &self.replacements {
      for every_start in replacements {
        let mut bash_c = self.bash_c.to_owned();
        
        for (from, to) in every_start {
          bash_c = bash_c.replace(from, to.get_value()?);
        }
        
        let bash_c_info = format!(r#"/bin/bash -c "{}""#, bash_c).green();
        
        let command_output = std::process::Command::new("/bin/bash")
          .current_dir(env.build_dir)
          .arg("-c")
          .arg(&bash_c)
          .stdout(std::process::Stdio::piped())
          .stderr(std::process::Stdio::piped())
          .spawn()
          .map_err(|e| anyhow::anyhow!("Can't execute `{}` due to: {}", bash_c_info, e))?
          .wait_with_output()
          .map_err(|e| anyhow::anyhow!("Can't wait for output `{}` due to: {}", bash_c_info, e))?;
        
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
        
        if !self.ignore_fails && !command_output.status.success() {
          return Ok((false, output))
        }
      }
    } else {
      let bash_c_info = format!(r#"/bin/bash -c "{}""#, self.bash_c.as_str()).green();
      
      let command_output = std::process::Command::new("/bin/bash")
        .current_dir(env.build_dir)
        .arg("-c")
        .arg(self.bash_c.as_str())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Can't execute `{}` due to: {}", bash_c_info, e))?
        .wait_with_output()
        .map_err(|e| anyhow::anyhow!("Can't wait for output `{}` due to: {}", bash_c_info, e))?;
      
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
      
      if !self.ignore_fails && !command_output.status.success() {
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
      output.push(format!("Executing `{}`:", bash_c_info));
    } else {
      output.push("Executing the command:".to_string());
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
    if total != 0 && !success { output.push(format!("{}", "Errors:".red().bold())); }
    
    for (i, line) in stderr.split('\n').enumerate() {
      if i == total && line.trim().is_empty() { break }
      output.push(format!(">>> {}", line));
    }
  }
  
  output
}
