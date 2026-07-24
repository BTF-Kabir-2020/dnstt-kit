@echo off
REM Launcher: run from dnstt-kit folder as:  dns-cli.cmd serve
REM Pick the newest among release / debug / dist (avoids stale release hiding newer debug).
setlocal
set "ROOT=%~dp0"
set "EXE="
for /f "usebackq delims=" %%I in (`powershell -NoProfile -Command "$c=@('%ROOT%target\release\dns-cli.exe','%ROOT%target\debug\dns-cli.exe','%ROOT%dist\windows-x86_64\dns-cli.exe'); $c | Where-Object { Test-Path $_ } | Sort-Object { (Get-Item $_).LastWriteTimeUtc } -Descending | Select-Object -First 1"`) do set "EXE=%%I"
if not defined EXE (
  echo [dns-cli.cmd] binary not found. Build first:
  echo   cd /d "%ROOT%"
  echo   cargo build -p dns-cli --release
  exit /b 1
)
cd /d "%ROOT%"
"%EXE%" %*
exit /b %ERRORLEVEL%
