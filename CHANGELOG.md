# Changelog

Todas as alterações notáveis deste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).

## [v0.0.1] - 2026-11-27

### Adicionado
- Estrutura inicial do servidor de sinalização e cliente web em PHP-FPM (`server-php/`).
- Banco de dados SQLite local com migração automática para usuários, hosts e sinais (`server-php/src/Database.php`).
- Endpoints REST e SSE para autenticação, registro de hosts, credenciais STUN/TURN e troca de sinais WebRTC (`server-php/public/api.php`).
- Interface web com login, listagem de hosts online e visualizador WebRTC (`server-php/public/index.php`).
- Arquivo de configuração para segredos e servidores STUN/TURN (`server-php/config.php`).
