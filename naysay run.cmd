@echo off
REM ============================================================
REM  naysay run.cmd -- thin wrapper that just runs naysay.exe
REM  naysay.exe itself handles key prompting + TUI launch.
REM ============================================================
cd /d "%~dp0"

target\release\naysay.exe

if errorlevel 1 (
  echo.
  echo   ============================================================
  echo   naysay exited with an error.
  echo.
  echo   Debug logs:
  echo     %LOCALAPPDATA%\naysay\session.log
  echo     %LOCALAPPDATA%\naysay\panic.log
  echo   ============================================================
  pause
)
