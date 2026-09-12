# Notas del fork

Fork de `BloopAI/vibe-kanban` para uso propio. Rama de trabajo: `iago`.
Upstream quedó abandonado el 2026-04-24 — no habrá sincronizaciones.

## Auditoría de llamadas a casa (paso 2)

**Conclusión: no hay nada que cortar en el código fuente.** Todas las llamadas externas
están detrás de variables de entorno leídas con `option_env!` (incrustadas en tiempo de
compilación) con respaldo en `std::env::var`. El binario que distribuyen por `npx` las
lleva dentro porque lo compila BloopAI con sus claves; **un build desde fuente sin esas
variables las deja muertas por construcción.**

| Qué | Puerta | Dónde |
|---|---|---|
| PostHog (analytics) | `POSTHOG_API_KEY` + `POSTHOG_API_ENDPOINT` | `crates/services/src/services/analytics.rs` |
| `api.vibekanban.com` | `VK_SHARED_API_BASE` | `crates/local-deployment/src/lib.rs:171` |
| Relay compartido | `VK_SHARED_RELAY_API_BASE` | `crates/local-deployment/src/lib.rs:174` |

`AnalyticsConfig::new()` devuelve `None` si falta cualquiera de las dos claves de PostHog.
`remote_info.get_api_base()` devuelve `None` sin `VK_SHARED_API_BASE`, y entonces el
cliente remoto queda en `RemoteClientNotConfigured`.

### Lo que NO es telemetría

- **`PrMonitorService`** (`crates/services/src/services/pr_monitor.rs`) sondea cada 60 s,
  pero consulta **GitHub** por el estado de tus propios PRs. Es funcionalidad legítima.
  Acepta `Option<RemoteClient>` y `Option<AnalyticsContext>`, ambos `None` en un build
  propio.
- **`app.loops.so`** (email transaccional) vive en `crates/remote/src/mail.rs`. El crate
  `remote` es **su backend alojado**, no se ejecuta en el despliegue local.

### Regla para el futuro

**Nunca definir `POSTHOG_API_KEY`, `POSTHOG_API_ENDPOINT`, `VK_SHARED_API_BASE` ni
`VK_SHARED_RELAY_API_BASE`** al compilar o ejecutar. Con Tailscale de por medio, el relay
compartido no pinta nada en esta arquitectura.

## Agente Antigravity (paso 3)

`agy` no habla ACP de forma nativa — el feature request oficial
[antigravity-cli#31](https://github.com/google-antigravity/antigravity-cli/issues/31)
sigue abierto. El ejecutor lo lanza a través de
[`agy-acp`](https://github.com/shindgew/agy-acp) (Apache-2.0), un adaptador que envuelve el
binario `agy` instalado y expone ACP por stdio, reutilizando el harness ACP compartido
igual que Gemini, Qwen y Copilot.

Esto esquiva de paso el problema conocido de `agy -p`, que escribe la respuesta al terminal
controlador en vez de a stdout: el adaptador no usa el modo print.

### Ficheros tocados

| Fichero | Cambio |
|---|---|
| `crates/executors/src/executors/antigravity.rs` | **nuevo** — el ejecutor |
| `crates/executors/src/executors/mod.rs` | `pub mod`, import, variante del enum, `capabilities()` |
| `crates/executors/src/mcp_config.rs` | adaptador MCP `Passthrough` |
| `crates/server/src/bin/generate_types.rs` | declaración TS + JSON schema |
| `crates/executors/default_profiles.json` | perfil `ANTIGRAVITY` por defecto |

### Decisiones

- **Adaptador MCP `Passthrough`, no `Gemini`.** `adapt_gemini` reescribe `url` → `httpUrl`,
  pero agy espera `serverUrl` para transporte HTTP. `Passthrough` escribe `mcpServers` tal
  cual, formato que agy sí entiende (verificado en `~/.gemini/config/mcp_config.json`).
- **Sin flag `yolo`.** Las aprobaciones van por ACP a través del harness, que es el
  mecanismo correcto y más fiable que saltárselas con
  `--dangerously-skip-permissions`.
- **Modelos** extraídos de `agy models` con el CLI 1.2.2. Por defecto
  `gemini-3.1-pro-high`.

### Sin verificar todavía

1. **No compilado.** Falta `cargo check` del workspace.
2. **No probado de extremo a extremo.** No se ha lanzado `npx agy-acp` contra el harness.
3. **Paso de `--model`.** El README de `agy-acp` mapea su opción `model` al flag `--model`
   de agy; no está confirmado que lo acepte como argumento de línea de comandos en vez de
   como parámetro de sesión ACP. Si falla, el ajuste es en `build_command_builder()`.
