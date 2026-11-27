@echo off
setlocal
set SERVER_URL=http://192.168.0.113:8080/api.php
set HOST_ID=windows-host-01
set HOST_NAME=Windows 11
set HOST_TOKEN=changeme-token
set RUST_LOG=info

cd /d "%~dp0agent"
if not exist ".\target\release\ag-remotedesk-agent.exe" (
    call "%~dp0build-windows.cmd"
    if errorlevel 1 (
        pause
        exit /b 1
    )
)
.\target\release\ag-remotedesk-agent.exe
pause
