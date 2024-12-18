# deployer

Deployer is a relative simple, yet powerful localhost CI/CD instrument. It allows you to:

- have your own actions and pipelines repositories (`Actions Registry` and `Pipelines Registry`) in a single JSON file
- create actions and pipelines from TUI or JSON configuration files
- configure actions for specific project
- check compatibility over actions and projects
- and share your project build/deploy settings very quickly and without any dependencies.

## Build

Well, the building process is very easy. You need to install Rust first:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, execute this:

```bash
git clone https://github.com/impulse-sw/deployer.git
cd deployer
cargo install --path .
```

That's it! Now you have `/home/username/.cargo/bin/deployer` binary. Modify the `PATH` variable, if you need to.

## Usage

First of all, let's create a simple action.

```bash
deployer new action
```

For example, let's name it `UPX Compress`. The short name will be `upx-compress`, the version - `0.1.0`.

The full JSON is:

```json
{
  "title": "UPX Compress",
  "desc": "Compress the binary file with UPX.",
  "info": "upx-compress@0.1.0",
  "tags": [
    "upx"
  ],
  "action": {
    "PostBuild": {
      "supported_langs": [
        "Rust",
        "Go",
        "C",
        "Cpp",
        "Python",
        {
          "Other": "any"
        }
      ],
      "commands": [
        {
          "bash_c": "upx <artifact>",
          "placeholders": [
            "<artifact>"
          ],
          "ignore_fails": false,
          "show_success_output": false,
          "show_bash_c": false
        }
      ]
    }
  }
}
```

If you're interesting in UPX, consider to visit it [home page](https://upx.github.io/).

So, let's create a pipeline that will build the binary from the Rust code with preinstalled `cargo-rel@0.1` action and then compress this binary with `upx-compress@0.1.0`.

```bash
deployer new pipeline
```

The full JSON is:

```json
{
  "title": "Rust Enhanced Pipeline",
  "desc": "Build the Rust project with Cargo.",
  "info": "rust-default@0.1.0",
  "tags": [],
  "actions": [
    {
      "title": "Build the project.",
      "desc": "Got from `Cargo Build (Release)`. Build the Rust project with Cargo default settings in release mode",
      "info": "cargo-rel@0.1",
      "tags": [
        "rust",
        "cargo"
      ],
      "action": {
        "Build": {
          "supported_langs": [
            "Rust"
          ],
          "commands": [
            {
              "bash_c": "cargo build --release",
              "ignore_fails": false,
              "af_placeholder": null,
              "replace_af_with": []
            }
          ]
        }
      }
    },
    {
      "title": "Compress the resulting binary.",
      "desc": "Got from `UPX Compress`. Compress the binary file with UPX.",
      "info": "upx-compress@0.1.0",
      "tags": [],
      "action": {
        "PostBuild": {
          "supported_langs": [
            "Rust",
            "Go",
            "C",
            "Cpp",
            {
              "Other": "any"
            }
          ],
          "commands": [
            {
              "bash_c": "upx <artifact>",
              "placeholders": [
                "<artifact>"
              ],
              "ignore_fails": false,
              "show_success_output": false,
              "show_bash_c": false
            }
          ]
        }
      }
    }
  ]
}
```

Note that you can change the inner content of Actions inside Pipelines, and also can change the inner content of Pipelines and their Actions if these Pipelines assigned to your project. The changes will not affect Actions and Pipelines from Deployer's Registries.

You can view your Actions and Pipelines and get it in JSON by simple commands:

```bash
deployer ls actions
deployer ls pipelines

deployer cat action upx-compress@0.1.0
deployer cat pipeline rust-default@0.1.0
```

And, of course, load Actions and Pipelines from JSON files by:

```bash
deployer new action -f {your config}
```

The next step is to init the project and assign the `rust-default@0.1.0` Pipeline to it.

```bash
cd my-rust-project
deployer init
deployer with rust-default@0.1.0
```

Deployer will consider you to specify some things (e.g., targets - for this project and `rust-default@0.1.0` Pipeline it will be `target/release/deployer`). After all you will get this `deploy-config.json`:

```json
{
  "project_name": "my-rust-project",
  "langs": [
    "Rust"
  ],
  "targets": [
    {
      "arch": "x86_64",
      "os": "Linux",
      "derivative": "any",
      "version": "No"
    }
  ],
  "deploy_toolkit": null,
  "builds": [],
  "cache_files": [
    "Cargo.lock",
    "target"
  ],
  "pipelines": [
    {
      "title": "build-and-compress",
      "desc": "Got from `Rust Enhanced Pipeline`. Build the Rust project with Cargo.",
      "info": "rust-default@0.1.0",
      "tags": [],
      "actions": [
        {
          "title": "Build the project.",
          "desc": "Got from `Cargo Build (Release)`. Build the Rust project with Cargo default settings in release mode",
          "info": "cargo-rel@0.1",
          "tags": [
            "rust",
            "cargo"
          ],
          "action": {
            "Build": {
              "supported_langs": [
                "Rust"
              ],
              "commands": [
                {
                  "bash_c": "cargo build --quiet --release",
                  "ignore_fails": false,
                  "af_placeholder": null,
                  "replace_af_with": []
                }
              ]
            }
          }
        },
        {
          "title": "Compress the resulting binary.",
          "desc": "Got from `UPX Compress`. Compress the binary file with UPX.",
          "info": "upx-compress@0.1.0",
          "tags": [],
          "action": {
            "PostBuild": {
              "supported_langs": [
                "Rust",
                "Go",
                "C",
                "Cpp",
                {
                  "Other": "any"
                }
              ],
              "commands": [
                {
                  "bash_c": "upx <artifact>",
                  "placeholders": [
                    "<artifact>"
                  ],
                  "replacements": [
                    [
                      [
                        "<artifact>",
                        {
                          "title": "target/release/my-rust-project",
                          "is_secret": false,
                          "value": {
                            "Plain": "target/release/my-rust-project"
                          }
                        }
                      ]
                    ]
                  ],
                  "ignore_fails": false,
                  "show_success_output": false,
                  "show_bash_c": false
                }
              ]
            }
          }
        }
      ]
    }
  ],
  "artifacts": [
    "target/release/my-rust-project"
  ],
  "inplace_artifacts_into_project_root": [
    [
      "target/release/my-rust-project",
      "result"
    ]
  ]
}
```

Having only `deploy-config.json` inside your project's root, you can share your build/deploy configurations.

At the end, let's build the project!

```bash
deployer build

# see the build options: you can share cache files and folders by symlinking or copying
deployer build --help
deployer build -fc

# or explicitly specify the project pipeline's short name - `build-and-compress`
deployer build build-and-compress
```

For other options, check:

```bash
deployer build -h
```
