# Ag-RemoteDesk

Alternativa Open Source ao AnyDesk baseada em **WebRTC + Cliente Web** com sinalização em **PHP-FPM** (HTTP/SSE, sem daemons extras).

## Arquitetura

- **Servidor Web & Sinalização (`server-php/`):**
  - Interface web em HTML/JS com `RTCPeerConnection` e `RTCDataChannel`.
  - Autenticação por sessão e banco SQLite local (`server-php/src/Database.php`).
  - Sinalização via REST + Server-Sent Events (`server-php/public/api.php`).
  - Geração de credenciais temporárias para STUN/TURN (`coturn` com `use-auth-secret`).
- **Agente Host (em desenvolvimento):**
  - Alvos iniciais: **Debian 13** e **macOS 27 Golden Gate**.

## Como executar localmente

```bash
php -S 0.0.0.0:8080 -t server-php/public
```

Acesse `http://localhost:8080` e faça login com o usuário padrão criado na primeira execução:
- **Usuário:** `admin`
- **Senha:** `admin123`

> **Atenção:** altere a senha padrão, `app_secret` e configure `turn_urls` / `turn_secret` em `server-php/config.php` antes de usar em produção.

## Changelog

Veja [CHANGELOG.md](CHANGELOG.md).
