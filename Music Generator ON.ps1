# Startet Modellserver + Web-App und öffnet den Browser.
. (Join-Path $PSScriptRoot "scripts\common.ps1")
Set-Location $ROOT

# MODE=split  -> Sprachmodell auf CPU (Port 8086), Synthese auf der Grafikkarte (Port 8085)
# MODE=single -> alles auf einem Server (Port 8085)
$MODE = "single"; $LM_FILE = ""; $VAE_CHUNK = ""
if (Test-Path $CONF) {
    foreach ($line in Get-Content $CONF) {
        if ($line -match '^MODE=(.+)$')      { $MODE = $Matches[1].Trim() }
        if ($line -match '^LM_FILE=(.+)$')   { $LM_FILE = $Matches[1].Trim() }
        if ($line -match '^VAE_CHUNK=(.+)$') { $VAE_CHUNK = $Matches[1].Trim() }
    }
}
New-Item -ItemType Directory -Force -Path $DATA | Out-Null

# Gemeinsame Server-Optionen; VAE_CHUNK (vom Installer nach Grafikspeicher gesetzt) kachelt den VAE-Decoder
$SERVER_ARGS = @("--models", "`"$MODELS`"", "--max-batch", "1")
if ($VAE_CHUNK) { $SERVER_ARGS += @("--vae-chunk", $VAE_CHUNK) }

$ACE = Ace-Server
if (-not $ACE) {
    Write-Host "Noch nicht installiert. Bitte zuerst 'Install' starten."
    Taste-Zum-Schliessen
    exit 1
}

function Fehler([string]$text, [string]$log) {
    Write-Host "$text Log-Datei ($log):"
    if (Test-Path $log) { Get-Content $log -Tail 30 }
    $err = [System.IO.Path]::ChangeExtension($log, ".err.log")
    if (Test-Path $err) { Get-Content $err -Tail 30 }
    Taste-Zum-Schliessen
    exit 1
}

# Zeitstempel der neusten Quelldatei - daran erkennt man veralteten Code.
function Neuste-Quelle {
    $f = Get-ChildItem -Recurse -File -Path (Join-Path $ROOT "app"), (Join-Path $ROOT "static") -Include *.py, *.js, *.html, *.css |
         Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($f) { return $f.LastWriteTime } else { return $null }
}

# --- Modellserver -----------------------------------------------------------
if ($MODE -eq "split") {
    if (Port-Laeuft $LM_PORT) {
        Write-Host "Sprachmodell-Server läuft bereits (Port $LM_PORT)."
    } else {
        Write-Host "Starte Sprachmodell-Server (CPU)..."
        $env:GGML_BACKEND = "CPU"
        Start-Hintergrund $ACE (@("--host", "127.0.0.1", "--port", $LM_PORT) + $SERVER_ARGS) (Join-Path $DATA "ace-lm.log") | Out-Null
        Remove-Item Env:\GGML_BACKEND
    }
}

if (Port-Laeuft $SYNTH_PORT) {
    Write-Host "Synthese-Server läuft bereits (Port $SYNTH_PORT)."
} else {
    Write-Host "Starte Synthese-Server (Grafikkarte)..."
    Start-Hintergrund $ACE (@("--host", "127.0.0.1", "--port", $SYNTH_PORT) + $SERVER_ARGS) (Join-Path $DATA "ace-server.log") | Out-Null
}

# --- Web-App ----------------------------------------------------------------
$pids = Port-Pids $PORT
if ($pids.Count) {
    $proc = Get-Process -Id $pids[0] -ErrorAction SilentlyContinue
    $neuste = Neuste-Quelle
    if ($proc -and $neuste -and $neuste -gt $proc.StartTime) {
        Write-Host "Der Code ist neuer als die laufende Web-App - Neustart."
        Stop-Process -Id $pids -Force -ErrorAction SilentlyContinue
        for ($i = 0; $i -lt 20 -and (Port-Laeuft $PORT); $i++) { Start-Sleep -Milliseconds 500 }
        $pids = @()
    } else {
        Write-Host "Web-App läuft bereits auf Port $PORT."
    }
}
if (-not $pids.Count) {
    Write-Host "Starte Web-App..."
    Start-Hintergrund "uv" @("run", "uvicorn", "app.main:app", "--host", "127.0.0.1", "--port", $PORT) (Join-Path $DATA "server.log") | Out-Null
}

# --- Warten, bis alles antwortet --------------------------------------------
for ($i = 0; $i -lt 60; $i++) {
    if ((Port-Laeuft $PORT) -and (Port-Laeuft $SYNTH_PORT) -and ($MODE -ne "split" -or (Port-Laeuft $LM_PORT))) { break }
    Start-Sleep -Milliseconds 500
}
if (-not (Port-Laeuft $PORT))       { Fehler "Web-App ist nicht rechtzeitig gestartet." (Join-Path $DATA "server.log") }
if (-not (Port-Laeuft $SYNTH_PORT)) { Fehler "Synthese-Server ist nicht rechtzeitig gestartet." (Join-Path $DATA "ace-server.log") }
if ($MODE -eq "split" -and -not (Port-Laeuft $LM_PORT)) { Fehler "Sprachmodell-Server ist nicht rechtzeitig gestartet." (Join-Path $DATA "ace-lm.log") }
Write-Host "Alle Server laufen."

Start-Process "http://localhost:$PORT"
Write-Host ""
Write-Host "Music Generator ist offen unter http://localhost:$PORT"
Write-Host "Zum Beenden: 'Music Generator OFF' verwenden."
Start-Sleep -Seconds 2
exit 0
