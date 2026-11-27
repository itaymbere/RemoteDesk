@echo off
setlocal
cd /d "%~dp0agent"
echo Compilando RemoteDeskIT Agent (release)...
cargo build --release
if errorlevel 1 (
    echo Falha na compilacao!
    pause
    exit /b 1
)
echo Build concluido: target\release\ag-remotedesk-agent.exe
pause
