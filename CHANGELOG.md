# Changelog

Todas as alterações notáveis deste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).

## [v0.0.1] - 2026-11-27

### Adicionado
- Estrutura inicial do servidor de sinalização e cliente web em PHP-FPM (`server-php/`), banco SQLite local (`server-php/src/Database.php`), endpoints REST/SSE (`server-php/public/api.php`) e configuração STUN/TURN (`server-php/config.php`) (`1bf2c83`).
- Interface web com login, listagem de hosts online e visualizador WebRTC (`server-php/public/index.php`) com suporte a mouse, teclado (`1bf2c83`, `218583f`) e toque para Android/iOS (`e23c16d`).
- Guia de instalação para Nginx/Apache + PHP-FPM e `coturn` (`server-php/README.md`) (`c9e2c33`).
- Agente Host multiplataforma em Rust (`agent/` com `webrtc-rs`) (`c9e2c33`, `4e80675`):
  - **Debian 13:** captura X11 (`x11grab`) / Wayland (`pipewire`) e injeção de entrada (`xdotool` / `ydotool`) (`218583f`, `65fc475`, `3c1d54a`).
  - **macOS 27 Golden Gate:** captura (`avfoundation`) com detecção automática de dispositivo (`2dd8851`) e entrada (`cliclick` / `osascript`) (`7dd6664`).
  - **Windows 10 / 11:** captura (`gdigrab`) e entrada (`PowerShell` / `user32.dll`) (`f26a502`), com guia de instalação via `winget` no `README.md` (`d2a252c`).
  - Arquivos de serviço para `systemd` (Debian) e `launchd` (macOS) em `agent/deploy/` (`007487c`).
- App **Android Host** em Kotlin (`android-agent/`) com `MediaProjection`, `AccessibilityService`, `webrtc-android` (`f7e709a`) e sinalização SSE (`8705a2f`).
- App **iOS Screen-Sharing Host** em Swift (`ios-agent/`) com `ReplayKit` (`RPBroadcastSampleHandler`), `WebRTC` (`322d436`) e sinalização SSE (`8705a2f`).
- Telas de **Configuração do Servidor** (STUN/TURN) e **Configuração do Cliente Remoto** (FPS, Bitrate, Nome do Host com reinício automático via `DataChannel`) (`2079b57`).
- Presets de qualidade de imagem, botão **Tela Cheia**, container ampliado (`96vw`) e arquivo de log do servidor (`server-php/data/server.log`) com fuso horário `America/Sao_Paulo` (`d2a252c`).
- Renomeação da identidade do projeto para **RemoteDeskIT** (`d43f554`).
- Leitura dinâmica de `fps` e `bitrate` a partir de `agent_config.json` no agente Linux e sincronização automática das configurações atuais (`config_current`) para o navegador ao conectar (`e6399a1`).
- Logs rotativos em arquivo no agente (`agent/src/logging.rs`) e logs de erro de `ffmpeg`/input no macOS (`13510b0`).
- Sincronização automática de workspace (`.vscode/tasks.json`, `scripts/auto-sync.sh`) e trava `pre-commit` (`scripts/hooks/pre-commit`) para impedir commits desatualizados.

### Corrigido
- Suporte a candidatos ICE em loopback (`localhost`), latência de 15 FPS e `muted` no `<video>` para autoplay no Safari (`eaee42d`).
- Evitado erro `500` por lock de escrita no SQLite e adicionado retry/cleanup na sinalização (`aec6b71`).
- Corrigido lock de sessão no SSE, replay de ICE antigo e limpeza de peers no agente (`1808868`).
- Reinício gracioso do agente ao recarregar configuração e ajuste de FPS/qualidade VP8 no macOS (`ac74e9c`).
- *Throttle* de 30 ms no `mousemove` em `server-php/public/index.php` para reduzir atraso e fila de processos de entrada (`d2a252c`).
