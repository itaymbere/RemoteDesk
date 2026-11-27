<?php
declare(strict_types=1);
session_start();
$loggedIn = !empty($_SESSION['user']);
?>
<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Ag-RemoteDesk</title>
  <style>
    body { font-family: system-ui, sans-serif; background: #0f172a; color: #e2e8f0; margin: 0; padding: 24px; }
    .card { max-width: 960px; margin: 0 auto; background: #1e293b; border-radius: 12px; padding: 20px; }
    input, button { padding: 10px 14px; border-radius: 8px; border: 1px solid #334155; background: #0f172a; color: #f8fafc; }
    button { background: #3b82f6; border-color: #3b82f6; cursor: pointer; font-weight: 600; }
    button:hover { background: #2563eb; }
    .hosts { display: grid; gap: 12px; margin-top: 16px; }
    .host-item { display: flex; justify-content: space-between; align-items: center; padding: 12px; background: #0f172a; border-radius: 8px; }
    video { width: 100%; background: #000; border-radius: 8px; margin-top: 16px; outline: none; touch-action: none; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Ag-RemoteDesk</h1>

    <?php if (!$loggedIn): ?>
      <form id="login-form">
        <input type="text" id="username" placeholder="Usuário (admin)" required>
        <input type="password" id="password" placeholder="Senha (admin123)" required>
        <button type="submit">Entrar</button>
      </form>
      <p id="login-error" style="color:#f87171;"></p>
    <?php else: ?>
      <div style="display:flex; justify-content:space-between; align-items:center;">
        <span>Conectado como <strong><?= htmlspecialchars((string) $_SESSION['user']) ?></strong></span>
        <div style="display:flex; gap:8px;">
          <button id="toggle-server-cfg" style="background:#334155; border-color:#334155;">Config. Servidor</button>
          <button id="logout-btn" style="background:#475569; border-color:#475569;">Sair</button>
        </div>
      </div>

      <div id="server-cfg-panel" style="display:none; margin-top:16px; padding:16px; background:#0f172a; border-radius:8px;">
        <h3 style="margin-top:0;">Configuração do Servidor</h3>
        <div style="display:grid; gap:8px;">
          <input type="text" id="cfg-stun" placeholder="STUN URLs (separadas por vírgula)">
          <input type="text" id="cfg-turn" placeholder="TURN URLs (ex: turn:seu-servidor.com:3478?transport=udp, turns:seu-servidor.com:5349?transport=tcp)">
          <input type="password" id="cfg-turn-secret" placeholder="TURN Secret (deixe vazio para manter)">
          <input type="number" id="cfg-turn-ttl" placeholder="TURN TTL (segundos)" value="3600">
          <button id="save-server-cfg">Salvar Servidor</button>
        </div>
      </div>

      <div class="hosts" id="hosts-list">Carregando hosts...</div>
      <p id="status" style="color:#94a3b8; margin-top:12px;">Pronto para conectar.</p>

      <div id="client-cfg-panel" style="display:none; margin-top:12px; padding:16px; background:#0f172a; border-radius:8px;">
        <h3 style="margin-top:0;">Configuração do Cliente Remoto</h3>
        <div style="display:flex; flex-wrap:wrap; gap:8px; align-items:center;">
          <input type="number" id="cli-fps" placeholder="FPS (ex: 15)" value="15" style="width:110px;">
          <input type="text" id="cli-bitrate" placeholder="Bitrate (ex: 1500k)" value="1500k" style="width:140px;">
          <input type="text" id="cli-name" placeholder="Nome do Host">
          <button id="send-client-cfg">Aplicar e Reiniciar Cliente</button>
        </div>
      </div>

      <video id="remote-video" autoplay playsinline muted tabindex="0"></video>
    <?php endif; ?>
  </div>

  <script>
    const loginForm = document.getElementById('login-form');
    if (loginForm) {
      loginForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const res = await fetch('api.php?action=login', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            username: document.getElementById('username').value,
            password: document.getElementById('password').value,
          }),
        });
        if (res.ok) location.reload();
        else document.getElementById('login-error').textContent = 'Login inválido';
      });
    }

    const logoutBtn = document.getElementById('logout-btn');
    if (logoutBtn) {
      logoutBtn.addEventListener('click', async () => {
        await fetch('api.php?action=logout', { method: 'POST' });
        location.reload();
      });

      let pc = null;
      let dc = null;
      let currentHostId = null;

      document.getElementById('toggle-server-cfg').addEventListener('click', async () => {
        const panel = document.getElementById('server-cfg-panel');
        const show = panel.style.display === 'none';
        panel.style.display = show ? 'block' : 'none';
        if (show) {
          const res = await fetch('api.php?action=server_config');
          const cfg = await res.json();
          document.getElementById('cfg-stun').value = (cfg.stun_urls || []).join(', ');
          document.getElementById('cfg-turn').value = (cfg.turn_urls || []).join(', ');
          document.getElementById('cfg-turn-ttl').value = cfg.turn_ttl || 3600;
        }
      });

      document.getElementById('save-server-cfg').addEventListener('click', async () => {
        const res = await fetch('api.php?action=server_config', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            stun_urls: document.getElementById('cfg-stun').value.split(','),
            turn_urls: document.getElementById('cfg-turn').value.split(','),
            turn_secret: document.getElementById('cfg-turn-secret').value,
            turn_ttl: Number(document.getElementById('cfg-turn-ttl').value || 3600),
          }),
        });
        if (res.ok) {
          document.getElementById('status').textContent = 'Configuração do servidor salva.';
          document.getElementById('server-cfg-panel').style.display = 'none';
        }
      });

      document.getElementById('send-client-cfg').addEventListener('click', () => {
        if (!dc || dc.readyState !== 'open') return;
        dc.send(JSON.stringify({
          type: 'config',
          fps: Number(document.getElementById('cli-fps').value || 15),
          bitrate: document.getElementById('cli-bitrate').value || '1500k',
          host_name: document.getElementById('cli-name').value || '',
        }));
        document.getElementById('status').textContent = 'Configuração enviada! O cliente será reiniciado para reconexão.';
      });

      async function loadHosts() {
        const res = await fetch('api.php?action=hosts');
        const data = await res.json();
        const list = document.getElementById('hosts-list');
        if (!data.hosts || data.hosts.length === 0) {
          list.innerHTML = '<p>Nenhum host online (Debian 13 / macOS 27).</p>';
          return;
        }
        list.innerHTML = '';
        data.hosts.forEach((h) => {
          const div = document.createElement('div');
          div.className = 'host-item';
          div.innerHTML = `<div><strong>${h.name}</strong> <small>(${h.os})</small></div>`;
          const btn = document.createElement('button');
          btn.textContent = 'Conectar';
          btn.onclick = () => connectToHost(h.host_id);
          div.appendChild(btn);
          list.appendChild(div);
        });
      }

      async function connectToHost(hostId) {
        const statusEl = document.getElementById('status');
        statusEl.textContent = 'Conectando a ' + hostId + '...';
        currentHostId = hostId;
        const iceRes = await fetch('api.php?action=ice_servers');
        const { iceServers } = await iceRes.json();

        pc = new RTCPeerConnection({ iceServers });
        pc.onconnectionstatechange = () => {
          statusEl.textContent = 'WebRTC: ' + pc.connectionState;
          document.getElementById('client-cfg-panel').style.display =
            pc.connectionState === 'connected' ? 'block' : 'none';
        };
        dc = pc.createDataChannel('input');

        const videoEl = document.getElementById('remote-video');
        const sendInput = (evt) => {
          if (dc && dc.readyState === 'open') {
            dc.send(JSON.stringify(evt));
          }
        };

        videoEl.addEventListener('mousemove', (e) => {
          const rect = videoEl.getBoundingClientRect();
          sendInput({
            type: 'mousemove',
            x: (e.clientX - rect.left) / rect.width,
            y: (e.clientY - rect.top) / rect.height,
          });
        });
        videoEl.addEventListener('mousedown', (e) => sendInput({ type: 'mousedown', button: e.button }));
        videoEl.addEventListener('mouseup', (e) => sendInput({ type: 'mouseup', button: e.button }));
        videoEl.addEventListener('wheel', (e) => sendInput({ type: 'wheel', deltaY: e.deltaY }));
        videoEl.addEventListener('contextmenu', (e) => e.preventDefault());
        videoEl.addEventListener('keydown', (e) => {
          e.preventDefault();
          sendInput({ type: 'keydown', key: e.key });
        });
        videoEl.addEventListener('keyup', (e) => {
          e.preventDefault();
          sendInput({ type: 'keyup', key: e.key });
        });

        const touchCoords = (touch) => {
          const rect = videoEl.getBoundingClientRect();
          return {
            x: (touch.clientX - rect.left) / rect.width,
            y: (touch.clientY - rect.top) / rect.height,
          };
        };

        videoEl.addEventListener('touchstart', (e) => {
          if (e.touches.length === 0) return;
          e.preventDefault();
          const { x, y } = touchCoords(e.touches[0]);
          sendInput({ type: 'mousemove', x, y });
          sendInput({ type: 'mousedown', button: 0 });
        }, { passive: false });

        videoEl.addEventListener('touchmove', (e) => {
          if (e.touches.length === 0) return;
          e.preventDefault();
          const { x, y } = touchCoords(e.touches[0]);
          sendInput({ type: 'mousemove', x, y });
        }, { passive: false });

        videoEl.addEventListener('touchend', (e) => {
          e.preventDefault();
          sendInput({ type: 'mouseup', button: 0 });
        }, { passive: false });

        pc.ontrack = (e) => {
          document.getElementById('remote-video').srcObject = e.streams[0];
        };

        pc.onicecandidate = async (e) => {
          if (e.candidate) {
            await fetch('api.php?action=signal', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ target_id: hostId, type: 'ice', payload: e.candidate }),
            });
          }
        };

        startSSE();

        const offer = await pc.createOffer({ offerToReceiveVideo: true });
        await pc.setLocalDescription(offer);
        await fetch('api.php?action=signal', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ target_id: hostId, type: 'offer', payload: offer }),
        });
      }

      function startSSE() {
        const es = new EventSource('api.php?action=poll');
        es.addEventListener('signal', async (e) => {
          const msg = JSON.parse(e.data);
          if (!pc || msg.sender_id !== currentHostId) return;
          if (msg.type === 'answer') {
            document.getElementById('status').textContent = 'Answer recebido, negociando ICE...';
            await pc.setRemoteDescription(new RTCSessionDescription(msg.payload));
          } else if (msg.type === 'ice') {
            await pc.addIceCandidate(new RTCIceCandidate(msg.payload));
          }
        });
      }

      loadHosts();
      setInterval(loadHosts, 30000);
    }
  </script>
</body>
</html>
