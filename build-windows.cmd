@echo off
setlocal
where cl >nul 2>&1
if errorlevel 1 (
    echo Visual Studio Build Tools (cl.exe) nao encontrado. Instalando...
    winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    echo Reabra o terminal apos a instalacao e execute build-windows.cmd novamente.
    pause
    exit /b 1
)

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
