# Changelog

Todas as alterações notáveis deste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).

## [v0.0.1] - 2026-11-27

### Adicionado
- Estrutura inicial do servidor de sinalização e cliente web em PHP-FPM (`server-php/`).
- Banco de dados SQLite local com migração automática para usuários, hosts e sinais (`server-php/src/Database.php`).
- Endpoints REST e SSE para autenticação, registro de hosts, credenciais STUN/TURN e troca de sinais WebRTC (`server-php/public/api.php`).
- Interface web com login, listagem de hosts online e visualizador WebRTC (`server-php/public/index.php`) com suporte a mouse, teclado e toque (Android/iOS).
- Arquivo de configuração para segredos e servidores STUN/TURN (`server-php/config.php`).
- Guia de instalação para Nginx/Apache + PHP-FPM e `coturn` (`server-php/README.md`).
- Agente Host multiplataforma em Rust (`agent/` com `webrtc-rs`):
  - **Debian 13:** captura X11 (`x11grab`) / Wayland (`pipewire`) e injeção de entrada (`xdotool` / `ydotool`).
  - **macOS 27 Golden Gate:** captura (`avfoundation`) e entrada (`cliclick` / `osascript`).
  - **Windows 10:** captura (`gdigrab`) e entrada (`PowerShell` / `user32.dll`).
  - Arquivos de serviço para `systemd` (Debian) e `launchd` (macOS) em `agent/deploy/`.
- App **Android Host** em Kotlin (`android-agent/`) com `MediaProjection`, `AccessibilityService`, `webrtc-android` e sinalização SSE (`api.php?action=poll`).
- App **iOS Screen-Sharing Host** em Swift (`ios-agent/`) com `ReplayKit` (`RPBroadcastSampleHandler`), `WebRTC` e sinalização SSE.
- Telas de **Configuração do Servidor** (STUN/TURN) e **Configuração do Cliente Remoto** (FPS, Bitrate, Nome do Host com reinício automático via `DataChannel`).
- Leitura dinâmica de `fps` e `bitrate` a partir de `agent_config.json` no agente Linux e sincronização automática das configurações atuais (`config_current`) para o navegador ao conectar.

### Corrigido
- Suporte a candidatos ICE em loopback (`localhost`) no `webrtc-rs`.
- Redução do framerate padrão para 15 FPS no Linux para menor latência.
- Adicionado `muted` no `<video>` para compatibilidade com autoplay no Safari.
