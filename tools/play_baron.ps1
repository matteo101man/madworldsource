# Launch mw-play (modern look), holding the shared .bevy-instance.lock while it runs.
$root = Split-Path -Parent $PSScriptRoot
$lock = Join-Path $root '.bevy-instance.lock'
New-Item -ItemType Directory -Path $lock -ErrorAction Stop | Out-Null
Set-Content -Path (Join-Path $lock 'owner.txt') -Value 'play_baron.ps1 (user play session)' -Encoding utf8
try {
    $env:MW_LOOK = 'modern'
    # no argument: the stage select (F9 in a fight returns to it); otherwise that stage directly
    if ($args.Count -gt 0) { $env:MW_STAGE = $args[0] } else { Remove-Item Env:MW_STAGE -ErrorAction SilentlyContinue }
    & 'D:/GDecomp/build-cache/bevy-0.20/release/mw-play.exe'
    # the stage select relaunches mw-play as a new process: hold the lock until the last one closes
    while (Get-Process mw-play -ErrorAction SilentlyContinue) { Start-Sleep -Seconds 1 }
} finally {
    Remove-Item -Recurse -Force $lock
}
