@echo off
rem Installs or updates Omarchy Overlay. An older installed version is replaced;
rem settings are kept. Add --quiet for no messages. Exit code: 0 done, 1 failed, 2 cancelled.
"%~dp0OmarchyOverlay.exe" --install %*
exit /b %errorlevel%
