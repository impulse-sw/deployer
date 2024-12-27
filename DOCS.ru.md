# Деплойер: документация по версии `0.2.X`

## Описание принципов работы

Деплойер, по своей сути, - локальный CI/CD. Иными словами, менеджер `bash`-команд.

Как правило, запускает сборку Деплойер в отдельной папке, чтобы сохранять кэш и при этом держать папку с кодом чистой. Однако вы можете указать как любую папку, так и папку с кодом; если у вас уже есть кэши, вы можете их копировать из исходной папки, делать на них симлинки или полностью их игнорировать и собирать с нуля.

## Описание основных сущностей

### 1. Действие - `Action`

Действие - это основная сущность Деплойера. На Действиях в составе Пайплайнов строятся процессы сборки, установки и развёртывания. Однако само по себе Действие быть назначенным проекту не может, для этого и нужны Пайплайны (см. ниже).

В составе Пайплайнов или в Реестре Действий Деплойера действие выглядит как конструкция:

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
          "show_bash_c": false,
          "only_when_fresh": false
        }
      ]
    }
  }
}
```

В составе Реестров каждое Действие и каждый Пайплайн являются значениями в словаре с ключом `info` (например, `"upx-compress@0.1.0": { ... }`). Таким образом их можно быстро редактировать, выводить на экран содержимое, добавлять в Пайплайны и проекты.

Существует 3 категории основных Действий и 5 дополнительных видов Действий:

1. Действия сборки (`PreBuild`, `Build`, `PostBuild` и `Test`)
2. Действия установки (`Pack`, `Deliver`, `Install`)
3. Действия развёртывания (`ConfigureDeploy`, `Deploy`, `PostDeploy`)
4. Действие наблюдения `Observe`
5. Действие прерывания `Interrupt`
6. Действие принудительной синхронизации готовых артефактов `ForceArtifactsEnplace`
7. Действие с кастомной командой `Custom`
8. Действие проверки вывода кастомной команды `Check`

Основополагающим является концепт кастомной команды - команды для оболочки терминала. Действия `Custom`, `Observe` и три основные категории Действий содержат внутри одну или больше кастомных команд.

#### 1.1. Кастомная команда

Описание команды для Деплойера выглядит следующим образом:

```json
{
  "bash_c": "upx <artifact>",
  "placeholders": [
    "<artifact>"
  ],
  "ignore_fails": false,
  "show_success_output": false,
  "show_bash_c": false,
  "only_when_fresh": false
}
```

- `bash_c` содержит текст команды, которая будет выполняться в терминале
- `placeholders` содержит список плейсхолдеров, которые можно будет заменять на переменные и артефакты проекта, чтобы выполнять с ними необходимые действия
- `ignore_fails` говорит Деплойеру, нужно ли квалифицировать статус выхода процесса, не равный нулю, как нормальное поведение команды, или нет; если нет, то Деплойер прервёт выполнение Пайплайна и выйдет со статусом `1`
- `show_success_output` говорит Деплойеру, нужно ли печатать вывод команды всегда (в т.ч. когда статус выхода процесса - `0`), или же нужно печатать только при ошибке
- `show_bash_c` говорит Деплойеру, нужно ли печатать на экране полный текст команды; это может быть полезным, когда команда содержит уязвимые переменные
- `only_when_fresh` говорит Деплойеру, что это действие нужно выполнять только при свежей сборке (либо при первой сборке, либо при явном указании пересобрать с нуля при помощи опции `-f`)

Когда команда специализируется для конкретного проекта, она обрастает дополнительным свойством - `replacements`:

```json
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
          "title": "target/release/deployer",
          "is_secret": false,
          "value": {
            "Plain": "target/release/deployer"
          }
        }
      ]
    ]
  ],
  "ignore_fails": false,
  "show_success_output": false,
  "show_bash_c": false,
  "only_when_fresh": false
}
```

`replacements` содержит список замен плейсхолдеров в команде на указанные артефакты или переменные. Следует заметить, что одна и та же команда может выполняться несколько раз для разных наборов переменных, даже если указана в Действии один раз:

```json
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
          "title": "target/release/deployer",
          "is_secret": false,
          "value": {
            "Plain": "target/release/deployer"
          }
        }
      ]
    ],
    [
      [
        "<artifact>",
        {
          "title": "target/release/another",
          "is_secret": false,
          "value": {
            "Plain": "target/release/another"
          }
        }
      ]
    ]
  ],
  "ignore_fails": false,
  "show_success_output": false,
  "show_bash_c": false,
  "only_when_fresh": false
}
```

В указанном примере используется только один плейсхолдер `<artifact>`, но их может быть несколько, в т.ч. - различные опции для выполнения команды.

Соответственно, если вы хотите просто выполнять команды, которые нельзя отнести к одному из трёх основных видов Действий, следует использовать Действие типа `Custom`:

```json
{
  "title": "List all files and folders",
  "desc": "",
  "info": "ls@0.1.0",
  "tags": [],
  "action": {
    "Custom": {
      "bash_c": "ls",
      "ignore_fails": false,
      "show_success_output": true,
      "show_bash_c": true,
      "only_when_fresh": false
    }
  }
}
```

#### 1.2. Действия сборки - `PreBuild`, `Build`, `PostBuild` и `Test`

Для Действий сборки является специфичной специализация на языках программирования: в зависимости от того, соответствует ли набор языков, используемых в проекте, тому набору, который указан в действиях по сборке, Деплойер будет предупреждать вас об использовании несовместимых с проектом Действий.

В вышеуказанном примере мы видим действие, которое должно выполняться после сборки:

```json
{
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
        "show_bash_c": false,
        "only_when_fresh": false
      }
    ]
  }
}
```

#### 1.3. Действия установки - `Pack`, `Deliver` и `Install`

Для этой группы Действий ключевым фактором специализации является целевой объект установки - *таргет*. Если характеристики таргета проекта - аппаратная или программная платформа - не соответствуют характеристикам Действия установки, будет выдано предупреждение.

С удовольствием заметим, что UPX скорее относится к Действию упаковки, нежели к Действию после сборки:

```json
{
  "title": "UPX Pack",
  "desc": "Pack the binary by UPX.",
  "info": "upx-pack@0.1.0",
  "tags": [
    "upx"
  ],
  "action": {
    "Pack": {
      "target": {
        "arch": "x86_64",
        "os": "Linux",
        "derivative": "any",
        "version": "No"
      },
      "commands": [
        {
          "bash_c": "upx <af>",
          "placeholders": [
            "<af>"
          ],
          "ignore_fails": false,
          "show_success_output": false,
          "show_bash_c": false,
          "only_when_fresh": false
        }
      ]
    }
  }
}
```

- `arch` - это строковое обозначение архитектуры аппаратного обеспечения таргета
- `os` - это один из вариантов (`android`|`ios`|`linux`|`unix-{unix-name}`|`windows`|`macos`) или любое другое строковое обозначение операционнной системы
- `derivative` - это дополнительное описание операционной системы или программной платформы
- `version` - это версия операционной системы или программной платформы

Если `derivative` отсутствует, рекомендуется писать `any`.

#### 1.4. Действия развёртывания - `ConfigureDeploy`, `Deploy`, `PostDeploy`

Для этой группы Действий ключевым фактором специализации является тулкит для развёртывания - Docker, Docker Compose, Podman, k8s или иной инструментарий контейнеризации или виртуализации. Если в проекте будет указан не тот тулкит, Деплойер выдаст предупреждение.

Приведём пример с Docker Compose:

```json
{
  "title": "Build Docker Compose Image",
  "desc": "Build Docker image with Docker Compose",
  "info": "docker-compose-build@0.1.0",
  "tags": [
    "docker",
    "compose"
  ],
  "action": {
    "ConfigureDeploy": {
      "deploy_toolkit": "docker-compose",
      "tags": [
        "docker",
        "compose"
      ],
      "commands": [
        {
          "bash_c": "docker compose build",
          "ignore_fails": false,
          "show_success_output": false,
          "show_bash_c": true,
          "only_when_fresh": false
        }
      ]
    }
  }
}
```

#### 1.5. Другие действия - `Interrupt`, `ForceArtifactsEnplace`, `Observe` и `Check`

> NOTE: Нет нужного примера конфигурации? Создайте действие самостоятельно при помощи команды `deployer new action` и выведите его на экран при помощи `deployer cat action my-action@x.y.z`.

`Interrupt` используется для ручного прерывания сборки/развёртывания проекта. Когда Деплойер доходит до этого действия, он ожидает пользовательского ввода, чтобы продолжить, когда вы выполните необходимые действия вручную.

`ForceArtifactsEnplace` используется для того, чтобы принудительно синхронизировать артефакты, даже когда не все артефакты сгенерированы. По умолчанию указанные в конфигурации проекта артефакты переносятся в папку `artifacts`, но с помощью такого действия это можно выполнить чуть раньше, например, когда происходит рекурсивная сборка проекта с помощью Деплойера:

```json
{
  "title": "Force enplace",
  "desc": "",
  "info": "force-enplace@0.1.0",
  "tags": [],
  "action": "ForceArtifactsEnplace"
}
```

`Observe` - Действие, которое практически идентично `Custom`. Оно используется, например, чтобы запустить Prometheus, Jaeger или что угодно ещё.

А вот `Check` - особенное действие, позволяющее проверять, что вывела команда в `stdout`/`stderr`:

```json
{
  "Check": {
    "command": {
      "bash_c": "<af>",
      "placeholders": [
        "<af>"
      ],
      "ignore_fails": true,
      "show_success_output": false,
      "show_bash_c": false,
      "only_when_fresh": false
    },
    "success_when_found": "some rust regex",
    "success_when_not_found": null
  }
}
```

- `success_when_found` сообщает Деплойеру, что если он найдёт указанное регулярное выражение, то выполнение команды будет считаться успешным
- `success_when_not_found` сообщает Деплойеру, что если он не найдёт указанное регулярное выражение, то выполнение команды будет считаться успешным

Причём, если оба поля указаны, то успешным запуск будет считаться в случае, если оба варианта были успешны (первое регулярное выражение должен найти, второе - должен не найти).

На этом описание Действий заканчивается, и мы переходим к Пайплайнам.

### 2. Пайплайн - `Pipeline`

Пайплайн - это упорядоченный набор Действий, который необходим для достижения определённой цели. Например, когда нужно проверить качество кода, проверить код с помощью статического анализатора, затем собрать, сжать, упаковать в пакет для определённого дистрибутива и загрузить на хостинг. Или когда нужно собрать Android-приложение, подписать и установить на устройство, подключённое по ADB. Композиция Пайплайна может быть любой, главный же пример приведён в файле `deploy-config.json` этого репозитория:

```json
{
  "title": "Deployer Pipeline",
  "desc": "Default Deployer Pipeline for itself.",
  "info": "deployer-default@0.1.0",
  "tags": [
    "cargo",
    "clippy",
    "build",
    "upx"
  ],
  "actions": [
    {
      "title": "Lint",
      "desc": "Got from `Cargo Clippy`.",
      "info": "cargo-clippy@0.1.0",
      "tags": [
        "cargo",
        "clippy"
      ],
      "action": {
        "PreBuild": {
          "supported_langs": [
            "Rust"
          ],
          "commands": [
            {
              "bash_c": "cargo clippy",
              "ignore_fails": false,
              "show_success_output": true,
              "show_bash_c": true,
              "only_when_fresh": null
            }
          ]
        }
      }
    },
    {
      "title": "Build",
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
              "show_success_output": false,
              "show_bash_c": true,
              "only_when_fresh": null
            }
          ]
        }
      }
    },
    {
      "title": "Compress",
      "desc": "Got from `UPX Compress`.",
      "info": "upx@0.1.0",
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
              "replacements": [
                [
                  [
                    "<artifact>",
                    {
                      "title": "target/release/deployer",
                      "is_secret": false,
                      "value": {
                        "Plain": "target/release/deployer"
                      }
                    }
                  ]
                ]
              ],
              "ignore_fails": false,
              "show_success_output": false,
              "show_bash_c": false,
              "only_when_fresh": null
            }
          ]
        }
      }
    },
    {
      "title": "Install to ~/.cargo/bin",
      "desc": "",
      "info": "install-to-cargo-bin@0.1.1",
      "tags": [
        "cargo"
      ],
      "action": {
        "Install": {
          "target": {
            "arch": "x86_64",
            "os": "Linux",
            "derivative": "any",
            "version": "No"
          },
          "commands": [
            {
              "bash_c": "cp -f <artifact> ~/.cargo/bin",
              "placeholders": [
                "<artifact>"
              ],
              "replacements": [
                [
                  [
                    "<artifact>",
                    {
                      "title": "target/release/deployer",
                      "is_secret": false,
                      "value": {
                        "Plain": "target/release/deployer"
                      }
                    }
                  ]
                ]
              ],
              "ignore_fails": false,
              "show_success_output": false,
              "show_bash_c": false,
              "only_when_fresh": null
            }
          ]
        }
      }
    }
  ],
  "default": true
}
```

В общем, Пайплайн просто содержит список Действий в поле `actions`. Остальное - такое же, как и у Действия.

## Описание утилиты CLI

Деплойер, в первую очередь, - CLI-утилита. По любой команде Деплойера можно посмотреть справку, указав опцию `-h`. Приведём примеры самых распространённых команд:

```bash
deployer new action                            # создать Действие и поместить в Реестр
deployer new pipeline                          # создать Пайплайн и поместить в Реестр
deployer init                                  # инициализировать проект, указать все свойства
deployer with                                  # проверить совместимость и назначить Пайплайн для проекта,
                                               # а также указать необходимые переменные и артефакты вместо плейсхолдеров
deployer build                                 # запустить Пайплайн, назначенный по умолчанию
deployer build my-pipe                         # запустить Пайплайн по короткому имени
deployer build configure,build -o build-folder # запустить Пайплайны `configure` и `build` в папке `build-folder`
```

### Интерфейс консоли (TUI)

Деплойер обладает поддержкой высококлассного настройщика через терминал, что позволяет вам вообще забыть про ручное написание Действий и Пайплайнов для ваших проектов. Просто попробуйте создать Действие или Пайплайн, и Деплойер сам вас обо всём спросит.
