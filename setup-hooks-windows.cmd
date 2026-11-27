@echo off
setlocal
cd /d "%~dp0"
git config core.hooksPath scripts/hooks
echo Hook pre-commit ativado com sucesso!
pause
