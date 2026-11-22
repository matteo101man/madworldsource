@echo off
rem Play the original-core frontend with the effect-request trace (tools/upstream-native-core/port/runtime/game/trace_effects.cpp).
rem The trace goes to evidence\native-effects-trace-manual.log; close the game window to finish.
set MW_TRACE_EFFECTS=1
"D:\GDecomp\build-cache\bevy-0.20\debug\madworld-native.exe" 2> "%~dp0..\evidence\native-effects-trace-manual.log"
echo Game closed; trace written to evidence\native-effects-trace-manual.log
