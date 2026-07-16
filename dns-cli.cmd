@echo off
REM Launcher: run from dnstt-kit folder as:  dns-cli.cmd serve
REM Prefer release, fall back to debug / dist.
setlocal
set "ROOT=%~dp0"
set "EXE="
if exist "%ROOT%target\release\dns-cli.exe" set "EXE=%ROOT%target\release\dns-cli.exe"
if not defined EXE if exist "%ROOT%dist\windows-x86_64\dns-cli.exe" set "EXE=%ROOT%dist\windows-x86_64\dns-cli.exe"
if not defined EXE if exist "%ROOT%target\debug\dns-cli.exe" set "EXE=%ROOT%target\debug\dns-cli.exe"
if not defined EXE (
  echo [dns-cli.cmd] binary not found. Build first:
  echo   cd /d "%ROOT%"
  echo   cargo build -p dns-cli --release
  exit /b 1
)
cd /d "%ROOT%"
"%EXE%" %*
exit /b %ERRORLEVEL%
