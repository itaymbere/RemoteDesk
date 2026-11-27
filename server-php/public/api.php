<?php
declare(strict_types=1);

session_start();
header('Content-Type: application/json; charset=utf-8');

$config = require __DIR__ . '/../config.php';
$overrideFile = __DIR__ . '/../data/server_config.json';
if (is_file($overrideFile)) {
    $overrides = json_decode(file_get_contents($overrideFile) ?: '{}', true);
    if (is_array($overrides)) {
        $config = array_merge($config, $overrides);
    }
}
require __DIR__ . '/../src/Database.php';

$db = new Database($config['db_path']);
$pdo = $db->pdo();

$action = $_GET['action'] ?? '';
$input = json_decode(file_get_contents('php://input') ?: '[]', true) ?: [];

function jsonResponse(array $data, int $status = 200): never
{
    http_response_code($status);
    echo json_encode($data, JSON_UNESCAPED_UNICODE);
    exit;
}

function requireUser(): string
{
    if (empty($_SESSION['user'])) {
        jsonResponse(['error' => 'Não autenticado'], 401);
    }
    return (string) $_SESSION['user'];
}

switch ($action) {
    case 'login':
        $username = trim((string) ($input['username'] ?? ''));
        $password = (string) ($input['password'] ?? '');
        $stmt = $pdo->prepare('SELECT username, password_hash FROM users WHERE username = ?');
        $stmt->execute([$username]);
        $user = $stmt->fetch();

        if (!$user || !password_verify($password, $user['password_hash'])) {
            jsonResponse(['error' => 'Credenciais inválidas'], 401);
        }

        $_SESSION['user'] = $user['username'];
        $_SESSION['client_id'] = 'web_' . bin2hex(random_bytes(8));
        jsonResponse(['ok' => true, 'user' => $user['username'], 'client_id' => $_SESSION['client_id']]);

    case 'logout':
        session_destroy();
        jsonResponse(['ok' => true]);

    case 'host_register':
        // Chamado pelo agente (Debian 13 / macOS 27) para anunciar presença
        $hostId = trim((string) ($input['host_id'] ?? ''));
        $name = trim((string) ($input['name'] ?? $hostId));
        $os = trim((string) ($input['os'] ?? 'unknown'));
        $token = (string) ($input['token'] ?? '');

        if ($hostId === '' || $token === '') {
            jsonResponse(['error' => 'host_id e token são obrigatórios'], 400);
        }

        $tokenHash = hash_hmac('sha256', $token, $config['app_secret']);
        $stmt = $pdo->prepare('
            INSERT INTO hosts (host_id, name, os, token_hash, last_seen)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(host_id) DO UPDATE SET
                name = excluded.name,
                os = excluded.os,
                last_seen = excluded.last_seen
            WHERE hosts.token_hash = excluded.token_hash
        ');
        $stmt->execute([$hostId, $name, $os, $tokenHash, time()]);

        jsonResponse(['ok' => true, 'host_id' => $hostId]);

    case 'hosts':
        requireUser();
        $cutoff = time() - 60; // Online nos últimos 60s
        $stmt = $pdo->prepare('SELECT host_id, name, os, last_seen FROM hosts WHERE last_seen >= ? ORDER BY name');
        $stmt->execute([$cutoff]);
        jsonResponse(['hosts' => $stmt->fetchAll()]);

    case 'server_config':
        requireUser();
        if ($_SERVER['REQUEST_METHOD'] === 'POST') {
            $newCfg = [
                'stun_urls' => array_values(array_filter(array_map('trim', (array) ($input['stun_urls'] ?? $config['stun_urls'])))),
                'turn_urls' => array_values(array_filter(array_map('trim', (array) ($input['turn_urls'] ?? $config['turn_urls'])))),
                'turn_ttl' => max(60, (int) ($input['turn_ttl'] ?? $config['turn_ttl'])),
            ];
            if (isset($input['turn_secret']) && $input['turn_secret'] !== '') {
                $newCfg['turn_secret'] = (string) $input['turn_secret'];
            }
            $dir = dirname($overrideFile);
            if (!is_dir($dir)) {
                mkdir($dir, 0750, true);
            }
            file_put_contents($overrideFile, json_encode($newCfg, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES));
            jsonResponse(['ok' => true]);
        }
        jsonResponse([
            'stun_urls' => $config['stun_urls'],
            'turn_urls' => $config['turn_urls'],
            'turn_ttl' => $config['turn_ttl'],
            'has_turn_secret' => $config['turn_secret'] !== '',
        ]);

    case 'ice_servers':
        requireUser();
        $servers = [];
        if (!empty($config['stun_urls'])) {
            $servers[] = ['urls' => $config['stun_urls']];
        }
        if (!empty($config['turn_urls']) && $config['turn_secret'] !== '') {
            $expiry = time() + (int) $config['turn_ttl'];
            $username = $expiry . ':' . ($_SESSION['client_id'] ?? 'web');
            $credential = base64_encode(hash_hmac('sha1', $username, $config['turn_secret'], true));
            $servers[] = [
                'urls' => $config['turn_urls'],
                'username' => $username,
                'credential' => $credential,
            ];
        }
        jsonResponse(['iceServers' => $servers]);

    case 'signal':
        // Aceita sinal do cliente web (sessão) ou do agente (host_id + token)
        $senderId = '';
        if (!empty($_SESSION['client_id'])) {
            $senderId = (string) $_SESSION['client_id'];
        } elseif (!empty($input['host_id']) && !empty($input['token'])) {
            $stmt = $pdo->prepare('SELECT host_id, token_hash FROM hosts WHERE host_id = ?');
            $stmt->execute([(string) $input['host_id']]);
            $host = $stmt->fetch();
            $expected = hash_hmac('sha256', (string) $input['token'], $config['app_secret']);
            if (!$host || !hash_equals($host['token_hash'], $expected)) {
                jsonResponse(['error' => 'Host não autorizado'], 401);
            }
            $senderId = (string) $host['host_id'];
        } else {
            jsonResponse(['error' => 'Não autenticado'], 401);
        }

        $targetId = trim((string) ($input['target_id'] ?? ''));
        $type = trim((string) ($input['type'] ?? ''));
        $payload = json_encode($input['payload'] ?? null);

        if ($targetId === '' || $type === '') {
            jsonResponse(['error' => 'target_id e type são obrigatórios'], 400);
        }

        $stmt = $pdo->prepare(
            'INSERT INTO signals (target_id, sender_id, type, payload, created_at) VALUES (?, ?, ?, ?, ?)'
        );
        $stmt->execute([$targetId, $senderId, $type, $payload, time()]);

        // Limpeza simples de sinais antigos (> 2 min)
        $pdo->prepare('DELETE FROM signals WHERE created_at < ?')->execute([time() - 120]);

        jsonResponse(['ok' => true]);

    case 'poll':
        // SSE simples para receber sinais sem daemon extra
        $targetId = '';
        if (!empty($_SESSION['client_id'])) {
            $targetId = (string) $_SESSION['client_id'];
        } elseif (!empty($_GET['host_id']) && !empty($_GET['token'])) {
            $stmt = $pdo->prepare('SELECT host_id, token_hash FROM hosts WHERE host_id = ?');
            $stmt->execute([(string) $_GET['host_id']]);
            $host = $stmt->fetch();
            $expected = hash_hmac('sha256', (string) $_GET['token'], $config['app_secret']);
            if (!$host || !hash_equals($host['token_hash'], $expected)) {
                jsonResponse(['error' => 'Host não autorizado'], 401);
            }
            $targetId = (string) $host['host_id'];
        } else {
            jsonResponse(['error' => 'Não autenticado'], 401);
        }

        header('Content-Type: text/event-stream');
        header('Cache-Control: no-cache');
        header('X-Accel-Buffering: no');

        $lastId = (int) ($_GET['last_id'] ?? 0);
        $end = time() + 25; // Encerra antes do timeout padrão do PHP-FPM

        while (time() < $end) {
            $stmt = $pdo->prepare(
                'SELECT id, sender_id, type, payload FROM signals WHERE target_id = ? AND id > ? ORDER BY id ASC'
            );
            $stmt->execute([$targetId, $lastId]);
            $rows = $stmt->fetchAll();

            foreach ($rows as $row) {
                $lastId = (int) $row['id'];
                echo "id: {$lastId}\n";
                echo "event: signal\n";
                echo 'data: ' . json_encode([
                    'id' => $lastId,
                    'sender_id' => $row['sender_id'],
                    'type' => $row['type'],
                    'payload' => json_decode($row['payload'], true),
                ]) . "\n\n";
            }

            if (ob_get_level() > 0) {
                ob_flush();
            }
            flush();

            if (connection_aborted()) {
                break;
            }
            usleep(300000); // 300ms
        }
        exit;

    default:
        jsonResponse(['error' => 'Ação desconhecida'], 404);
}
