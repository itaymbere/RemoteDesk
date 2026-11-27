<?php
declare(strict_types=1);

return [
    // Altere em produção
    'app_secret' => getenv('APP_SECRET') ?: 'troque-esta-chave-secreta-em-producao',

    // Banco SQLite local (zero configuração extra)
    'db_path' => __DIR__ . '/data/remotedesk.sqlite',

    // Configuração STUN / TURN (coturn com use-auth-secret)
    'stun_urls' => [
        'stun:stun.l.google.com:19302',
    ],
    'turn_urls' => [
        // Exemplo: 'turn:seu-servidor.com:3478?transport=udp',
        // 'turns:seu-servidor.com:5349?transport=tcp',
    ],
    'turn_secret' => getenv('TURN_SECRET') ?: '',
    'turn_ttl' => 3600,
];
