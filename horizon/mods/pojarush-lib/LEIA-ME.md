# Pojarush Lib 1.0.0

Biblioteca para mods WebAssembly do Pojarush (motor Horizon). Recria, do nosso jeito, as ideias das bibliotecas de mods Java mais usadas, sobre as funções que o jogo oferece aos mods.

| Módulo | Para que serve | Ideia equivalente no Java |
|---|---|---|
| `config` | Arquivo `.txt` único por mod, com valores booleanos, inteiros, decimais e texto, padrões, limites e restaurar padrões | Cloth Config, Forge Config API Port, YAML API, Fast Config API, Vanilla Config |
| `settings` | Monta a página de configurações do mod (botões e sliders) a partir do `config` e trata os cliques | Cloth Config (tela), owo-lib (config UI) |
| `i18n` | Tabela de traduções escolhida pelo idioma do jogo, com fallback | Fabric API (lang), Kaleido |
| `events` | Eventos de teclas, roda do mouse e uso de itens, já com o estado das teclas | Fabric API (events) |
| `keys` | Registro de teclas do mod e estado pressionado | Fabric API (key bindings) |
| `scheduler` | Tarefas atrasadas ou repetidas por tick | Silk, u_team_core |
| `hud` | Painel de texto na tela, arrastável pelo toque e opcionalmente sempre visível | owo-lib, Fabric API (HUD render callback) |
| `text` | Cores do Minecraft, placeholders `%nome%`, quebra de linha e mistura de cores | Placeholder API |

Uso no `Cargo.toml` do mod:

```toml
[dependencies]
horizon-mod-sdk = { path = "../../crates/horizon-mod-sdk" }
pojarush-lib = { path = "../pojarush-lib" }
```

Requer o nível de API 9 (Pojarush 1.11.14) para `hud`.
