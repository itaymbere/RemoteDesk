# Ag-RemoteDesk

Alternativa Open Source ao AnyDesk baseada em **WebRTC + Cliente Web** com sinalização em **PHP-FPM** (HTTP/SSE, sem daemons extras) e **Agente Host em Rust (`webrtc-rs`)**.

## Arquitetura

- **Servidor Web & Sinalização (`server-php/`):**
  - Interface web em HTML/JS com `RTCPeerConnection` e `RTCDataChannel`.
  - Autenticação por sessão e banco SQLite local (`server-php/src/Database.php`).
  - Sinalização via REST + Server-Sent Events (`server-php/public/api.php`).
  - Geração de credenciais temporárias para STUN/TURN (`coturn` com `use-auth-secret`).
- **Agente Host (`agent/`):**
  - Codebase único em Rust para **Debian 13**, **macOS 27 Golden Gate** e **Windows 10** (planejado).

## Requisitos por Sistema Operacional

### Servidor (`server-php/`)
- PHP 8.1+ com extensões `pdo_sqlite` e `sqlite3`
- Nginx ou Apache com PHP-FPM (veja [server-php/README.md](server-php/README.md))
- HTTPS em produção (obrigatório para WebRTC)
- `coturn` (opcional, para NAT traversal via TURN)

### Agente — Debian 13 (X11)
- Rust / Cargo (para compilar)
- `ffmpeg` e `xdotool`:
  ```bash
  sudo apt install -y ffmpeg xdotool
  ```

### Agente — macOS 27 Golden Gate
- Rust / Cargo (para compilar)
- `ffmpeg` e `cliclick` via Homebrew:
  ```bash
  brew install ffmpeg cliclick
  ```
- Permissões em **Ajustes do Sistema > Privacidade e Segurança**: **Gravação de Tela** e **Acessibilidade** para o terminal/binário do agente.

### Agente — Windows 10 *(planejado)*
- Captura via DXGI Desktop Duplication e entrada via `SendInput` (Win32 API).

## Como executar localmente

1. Suba o servidor PHP:
   ```bash
   php -S 0.0.0.0:8080 -t server-php/public
   ```
2. Rode o agente na máquina alvo:
   ```bash
   cd agent
   SERVER_URL=http://localhost:8080/api.php cargo run
   ```
3. Acesse `http://localhost:8080` e entre com o usuário padrão (`admin` / `admin123`).

> **Atenção:** altere a senha padrão, `app_secret` e configure `turn_urls` / `turn_secret` em `server-php/config.php` antes de usar em produção.

## Changelog

Veja [CHANGELOG.md](CHANGELOG.md).
