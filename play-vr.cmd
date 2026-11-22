@echo off
rem MadWorld in VR: Quest 3 over Link cable / Air Link (Meta Quest Link app), or Virtual Desktop / SteamVR,
rem whichever is the active OpenXR runtime. Put the headset on and start Link first. Stage: argument or MW_STAGE
rem (rinrin, elise, baron, mathilda, street, town); the VR menu (left menu button) switches stages too.
rem   play-vr.cmd            RinRin's arena
rem   play-vr.cmd street     Asia Town's streets
rem   set MW_VR=sim ^& play-vr.cmd   the desktop VR simulator (no headset)
setlocal
if "%MW_VR%"=="" set MW_VR=1
if not "%~1"=="" set MW_STAGE=%~1
set EXE=D:\GDecomp\build-cache\bevy-0.20\release\mw-play.exe
if not exist "%EXE%" (
  echo Building the release game first...
  pushd "%~dp0" && cargo build --release -p mw-game --bin mw-play && popd
)
"%EXE%"
