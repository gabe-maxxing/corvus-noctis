@echo off
rem Corvus OS for Windows: boots the bundled kernel in a QEMU window.
rem The Corvus shell runs in this console window too. Nothing on your PC is changed.
setlocal
title Corvus OS
set "HERE=%~dp0"
set "QEMU="

where qemu-system-aarch64 >nul 2>nul && set "QEMU=qemu-system-aarch64"
if not defined QEMU if exist "%ProgramFiles%\qemu\qemu-system-aarch64.exe" set "QEMU=%ProgramFiles%\qemu\qemu-system-aarch64.exe"
if not defined QEMU if exist "%HERE%qemu\qemu-system-aarch64.exe" set "QEMU=%HERE%qemu\qemu-system-aarch64.exe"

if defined QEMU goto :boot
echo Corvus needs QEMU, a free virtual machine.
where winget >nul 2>nul || goto :manual
set "ANSWER="
set /p "ANSWER=Install it now with winget? [Y/n] "
if /i "%ANSWER%"=="n" goto :manual
winget install --id SoftwareFreedomConservancy.QEMU -e --accept-source-agreements --accept-package-agreements
if exist "%ProgramFiles%\qemu\qemu-system-aarch64.exe" set "QEMU=%ProgramFiles%\qemu\qemu-system-aarch64.exe"
if not defined QEMU goto :manual

:boot

echo.
echo    .__
echo    (o )^>   C O R V U S
echo    / )     running in a VM on Windows
echo   /_/
echo.
echo Booting. Type here, or click HELP in the Corvus window. Quit: close the window.
set "PYTHON=python"
python --version >nul 2>nul && goto :bridge
set "PYTHON=py -3"
py -3 --version >nul 2>nul && goto :bridge
echo Diogenes needs Python 3: winget install Python.Python.3.13
pause
exit /b 1
:bridge
rem The release workflow owns packaging; fetch the bridge beside older ZIPs.
if not exist "%HERE%diogenes_bridge.py" powershell -NoProfile -Command "Invoke-WebRequest 'https://raw.githubusercontent.com/gabe-maxxing/corvus-noctis/main/tools/diogenes_bridge.py' -OutFile ($env:HERE + 'diogenes_bridge.py')"
if not exist "%HERE%DIOGENES.md" powershell -NoProfile -Command "Invoke-WebRequest 'https://raw.githubusercontent.com/gabe-maxxing/corvus-noctis/main/diogenes/DIOGENES.md' -OutFile ($env:HERE + 'DIOGENES.md')"
if not exist "%HERE%diogenes_bridge.py" exit /b 1
if not exist "%HERE%DIOGENES.md" exit /b 1
%PYTHON% "%HERE%diogenes_bridge.py" --persona "%HERE%DIOGENES.md" -- "%QEMU%" -machine virt -cpu cortex-a57 -nic none -serial stdio -monitor none -device ramfb -global virtio-mmio.force-legacy=false -device virtio-tablet-device -kernel "%HERE%corvus-os.elf"
goto :eof

:manual
echo Install QEMU from https://qemu.weilnetz.de/w64/ (or: winget install SoftwareFreedomConservancy.QEMU)
echo and run Corvus.bat again.
pause
