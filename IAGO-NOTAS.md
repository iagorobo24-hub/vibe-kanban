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

### 🟡 Telemetría en tiempo de COMPILACIÓN (descubierta al construir el frontend)

`pnpm run build` en `packages/local-web` ejecuta **`sentry-vite-plugin`**, que intenta subir
source maps a Sentry. Sin token falla y el build continúa:

```
error: Auth token is required for this request. Please run `sentry-cli login`
```

No se sube nada sin `SENTRY_AUTH_TOKEN`, igual que el resto: apagado por defecto en un
build propio. Pero conviene saber que **existe una llamada saliente en el build del
frontend**, no solo en runtime. Si molesta, se quita del `vite.config.ts`.

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
| `packages/web-core/src/shared/components/AgentIcon.tsx` | nombre e icono en la UI |
| `packages/public/agents/antigravity-{light,dark}.svg` | **nuevos** — icono propio |

> El fichero de la UI se me pasó en la primera pasada: busqué puntos de registro en
> `frontend/src`, que en este repo no existe — el código vive en `packages/`. Lo cazó el
> compilador de TypeScript, porque `getAgentName` tiene un `switch` exhaustivo sin
> `default` y añadir un agente al enum lo rompe.
>
> Los iconos son un **marcador de posición neutro** (flecha ascendente en un círculo), no
> una reproducción de la marca de Google.

### Decisiones

- **Adaptador MCP `Passthrough`, no `Gemini`.** `adapt_gemini` reescribe `url` → `httpUrl`,
  pero agy espera `serverUrl` para transporte HTTP. `Passthrough` escribe `mcpServers` tal
  cual, formato que agy sí entiende (verificado en `~/.gemini/config/mcp_config.json`).
- **Sin flag `yolo`.** Las aprobaciones van por ACP a través del harness, que es el
  mecanismo correcto y más fiable que saltárselas con
  `--dangerously-skip-permissions`.
- **Modelos** extraídos de `agy models` con el CLI 1.2.2. Por defecto
  `gemini-3.1-pro-high`.

### Verificado contra el adaptador real (`agy-acp` 0.5.2, Apache-2.0)

Handshake ACP ejecutado a mano:

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}' \
  | npx -y agy-acp
```

Respuesta (`protocolVersion: 1`), con tres datos que confirman decisiones del ejecutor:

| Lo que responde | Qué valida |
|---|---|
| `sessionCapabilities.fork: {}` | `BaseAgentCapability::SessionFork` es correcto |
| `mcpCapabilities: {http: false, sse: false}` | **solo MCP por stdio** — refuerza `Passthrough`; los servidores MCP HTTP no funcionarán por esta vía, independientemente del adaptador elegido |
| `authMethods: [agy-login]` | el login es un flujo de terminal (`agy --login`), no algo que el ejecutor deba gestionar |

`npx -y agy-acp --model gemini-3.1-pro-high` arranca limpio, exit 0 y sin stderr: **el flag
no rompe**. No está probado que el modelo se *honre* — podría ignorarse en silencio.
Comprobarlo requiere una sesión real con login.

### Estado de compilación — TODO VERDE

```
cargo check -p executors            ->  OK, 0 errores
cargo check --workspace             ->  OK, 0 errores, 2m40s
cargo fmt --all -- --check          ->  limpio
cargo run --bin generate_types      ->  OK, ANTIGRAVITY en shared/types.ts
cargo test -p executors             ->  39 pasan, 3 fallan (preexistentes, ver abajo)
```

Warnings restantes (`utils/src/command_ext.rs:36`, `embedded-ssh/src/sftp.rs:371`) son
preexistentes de upstream.

#### Los 3 tests que fallan ya fallaban antes

`executors::claude::tests::{test_ls_tool_content_extraction, test_path_relative_conversion,
test_amp_tool_aliases_create_file_and_edit_file}`.

Verificado empíricamente: se creó un worktree en el commit base `4deb7eca` (sin ninguno de
estos cambios) y da **exactamente el mismo resultado**, `39 passed; 3 failed`, mismos tests
y mismas líneas. Son fallos de Windows en código de upstream: los tests hardcodean rutas
POSIX (`/tmp/test-worktree`) y `make_path_relative` no puede recortar ese prefijo en
Windows. **Cero regresiones introducidas por este trabajo.**

### 🟡 RESUELTO: build nativo de Windows

`cargo check --workspace` muere en `libsqlite3-sys v0.30.1`:

```
Unable to find libclang: "couldn't find any valid shared libraries matching:
['clang.dll', 'libclang.dll'], set the `LIBCLANG_PATH` environment variable"
```

`libsqlite3-sys` usa **bindgen**, que necesita `libclang.dll`. No está instalado en esta
máquina, y no viene con Visual Studio Build Tools 2026 salvo que se marque el componente
"C++ Clang tools for Windows".

**Esto no es un problema nuestro: upstream no soporta compilación nativa en Windows.**
Su CI (`.github/workflows/pre-release.yml`) **cross-compila desde Ubuntu** con
`cargo xwin --cross-compiler clang-cl`, tras instalar por apt
`clang libclang-dev lld llvm nasm cmake ninja-build`. Los binarios de Windows que
distribuyen por npm salen de ahí, no de un build en Windows.

Afecta a todo lo que dependa del crate `db` — es decir, al servidor entero. El crate
`executors`, que es donde vive el ejecutor de Antigravity, no depende de `db` y compila
bien.

#### Cómo se resolvió

Se descartó el MSI de LLVM (609 MB, pide administrador) y compilar en WSL o Docker — esa
vía produce un binario Linux, y **el orquestador tiene que correr en Windows nativo** para
poder lanzar `claude.exe`, `agy.exe` y `opencode`.

Solución mínima: del release oficial `clang+llvm-23.1.1-x86_64-pc-windows-msvc.tar.zst`
(468 MB, sha256 `c8a12d754b5050c5668b56a5425c806792d46c70f7244b1216046164aa4b6462`) se
extrajo **solo `libclang.dll`** (86 MB) a `../.toolchain/llvm/bin`, fuera del repo. El
archivo comprimido se borró.

`.cargo/config.toml` apunta ahí con ruta **relativa**, así que sobrevive a mover el
proyecto entero:

```toml
[env]
LIBCLANG_PATH = { value = "../.toolchain/llvm/bin", relative = true }
```

Verificado: `cargo check -p db` compila con la variable **sin exportar**, solo con el
config. No hizo falta tocar el PATH ni las variables de entorno del sistema.

El toolset de C de MSVC ya estaba (Build Tools 2026, MSVC 14.50.35717 con `cl.exe`); el
crate `cc` lo localiza solo vía vswhere, aunque no esté en el PATH.

### Sin verificar todavía

1. **No probado de extremo a extremo.** El servidor nunca se ha arrancado con un agente
   Antigravity real ejecutando una tarea. Es lo siguiente que hay que hacer.
2. **Que `--model` se honre**, no solo que se acepte sin error.
3. **Login.** `agy` necesita sesión iniciada; el adaptador expone `agy --login` como método
   de autenticación por terminal.
4. **Frontend.** No se ha verificado que el selector de agentes muestre Antigravity en la
   UI, solo que el tipo existe en `shared/types.ts`.

---

## Backend remoto autoalojado (tableros kanban)

### Por qué hace falta

La app tiene **dos backends**. El local (`server.exe`) ejecuta los agentes y los
workspaces. El remoto (`crates/remote`) lleva cuentas, proyectos y **los tableros
kanban** — normalmente en los servidores de BloopAI. Por eso el tablero pide iniciar
sesión: no es una función apagada, es una función que vive en otra máquina.

Bloop cerró el 10 de abril de 2026. Su API alojada aún responde
(`/v1/health` → `{"status":"ok","version":"0.1.27"}`) pero está **17 versiones por
detrás** de este código, nadie la mantiene y no hay comprobación de compatibilidad en el
cliente. Autoalojar es la única opción sensata.

### Configuración

Del compose solo **una variable es obligatoria**: `VIBEKANBAN_REMOTE_JWT_SECRET`. Todo lo
demás tiene defecto, y **la telemetría viene vacía de fábrica** (`POSTHOG_*`,
`SENTRY_DSN_REMOTE`, `LOOPS_EMAIL_API_KEY`, `STRIPE_*`).

`crates/remote/.env.remote` está escrito con secretos generados al azar y **lo ignora git**
(`.gitignore` línea 19). Usa la vía de autenticación local (`SELF_HOST_LOCAL_AUTH_EMAIL` /
`_PASSWORD`, implementada en `src/auth/local.rs`), así que **no hace falta registrar una
GitHub App ni OAuth de Google**. La contraseña generada está en ese fichero.

El servidor escucha en `127.0.0.1:3000` — solo localhost, nada expuesto.

```bash
pnpm run remote:dev                        # levanta postgres + electric + remote-server
VK_SHARED_API_BASE=http://localhost:3000   # y se arranca el server local con esto
```

### Almacenamiento en disco

**No requiere configuración.** `C:\Users\iagui\AppData\Local\Docker\wsl` es un *junction* a
`E:\Dev\Docker\wsl`, así que el data root de Docker ya resuelve a E. Los volúmenes con
nombre a secas —misma convención que GestionaB2B— aterrizan ahí solos. No hacen falta
`driver_opts` ni bind mounts.

### Estado: FUNCIONANDO

```
remote-db       Up (healthy)   0.0.0.0:5433->5432/tcp
remote-server   Up (healthy)   127.0.0.1:3000->8081/tcp
electric        Up (healthy)
```

`GET localhost:3000/v1/auth/methods` → `{"local_auth_enabled":true,"oauth_providers":[]}`

El log de arranque confirma la auditoría de telemetría, sin tocar nada:

```
LOOPS_EMAIL_API_KEY not set. Email notifications will be disabled.
PostHog analytics not configured
Billing provider not configured
GitHub App not configured
```

El servidor local arranca ahora con `Remote client initialized with URL: http://localhost:3000`
en vez de `VK_SHARED_API_BASE not set`.

### Tres tropiezos del montaje, y cómo se resolvieron

1. **`ssh: [default]` en el build.** El compose pedía agente SSH para acceder a
   `BloopAI/vibe-kanban-private`, de donde sale el paquete `billing`. Es
   `optional = true` y solo entra vía `FEATURES`, que dejamos vacío, así que el requisito
   se comentó en `docker-compose.yml`. **Confirmado: el build compila sin esa dependencia
   privada.** Un `docker-compose.override.yml` no sirve — compose responde
   `cannot override services.remote-server.build.ssh`.
2. **`VIBEKANBAN_REMOTE_JWT_SECRET` debe ser base64 *estándar*** que decodifique a ≥32
   bytes (`validate_jwt_secret` en `src/config.rs:448`). Un `token_urlsafe` de Python
   falla, porque usa `-` y `_`.
3. **`ELECTRIC_ROLE_PASSWORD` va dentro de una URI de Postgres**
   (`postgresql://electric_sync:${...}@remote-db:5432/remote`). Con símbolos como `@`,
   `#`, `+` o `=` Electric muere con *"invalid or missing username"*. Debe ser
   alfanumérica.

### ⚠️ El relay va en un perfil aparte, y sin él no hay conexión al host

`relay-server` está bajo `profiles: ["relay"]`, así que **un `up -d` normal no lo
levanta**. Sin él, el backend muestra los proyectos pero no puede emparejarse con tu
máquina: en ajustes pide conectarse a un host y no lo consigue.

```bash
docker compose --env-file .env.remote --profile relay up -d --build
```

Escucha en `127.0.0.1:8082`, que es donde el frontend lo busca
(`VITE_RELAY_API_BASE_URL`). Comprobar con `curl localhost:8082/health` → 200.

### Cómo arrancar el conjunto

```bash
# 1. backend remoto, CON el perfil relay
cd crates/remote
docker compose --env-file .env.remote --profile relay up -d

# 2. servidor local, apuntando a las dos piezas
cd ../..
PORT=8420 HOST=127.0.0.1 \
  VK_SHARED_API_BASE=http://localhost:3000 \
  VK_SHARED_RELAY_API_BASE=http://localhost:8082 \
  ./target/debug/server.exe
```

### Hay DOS interfaces, y hay que entrar en las dos

| URL | Qué es | Para qué |
|---|---|---|
| `localhost:3000` | Web del backend remoto | Proyectos, organizaciones, tableros kanban |
| `localhost:8420` | App local | Agentes, workspaces, worktrees |

**Iniciar sesión en el 3000 no autentica el 8420.** Son sesiones separadas. Si solo entras
en el 3000, verás los proyectos pero sin ningún host conectado, porque tu máquina —que es
el host— no se ha emparejado. Hay que entrar también desde el 8420: icono de cuenta abajo
a la izquierda → *Iniciar sesión* → *Sign in with email*.

Credenciales en `.env.remote` (`SELF_HOST_LOCAL_AUTH_EMAIL` / `_PASSWORD`). Ojo: ese
fichero está en `.gitignore`, y editores como VS Code o Cursor lo **ocultan** del árbol de
ficheros. Ábrelo por ruta o con `cat crates/remote/.env.remote`.

### Selector de carpetas

El servidor local ya expone `/api/filesystem/directory` y `/api/filesystem/git-repos`, y el
frontend los usa en el botón **"Explorar"**: es un navegador de carpetas propio, dentro de
la web.

El explorador **nativo** de Windows no se puede abrir desde una página web — los
navegadores no permiten que una web obtenga rutas reales del disco. Dos vías si se quiere
nativo: la app de escritorio en `crates/tauri-app`, o añadir al servidor local un endpoint
que lance un diálogo nativo (crate `rfd`), aprovechando que el servidor corre en la propia
máquina. La segunda es pequeña y encaja con el resto de la arquitectura.

### Nota sobre Docker Desktop

Si el daemon no arranca y el log muestra `Error 1920` al renombrar ficheros `.sock`, son
sockets AF_UNIX huérfanos que Windows no deja borrar. No hace falta *"Reset to factory
defaults"* — eso borraría todas las imágenes y volúmenes. Basta con renombrar el
directorio padre (`%LOCALAPPDATA%\Docker\run`, `%LOCALAPPDATA%\docker-secrets-engine`) y
reiniciar. Si el problema reaparece sobre ficheros recién creados, reiniciar Windows lo
resuelve.

---

## Siguiente paso

```bash
cargo run --bin server
```

Abrir el tablero, comprobar que Antigravity aparece en el selector de agentes y lanzarle
una tarea de solo lectura. Para los tableros kanban, además, `pnpm run remote:dev`.
