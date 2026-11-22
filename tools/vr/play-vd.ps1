# MadWorld in VR over Virtual Desktop: the game on Virtual Desktop's own OpenXR runtime (VDXR), straight into VR.
# Connect the headset in Virtual Desktop first. Session drops switch to the monitor and back (MW_VR_AUTO).
$env:XR_RUNTIME_JSON = 'C:\Program Files\Virtual Desktop Streamer\OpenXR\virtualdesktop-openxr.json'
$env:MW_VR = '1'
$env:MW_VR_AUTO = '1'
& 'D:\GDecomp\build-cache\bevy-0.20\release\mw-play.exe'
