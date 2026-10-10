# Gemeinsame Einstellungen für Install, Start und Stopp unter Windows. Wird per ". " eingebunden.
# Ports, acestep.cpp-Version und Download-Adresse stehen in common.sh (eine Quelle für Mac und Windows).

$ROOT   = Split-Path -Parent $PSScriptRoot
$ENGINE = Join-Path $ROOT "engine\acestep.cpp"
$MODELS = Join-Path $ENGINE "models"
$DATA   = Join-Path $ROOT "data"
$CONF   = Join-Path $DATA "engine.conf"      # vom Installer geschrieben: MODE, LM_FILE

# KEY=value-Zeilen aus common.sh übernehmen (PORT, SYNTH_PORT, LM_PORT, ACESTEP_REPO, ACESTEP_REV, HF_BASE)
foreach ($line in Get-Content (Join-Path $PSScriptRoot "common.sh")) {
    if ($line -match '^(PORT|SYNTH_PORT|LM_PORT|ACESTEP_REPO|ACESTEP_REV|HF_BASE)="?([^"#]+?)"?\s*(#.*)?$') {
        Set-Variable -Name $Matches[1] -Value $Matches[2].Trim()
    }
}

# Fertige Windows-Binaries des acestep.cpp-Autors (siehe README von acestep.cpp, Abschnitt Windows)
$PREBUILT_URL = "https://www.serveurperso.com/temp/acestep.cpp-win64/build/Release/"

# Werkzeuge, die der Installer in den Benutzerordner legt (uv, cmake)
$env:PATH = "$env:USERPROFILE\.local\bin;$env:LOCALAPPDATA\Programs\Git\cmd;$env:ProgramFiles\Git\cmd;$env:PATH"

function Ace-Server {
    foreach ($p in @("build\Release\ace-server.exe", "build\ace-server.exe")) {
        $f = Join-Path $ENGINE $p
        if (Test-Path $f) { return $f }
    }
    return $null
}

function Port-Laeuft([int]$port) {
    return [bool](Get-NetTCPConnection -State Listen -LocalPort $port -ErrorAction SilentlyContinue)
}

function Port-Pids([int]$port) {
    return @(Get-NetTCPConnection -State Listen -LocalPort $port -ErrorAction SilentlyContinue |
             Select-Object -ExpandProperty OwningProcess -Unique)
}

# MODE=split  -> Sprachmodell auf CPU (Port 8086), Synthese auf der Grafikkarte (Port 8085)
# MODE=single -> alles auf einem Server (Port 8085)
function Lade-Conf {
    $script:MODE = "single"
    $script:LM_FILE = ""
    if (Test-Path $CONF) {
        foreach ($line in Get-Content $CONF) {
            if ($line -match '^MODE=(.+)$')    { $script:MODE = $Matches[1].Trim() }
            if ($line -match '^LM_FILE=(.+)$') { $script:LM_FILE = $Matches[1].Trim() }
        }
    }
}

# Hintergrundprozess ohne Fenster starten, Ausgabe in eine Log-Datei
function Start-Hintergrund([string]$exe, [string[]]$args, [string]$log) {
    $err = [System.IO.Path]::ChangeExtension($log, ".err.log")
    return Start-Process -FilePath $exe -ArgumentList $args -WorkingDirectory $ROOT -WindowStyle Hidden `
        -RedirectStandardOutput $log -RedirectStandardError $err -PassThru
}

function Taste-Zum-Schliessen {
    Write-Host ""
    Write-Host "Beliebige Taste drücken zum Schließen..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
}
