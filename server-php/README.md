# Instalação do Servidor PHP (`server-php`)

Guia rápido para instalar o servidor de sinalização e cliente web em um servidor remoto (Debian/Ubuntu) com **Nginx + PHP-FPM**.

## 1. Requisitos

- PHP 8.1+ com extensões `pdo_sqlite` e `sqlite3`
- Nginx (ou Apache)
- Certificado SSL/TLS (HTTPS é obrigatório para WebRTC em produção)
- `coturn` (opcional, recomendado para NAT traversal via TURN)

Instalação dos pacotes no Debian/Ubuntu:

```bash
sudo apt update
sudo apt install -y nginx php-fpm php-sqlite3
```

## 2. Enviar os arquivos

Copie a pasta `server-php/` para o servidor (ex: `/var/www/ag-remotedesk`):

```bash
sudo mkdir -p /var/www/ag-remotedesk
# copie server-php/* para /var/www/ag-remotedesk/
sudo chown -R www-data:www-data /var/www/ag-remotedesk
```

## 3. Configurar `config.php`

Edite `/var/www/ag-remotedesk/config.php`:
- Defina um `app_secret` forte e aleatório (ou use a variável de ambiente `APP_SECRET`).
- Se usar `coturn`, preencha `turn_urls` e `turn_secret` (mesmo valor de `static-auth-secret` no `coturn`).

## 4. Configurar Nginx

Crie `/etc/nginx/sites-available/ag-remotedesk`:

```nginx
server {
    listen 80;
    server_name seu-dominio.com;

    root /var/www/ag-remotedesk/public;
    index index.php;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    location ~ \.php$ {
        include snippets/fastcgi-php.conf;
        fastcgi_pass unix:/run/php/php-fpm.sock;
        # Importante para SSE (api.php?action=poll)
        fastcgi_buffering off;
        fastcgi_read_timeout 60s;
    }
}
```

Ative o site e configure HTTPS com Certbot:

```bash
sudo ln -s /etc/nginx/sites-available/ag-remotedesk /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
sudo certbot --nginx -d seu-dominio.com
```

## 5. Configurar `coturn` (STUN/TURN)

Para conexões atrás de NAT restrito:

```bash
sudo apt install -y coturn
```

Em `/etc/turnserver.conf`:

```ini
listening-port=3478
use-auth-secret
static-auth-secret=MESMO_TURN_SECRET_DO_CONFIG_PHP
realm=seu-dominio.com
```

Reinicie o serviço:

```bash
sudo systemctl enable --now coturn
```

## 6. Alternativa com Apache

Se preferir usar **Apache** com PHP-FPM:

```bash
sudo apt install -y apache2 libapache2-mod-fcgid php-fpm php-sqlite3
sudo a2enmod proxy_fcgi setenvif
sudo a2enconf php8.2-fpm # ajuste para a versão do PHP instalada
```

Crie `/etc/apache2/sites-available/ag-remotedesk.conf`:

```apache
<VirtualHost *:80>
    ServerName seu-dominio.com
    DocumentRoot /var/www/ag-remotedesk/public

    <Directory /var/www/ag-remotedesk/public>
        AllowOverride All
        Require all granted
    </Directory>

    # Desativa compressão/buffering para o SSE (api.php?action=poll)
    <FilesMatch "api\.php$">
        SetEnv no-gzip 1
    </FilesMatch>
</VirtualHost>
```

Ative o site e configure HTTPS:

```bash
sudo a2ensite ag-remotedesk.conf
sudo apache2ctl configtest && sudo systemctl reload apache2
sudo certbot --apache -d seu-dominio.com
```

## 7. Primeiro acesso

Acesse `https://seu-dominio.com` e entre com:
- **Usuário:** `admin`
- **Senha:** `admin123`

Altere a senha padrão após o primeiro login.
