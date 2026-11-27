# RemoteDeskIT

Alternativa Open Source ao AnyDesk baseada em **WebRTC + Cliente Web** com sinalização em **PHP-FPM** (HTTP/SSE, sem daemons extras) e **Agente Host em Rust (`webrtc-rs`)**.

## Arquitetura

- **Servidor Web & Sinalização (`server-php/`):**
  - Interface web em HTML/JS com `RTCPeerConnection` e `RTCDataChannel`.
  - Autenticação por sessão e banco SQLite local (`server-php/src/Database.php`).
  - Sinalização via REST + Server-Sent Events (`server-php/public/api.php`).
  - Geração de credenciais temporárias para STUN/TURN (`coturn` com `use-auth-secret`).
- **Agente Host (`agent/`):**
  - Codebase único em Rust para **Debian 13** (X11/Wayland), **macOS 27 Golden Gate** e **Windows 10**.
  - Arquivos de serviço `systemd` e `launchd` em `agent/deploy/`.
- **Mobile (`android-agent/` e `ios-agent/`):**
  - **Cliente Web (Android/iOS):** suporte nativo a toque (`touchstart`, `touchmove`, `touchend`) direto no navegador.
  - **Android Host (`android-agent/`):** captura via `MediaProjection` e controle via `AccessibilityService` (sem root).
  - **iOS Host (`ios-agent/`):** compartilhamento de tela via `ReplayKit` (`RPBroadcastSampleHandler`) + `WebRTC` (apenas visualização, devido às restrições da Apple).

## Requisitos por Sistema Operacional

### Servidor (`server-php/`)
- PHP 8.1+ com extensões `pdo_sqlite` e `sqlite3`
- Nginx ou Apache com PHP-FPM (veja [server-php/README.md](server-php/README.md))
- HTTPS em produção (obrigatório para WebRTC)
- `coturn` (opcional, para NAT traversal via TURN)

### Agente — Debian 13 (X11 / Wayland)
- Rust / Cargo (para compilar — via `rustup` ou pacotes da distribuição)
- Dependências de compilação, `ffmpeg`, `xdotool` (para X11) e `ydotool` (para Wayland):
  ```bash
  sudo apt install -y build-essential ffmpeg xdotool ydotool
  ```
- Em sessões Wayland, certifique-se de que o daemon `ydotoold` esteja em execução com permissão de acesso a `/dev/uinput`.

### Agente — macOS 27 Golden Gate
- Xcode Command Line Tools, Rust / Cargo (para compilar — via Homebrew ou `rustup`), `ffmpeg` e `cliclick`:
  ```bash
  xcode-select --install
  brew install rust ffmpeg cliclick
  ```
- Permissões em **Ajustes do Sistema > Privacidade e Segurança**: **Gravação de Tela**, **Acessibilidade** e **Automação** (`Finder` e `System Events`, usados via `osascript`) para o terminal ou binário do agente.
- A captura de tela detecta automaticamente o dispositivo `Capture screen 0` via AVFoundation em `agent/src/platform/macos.rs` e pode ser sobrescrita pela variável `AVFOUNDATION_DEVICE` (ex.: `AVFOUNDATION_DEVICE=4:none`). Para listar os dispositivos disponíveis:
  ```bash
  ffmpeg -f avfoundation -list_devices true -i ""
  ```
- Logs persistentes no macOS são gravados em `~/Library/Logs/RemoteDeskIT/agent.log` (rotação automática em `10 MB`). Para acompanhar em tempo real:
  ```bash
  tail -f ~/Library/Logs/RemoteDeskIT/agent.log
  ```

### Agente — Windows 10
- Rust / Cargo (para compilar)
- `ffmpeg` (com suporte a `gdigrab` e `libvpx`) disponível no `PATH`
- PowerShell nativo (usado pelo agente para obter a resolução da tela e injetar eventos via `System.Windows.Forms` / `user32.dll`)

## Variáveis de ambiente do Agente

Configuradas em `agent/src/config.rs` (e captura macOS em `agent/src/platform/macos.rs`):

- `SERVER_URL`: URL do `api.php` do servidor de sinalização (padrão: `http://localhost:8080/api.php`)
- `HOST_ID`: identificador único do host (padrão: `debian-host-01`; no macOS/Windows defina por exemplo `macos-host-01` ou `windows-host-01`)
- `HOST_NAME`: nome amigável exibido na interface web (padrão: mesmo valor de `HOST_ID`)
- `HOST_TOKEN`: token secreto para autenticar o host no servidor (padrão: `changeme-token`; troque em produção)
- `AVFOUNDATION_DEVICE` *(opcional, macOS)*: dispositivo AVFoundation usado pelo `ffmpeg` (ex.: `4:none`); se não definido, o agente detecta `Capture screen 0` automaticamente.
- `LOG_FILE` *(opcional)*: caminho do arquivo de log do agente (padrão no macOS: `~/Library/Logs/RemoteDeskIT/agent.log`; nos demais SOs: `<temp_dir>/remotedesk-agent.log`).
- `RUST_LOG` *(opcional)*: filtro de verbosidade dos logs (padrão: `info`; ex.: `debug` ou `ag_remotedesk_agent=debug,webrtc_ice=warn`).

## Como executar localmente

1. Suba o servidor PHP (com múltiplos workers para o SSE não bloquear a sinalização):
   ```bash
   PHP_CLI_SERVER_WORKERS=8 php -S 0.0.0.0:8080 -t server-php/public
   ```
2. Rode o agente na máquina alvo (ou aponte `SERVER_URL` para seu servidor remoto):
   ```bash
   cd agent
   SERVER_URL=http://localhost:8080/api.php HOST_ID=macos-host-01 HOST_TOKEN=changeme-token cargo run
   ```
3. Acesse `http://localhost:8080` e entre com o usuário padrão (`admin` / `admin123`).

> **Atenção:** altere a senha padrão, `app_secret` e configure `turn_urls` / `turn_secret` em `server-php/config.php` antes de usar em produção.

### Executar binário compilado apontando para servidor remoto

```bash
cd agent
cargo build --release
SERVER_URL="http://<IP_DO_SERVIDOR>:8080/api.php" HOST_ID="macos-host-01" HOST_NAME="macOS" HOST_TOKEN="changeme-token" RUST_LOG="info" ./target/release/ag-remotedesk-agent
```

### Serviço em segundo plano no Debian 13 (`systemd`)

1. Compile o binário em release e instale-o:
   ```bash
   cd agent
   cargo build --release
   sudo cp target/release/ag-remotedesk-agent /usr/local/bin/ag-remotedesk-agent
   ```
2. Para rodar manualmente o binário compilado:
   ```bash
   SERVER_URL=http://localhost:8080/api.php HOST_ID=debian-host-01 HOST_TOKEN=changeme-token /usr/local/bin/ag-remotedesk-agent
   ```
3. Ou instale como serviço de usuário (`systemd`):
   ```bash
   mkdir -p ~/.config/systemd/user
   cp agent/deploy/ag-remotedesk-agent.service ~/.config/systemd/user/
   # Edite SERVER_URL, HOST_ID e HOST_TOKEN no arquivo .service
   systemctl --user enable --now ag-remotedesk-agent
   ```

### Serviço em segundo plano no macOS (`launchd`)

1. Compile o binário em release e instale-o:
   ```bash
   cd agent
   cargo build --release
   sudo cp target/release/ag-remotedesk-agent /usr/local/bin/ag-remotedesk-agent
   ```
2. Copie [agent/deploy/com.agremotedesk.agent.plist](agent/deploy/com.agremotedesk.agent.plist) para `~/Library/LaunchAgents/com.agremotedesk.agent.plist`, ajuste `SERVER_URL`, `HOST_ID` e `HOST_TOKEN` e carregue o serviço:
   ```bash
   cp agent/deploy/com.agremotedesk.agent.plist ~/Library/LaunchAgents/com.agremotedesk.agent.plist
   # Edite SERVER_URL, HOST_ID e HOST_TOKEN no arquivo plist antes de carregar
   launchctl load -w ~/Library/LaunchAgents/com.agremotedesk.agent.plist
   ```

### Execução no Windows 10 / 11 (PowerShell)

1. Instale os pré-requisitos:
   ```powershell
   winget install Git.Git Rustlang.Rustup Gyan.FFmpeg
   winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```
   *(Feche e reabra o PowerShell após instalar para atualizar o `PATH`.)*
2. Clone o repositório e compile o binário em release (ou use `build-windows.cmd` na raiz):
   ```powershell
   git clone -b v0.0.1 https://github.com/itaymbere/RemoteDesk.git
   cd RemoteDesk\agent
   cargo build --release
   ```
3. Execute pelo atalho na raiz (`start-windows.cmd`) ou manualmente via PowerShell:
   ```powershell
   $env:SERVER_URL="http://IP_DO_SERVIDOR:8080/api.php"
   $env:HOST_ID="windows-host-01"
   $env:HOST_TOKEN="changeme-token"
   .\target\release\ag-remotedesk-agent.exe
   ```

### Sincronização automática entre ambientes (Dev)

- Ao abrir o workspace, `.vscode/tasks.json` executa `git pull --ff-only` automaticamente.
- Para bloquear commits quando houver alterações remotas pendentes, ative o hook `pre-commit` em cada máquina (no Windows, pode executar `setup-hooks-windows.cmd` na raiz):
  ```bash
  git config core.hooksPath scripts/hooks
  ```
- Para sincronização contínua em segundo plano (no Windows, execute via Git Bash):
  ```bash
  ./scripts/auto-sync.sh
  ```

## Changelog

Veja [CHANGELOG.md](CHANGELOG.md).
