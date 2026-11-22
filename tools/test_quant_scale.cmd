@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 exit /b 1
cd /d "D:\Games\gamesfordecomp\ports\madworld-rinrin\tools\upstream-native-core\port\build"
cl /nologo /std:c++20 /EHsc /fp:strict /O2 /I../runtime/cpu ../tests/quant_scale.cpp /Foobj/tests/quant_scale.obj /Fequant_scale.exe
if errorlevel 1 exit /b 1
quant_scale.exe
exit /b %errorlevel%
