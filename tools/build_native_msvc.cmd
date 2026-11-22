@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 exit /b 1
cd /d "D:\Games\gamesfordecomp\ports\madworld-rinrin\tools\upstream-native-core"
if errorlevel 1 exit /b 1
python port/build.py --jobs 4 --cxx "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.39.33519\bin\Hostx64\x64\cl.exe"
exit /b %errorlevel%
