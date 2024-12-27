use crate::tr;

// Check
tr!(CHECK_IGNORE_FAILS, "Does the command failure also means check failure?");

tr!(CHECK_CURR_REGEX, "Current regexes are:");
tr!(SPECIFY_REGEX_SUCC, "Specify success when found some regex?");
tr!(SPECIFY_REGEX_FAIL, "Specify success when NOT found some regex?");
tr!(SPECIFY_REGEX_FOR_SUCC, "for success on match");
tr!(SPECIFY_REGEX_FOR_FAIL, "for success on mismatch");
tr!(CHECK_NEED_TO_AT_LEAST, "For `Check` Action you need to specify at least one regex check!");
tr!(CHECK_SPECIFY_WHAT, "Specify an action for Check Action:");
tr!(CHECK_EDIT_CMD, "Edit check command");
tr!(CHECK_EDIT_REGEXES, "Edit regexes");
tr!(CHECK_ENTER_REGEX, "Enter regex");
tr!(CHECK_HELP, "(or enter '/h' for help)");
tr!(CHECK_REGEX_INVALID_DUE, "The regex you've written is invalid due to");

tr!(GUIDE, "Guide");

tr!(CHECK_GUIDE_TITLE, "Regex Checks for Deployer");
tr!(CHECK_GUIDE_1, "The usage of regex checks in Deployer is simple enough.");
tr!(CHECK_GUIDE_2, "If you want to specify some text that needed to be found, you simply write this text.");
tr!(CHECK_GUIDE_3, "For finding an info about any supported regex read this");
tr!(CHECK_GUIDE_4, "For checks use this");
tr!(CHECK_GUIDE_5, "select `Rust` flavor at left side panel");

tr!(PATTERN, "Pattern");
tr!(FOUND, "found");
tr!(NOT_FOUND, "not found");

// Custom Command
tr!(CMD_SPECIFY_BASH_C, "Enter a command for terminal");
tr!(CMD_PLACEHOLDERS, "Enter command placeholders, if any:");
tr!(CMD_IGNORE_FAILS, "Ignore command failures?");
tr!(CMD_SHOW_BASH_C, "Show an entire command at build stage?");
tr!(CMD_SHOW_SUCC_OUT, "Show an output of command if it executed successfully?");
tr!(CMD_ONLY_WHEN_FRESH, "Start a command only in fresh builds?");

tr!(CUSTOM_CMD_GUIDE_TITLE, "Shell Commands for Deployer");
tr!(CUSTOM_CMD_GUIDE_1, "The usage of shell commands in Deployer is very simple.");
tr!(CUSTOM_CMD_GUIDE_2, "You can use `%1%` for home directories, your default `%2%` variable and so on.");
tr!(CUSTOM_CMD_GUIDE_3, "Also you can write your commands even when there are some unspecified variables:");
tr!(CUSTOM_CMD_GUIDE_4, "To specify shell for Deployer, use `DEPLOYER_SH_PATH` environment variable.");
tr!(CUSTOM_CMD_GUIDE_5, "By default: using");
tr!(CUSTOM_CMD_GUIDE_6, "Now: using");

tr!(CUSTOM_CMD_EDIT, "Edit command");
tr!(CUSTOM_CMD_REORDER, "Reorder commands");
tr!(CUSTOM_CMD_ADD, "Add command");
tr!(CUSTOM_CMD_RM, "Remove command");

tr!(CMD_SPECIFY_VARS, "Specifying variables for `{}` Action:");
tr!(CMD_SELECT_TO_REPLACE, "Select variable to replace `{1}` in `{2}` shell command:");
tr!(CMD_HIDDEN_VAR, "At build stage the command will be hidden due to usage of secret variable.");
tr!(CMD_ONE_MORE_TIME, "Enter `y` if you need exec this command one more time with others variables.");
tr!(CMD_SELECT_TO_CHANGE, "Select an option to change in `{}` command");
tr!(CMD_SELECT_TO_REMOVE, "Select a command to remove:");

tr!(CMD_EDIT_SHELL, "Edit shell command");
tr!(CMD_CHANGE_PLACEHOLDERS, "Change command placeholders");
tr!(CMD_CHANGE_FAILURE_IGNORANCE, "Change command failure ignorance");
tr!(CMD_CHANGE_VISIBILITY_AT_BUILD, "Change whether command is displayed or not on build stage");
tr!(CMD_CHANGE_VISIBILITY_ON_SUCC, "Change whether command output is displayed or not when it executed successfully");
tr!(CMD_CHANGE_ON_FRESH, "Change command executing only at fresh builds");

tr!(CMDS_REORDER, "Reorder Action's commands:");

tr!(CMD_SKIP_DUE_TO_NOT_FRESH, "Skip a command due to not a fresh build...");
tr!(EXECUTING, "Executing");
tr!(EXECUTING_HIDDEN, "Executing the command:");
tr!(ERRORS, "Errors:");

tr!(HIT_ESC, "(hit `esc` when done)");
tr!(OR_HIT_ESC, "(or hit `esc`)");

tr!(CUSTOM_CMD_EDIT_PROMPT, "Select a concrete command to change (hit `esc` when done):");

// Project clean
tr!(PC_TO_REMOVE, "Enter comma-separated list of paths to remove:");

// Observe
tr!(OBSERVE_TAGS, "Enter observe tags:");

// PL
tr!(PL_INPUT_PROMPT, "Input the programming language name:");
tr!(LANGUAGE, "Language");

tr!(PL_ACTION_PROMPT, "Select a concrete language to change");
tr!(ADD, "Add");
tr!(REMOVE, "Remove");
tr!(REORDER, "Reorder");
tr!(PL_TO_REMOVE, "Select a language to remove:");
tr!(PL_SELECT, "Select programming languages:");
tr!(PL_COLLECT, "Enter the names of programming languages separated by commas:");

// Targets
tr!(TARGET_ARCH, "Enter the target's architecture:");
tr!(TARGET_OS_SELECT, "Select OS:");
tr!(TARGET_OS_UNIX_LIKE, "Enter Unix-like OS name:");
tr!(TARGET_OS_OTHER, "Enter OS name:");
tr!(TARGET_OS_DER, "Enter OS derivative:");
tr!(TARGET_OS_VER_S, "Select version specification type:");

tr!(TARGET_OS_VER_NS, "Not Specified");
tr!(TARGET_OS_VER_WS, "Weak Specified");
tr!(TARGET_OS_VER_SS, "Strong Specified");

tr!(TARGET_OS_VER, "Enter version:");

tr!(EDIT_ACTION_PROMPT, "Select an edit action");
tr!(EDIT_ARCH, "Edit arch");
tr!(EDIT_OS, "Edit OS");
tr!(EDIT_TITLE, "Edit title");
tr!(EDIT_VAR_SECRET, "Change secret flag");
tr!(EDIT_VALUE, "Edit value");

// Variables
tr!(VAR, "Variable");
tr!(VAR_TITLE, "Enter your variable's title:");
tr!(NOTE, "Note");
tr!(VAR_NOTE, "if variable is a secret, then no command containing this variable will be printed during the build stage.");
tr!(VAR_IS_SECRET, "Is this variable a secret?");
tr!(VAR_CONTENT, "Enter the variable's content:");

tr!(VAR_EDIT, "Edit variable");
tr!(VAR_SELECT_FC, "Select a concrete variable to change");
tr!(VAR_TO_REMOVE, "Select a variable to remove:");

tr!(VAR_SPECIFY_ANOTHER, "· Specify another variable");
tr!(ACTION_SPECIFY_ANOTHER, "· Specify another Action");
tr!(PIPELINE_SPECIFY_ANOTHER, "· Specify another Pipeline");

// Actions
tr!(ACTION_SHORT_NAME, "Write the Action's short name:");
tr!(ACTION_VERSION, "Specify the Action's version:");
tr!(ACTION_FULL_NAME, "Write the Action's full name:");
tr!(ACTION_DESC, "Write the Action's description:");
tr!(ACTION_TAGS, "Write Action's tags, if any:");

tr!(ACTION_SELECT_TYPE, "Select Action's type (read the docs for details):");
tr!(ACTION_REG_ALREADY_HAVE, "Actions Registry already have `{}` Action. Do you want to override it? (y/n)");
tr!(ACTION_COMPAT_PLS, "Action `{1}` may be not fully compatible with your project due to requirements (Action's supported langs: {2}, your project's: {3}). Use this Action anyway? If no, Action will be skipped. (y/n)");
tr!(ACTION_COMPAT_TARGETS, "Action `{1}` may be not fully compatible with your project due to requirements (Action's target: {2}, your project's: {3}). Use this Action anyway? If no, Action will be skipped. (y/n)");
tr!(ACTION_COMPAT_DEPL_TOOLKIT, "Action `{1}` may be not fully compatible with your project due to requirements (Action's deploy toolkit: {2}, your project's: {3}). Use this Action anyway? If no, Action will be skipped. (y/n)");

tr!(ACTION_SELECT_TO_CHANGE, "Select a concrete Action to change");
tr!(ACTION_EDIT, "Edit Action `{1}` - `{2}`");
tr!(ACTION_REMOVE, "Remove Action `{1}` - `{2}`");
tr!(ACTION, "Action `{1}` - `{2}`");
tr!(ACTION_ADD, "Add Action?");

tr!(ACTION_CHOOSE_TO_ADD, "Choose an Action to add:");
tr!(ACTION_CHOOSE_TO_REMOVE, "Select an Action to remove:");
tr!(ACTION_REGISTRY_CHOOSE_TO_REMOVE, "Select Action for removing from Actions' Registry:");

tr!(ARE_YOU_SURE, "Are you sure? (y/n)");

tr!(ADD_CMD, "Add command?");
tr!(EDIT_COMMAND, "Edit command");
tr!(EDIT_COMMANDS, "Edit commands");
tr!(EDIT_PLS, "Edit programming languages");
tr!(EDIT_TARGETS, "Edit targets");
tr!(EDIT_DEPL_TOOLKIT, "Edit deploy toolkit");
tr!(EDIT_DESC, "Edit description");
tr!(EDIT_TAGS, "Edit tags");
tr!(EDIT_PC_FILES, "Edit files and folders to remove");
tr!(EDIT_EXCL_TAG, "Edit exclusive execution tag");
tr!(EDIT_PIPELINE_ACTIONS, "Edit Pipeline's Actions");
tr!(EDIT_PROJECT_NAME, "Edit project name");
tr!(EDIT_PROJECT_PIPELINES, "Edit project Pipelines");
tr!(EDIT_PROJECT_REASSIGN, "Reassign project variables to Actions");
tr!(EDIT_CACHE, "Edit cache files");
tr!(EDIT_PROJECT_VARS, "Edit project variables");
tr!(EDIT_ARTIFACTS, "Edit artifacts");
tr!(EDIT_AF_INPLACE, "Edit artifact inplacements");

tr!(DEPL_TOOLKIT, "Enter deploy toolkit name");

tr!(ACTIONS_AVAILABLE, "Available Actions in Deployer's Registry:");
tr!(NO_ACTIONS, "There is no Actions in Registry.");

tr!(TAGS, "tags");

// Pipelines
tr!(PIPELINE_SHORT_NAME, "Write the Pipeline's short name:");
tr!(PIPELINE_VERSION, "Specify the Pipeline's version:");
tr!(PIPELINE_FULL_NAME, "Write the Pipeline's full name:");
tr!(PIPELINE_DESC, "Write the Pipeline's description:");
tr!(PIPELINE_TAGS, "Write Pipeline's tags, if any:");

tr!(PIPELINE_SPECIFY_EXCL_TAG, "Specify exclusive pipeline tag");

tr!(PIPELINES_AVAILABLE, "Available Pipelines in Deployer's Registry:");
tr!(NO_PIPELINES, "There is no Pipelines in Registry.");

tr!(PIPELINE_REG_ALREADY_HAVE, "Pipelines Registry already have `{}` Pipeline. Do you want to override it? (y/n)");
tr!(REORDER_PIPELINE_ACTIONS, "Reorder Pipeline's Actions:");
tr!(SELECT_ACTION_TO_ADD_TO, "Select Action for adding to Pipeline:");
tr!(PIPELINE_DESCRIBE_ACTION_IN, "Describe this Action inside your Pipeline:");

tr!(GOT_FROM, "Got from");

tr!(PIPELINE_CHOOSE_TO_ADD, "Choose a Pipeline to add:");
tr!(PIPELINE_CHOOSE_TO_REMOVE, "Select a Pipeline to remove:");
tr!(PIPELINE_REGISTRY_CHOOSE_TO_REMOVE, "Select Pipeline for removing from Pipeline's Registry:");
tr!(PIPELINES_REORDER, "Reorder Pipelines inside your project:");

tr!(NO_SUCH_PIPELINE, "There is no such Pipeline in Registry. See available Pipelines with `deployer ls pipelines`.");

tr!(CFG_INVALID, "Config is invalid! Reinit the project.");

tr!(PIPELINE_SELECT_FOR_PROJECT, "Select the Pipeline for this project:");
tr!(PIPELINE_SHORT_NAME_FOR_PROJECT, "Write the Pipeline's short name (only for this project):");
tr!(PIPELINE_NEW_DEFAULT, "Set this Pipeline running by default? (y/n)");
tr!(PIPELINE_NEW_DEFAULT_REPLACE, "Pipeline `{}` is already set by default. Set this Pipeline running by default instead?");
tr!(PIPELINE_DEFAULT_SET, "Pipeline is successfully set up for this project.");

tr!(PIPELINE_SHORT_NAME_FOR_PROJECT_OVERRIDE, "Do you want to overwrite an existing pipeline `{}` for this project? (y/n)");
tr!(PIPELINE_EDIT, "Edit Pipeline `{1}` - `{2}`");
tr!(SELECT_PIPELINE_TO_CHANGE, "Select a concrete Pipeline to change");
tr!(PIPELINE, "Pipeline");
tr!(PIPELINE_REORDER_ACTIONS, "Reorder Pipeline's Actions:");
tr!(PIPELINE_REMOVE, "Remove Pipeline `{1}` - `{2}`");

tr!(STARTING_PIPELINE, "Starting the `{}` Pipeline...");
tr!(STARTING_ACTION, "Action");
tr!(ARTIFACTS_ENPLACED, "Artifacts are enplaced successfully.");
tr!(INTERRUPT, "The Pipeline is interrupted. Hit `Enter` to continue");

tr!(BUILD_PATH, "Build path");

tr!(DONE, " done");
tr!(GOT_ERROR, " got an error!");

// Project
tr!(PROJECT_NAME, "Enter the project's name:");
tr!(PROJECT_SPECIFY_PLS, "Please, specify the project's programming languages to setup default cache folders.");
tr!(PROJECT_DEPL_TOOLKIT, "Specify your deploy toolkit (`docker`, `docker-compose`, `podman`, `k8s`, etc.)");

tr!(ENTITY, "Entity");
tr!(NEW_VALUE, "Input new value:");
tr!(VALUE_TO_REMOVE, "Select a value to remove:");
tr!(INPLACEMENT, "Inplacement");

tr!(SELECT_PROJECT_AF, "Select project's artifact:");
tr!(CHOOSE_AF_INPLACEMENT, "Enter relative path of artifact inplacement (inside `artifacts` subfolder):");
tr!(REMOVE_INPLACEMENT, "Select an inplacement to remove:");
tr!(TARGET, "Target");
tr!(EDIT_TARGET, "Edit target");
tr!(SELECT_TARGET_TO_CHANGE, "Select a concrete target to change");
tr!(SELECT_TARGET_TO_REMOVE, "Select a target to remove:");
tr!(ADD_NEW_TARGET, "Add new build target?");

tr!(AF_RELATIVE_PATH, "Enter the artifact's relative path:");
tr!(ADD_NEW_AF, "Add new build/deploy artifact?");
tr!(ADD_NEW_VAR, "Add new project-related variable or secret?");
tr!(ADD_NEW_INPLACEMENT_FIRST, "Do you want to create artifact inplacement from build directory to your project's location (inside `artifacts` subfolder)?");
tr!(ADD_NEW_INPLACEMENT_SECOND, "Add one more artifact inplacement?");

tr!(INIT_SUCC, "Setup is completed. Don't forget to assign at least one Pipeline to the project to build/deploy!");
