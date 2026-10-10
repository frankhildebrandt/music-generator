# Music Generator – Installer für Windows (NVIDIA-Grafikkarte empfohlen).
# Prüft den PC, installiert was fehlt, lädt Modellserver und Modelle und startet die App.
# Kann jederzeit erneut gestartet werden (z. B. nach "git pull"): Erledigtes wird übersprungen.
#
# Optionen:  -Yes         keine Rückfragen, Empfehlung übernehmen
#            -LM SIZE     Sprachmodell erzwingen: 0.6B, 1.7B oder 4B
#            -NoStart     App am Ende nicht starten
#            -Build       Modellserver selbst kompilieren statt fertige Binaries zu laden
#                         (braucht Visual Studio Build Tools + CUDA Toolkit)
param([switch]$Yes, [string]$LM = "", [switch]$NoStart, [switch]$Build)

. (Join-Path $PSScriptRoot "scripts\common.ps1")
Set-Location $ROOT
$ErrorActionPreference = "Continue"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Abbruch([string]$text) {
    Write-Host ""
    Write-Host "x $text"
    if (-not $Yes) { Taste-Zum-Schliessen }
    exit 1
}
function Schritt([string]$text) { Write-Host ""; Write-Host "> $text" }
function Ja([string]$frage) {   # Rückfrage mit Enter = ja
    if ($Yes) { return $true }
    $a = Read-Host "$frage [Enter = ja, n = nein]"
    return ($a -eq "" -or $a -match '^[jJyY]')
}
function Download([string]$url, [string]$ziel) {   # fortsetzbar, mit Fortschrittsbalken
    & curl.exe -L --fail -C - --progress-bar -o "$ziel.part" $url
    if ($LASTEXITCODE -ne 0) { return $false }
    Move-Item -Force "$ziel.part" $ziel
    return $true
}

New-Item -ItemType Directory -Force -Path $DATA | Out-Null
Write-Host "==========================================="
Write-Host "  MUSIC GENERATOR · Installation (Windows)"
Write-Host "==========================================="

# --- 1. PC prüfen ---------------------------------------------------------------
Schritt "PC prüfen"
$cs = Get-CimInstance Win32_ComputerSystem
$os = Get-CimInstance Win32_OperatingSystem
$RAM_GB = [int][math]::Floor($cs.TotalPhysicalMemory / 1GB)
$FREE_GB = [int][math]::Floor((Get-PSDrive -Name $ROOT.Substring(0, 1)).Free / 1GB)
$GPUS = @(Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name)
$NVIDIA = [bool]($GPUS | Where-Object { $_ -match 'NVIDIA' })
$VRAM_GB = 0
if (Get-Command nvidia-smi -ErrorAction SilentlyContinue) {
    $mem = (& nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits 2>$null | Select-Object -First 1)
    if ($mem) { $VRAM_GB = [int][math]::Round([double]$mem / 1024) }
}
# VAE-Decoder in Kacheln rechnen lassen (--vae-chunk, Latent-Frames pro Kachel, Standard 1024 = ein Stück).
# Ohne Kachelung braucht er für 30 s Audio 6,7 GB am Stück; auf einer 6-GB-Karte scheitert das.
$VAE_CHUNK = ""
if ($VRAM_GB) {
    if     ($VRAM_GB -lt 8)  { $VAE_CHUNK = "128" }
    elseif ($VRAM_GB -lt 12) { $VAE_CHUNK = "256" }
    elseif ($VRAM_GB -lt 16) { $VAE_CHUNK = "512" }
}
Write-Host "  Grafikkarte:     $($GPUS -join ', ')$(if ($VRAM_GB) { " ($VRAM_GB GB)" })"
Write-Host "  Arbeitsspeicher: $RAM_GB GB"
Write-Host "  Windows:         $($os.Caption) $($os.Version)"
Write-Host "  Frei auf der Festplatte: $FREE_GB GB"
if (-not $NVIDIA) {
    Write-Host ""
    Write-Host "  Hinweis: Keine NVIDIA-Grafikkarte gefunden. Getestet ist nur CUDA; mit AMD/Intel (Vulkan)"
    Write-Host "  oder nur CPU kann es sehr langsam sein."
    if (-not (Ja "  Trotzdem fortfahren?")) { Abbruch "Abgebrochen." }
}

# --- 2. Modellgröße wählen ------------------------------------------------------
if     ($RAM_GB -lt 12) { $EMPF = "0.6B" }
elseif ($RAM_GB -lt 24) { $EMPF = "1.7B" }
else                    { $EMPF = "4B" }
# Auf der Grafikkarte müssen Sprachmodell und KV-Cache (1 GB) zusammen hineinpassen, sonst stürzt der
# Modellserver ab (GTX 1060 6 GB: 4B lädt 4,2 GB, dann scheitert der Cache). Der Grafikspeicher begrenzt also.
if ($VRAM_GB) {
    $GROESSEN = @("0.6B", "1.7B", "4B")
    $VMAX = if ($VRAM_GB -lt 6) { "0.6B" } elseif ($VRAM_GB -lt 8) { "1.7B" } else { "4B" }
    if ($GROESSEN.IndexOf($VMAX) -lt $GROESSEN.IndexOf($EMPF)) { $EMPF = $VMAX }
}
$LM_SIZE = $LM
if (-not $LM_SIZE) {
    $LM_SIZE = $EMPF
    if (-not $Yes) {
        Write-Host ""
        Write-Host "  Sprachmodell (plant Aufbau, Tempo, Tonart; größer = bessere Songs, mehr Speicher):"
        Write-Host "    1) 0.6B  – 0,7 GB, für 8 GB Arbeitsspeicher, Grafikkarte unter 6 GB"
        Write-Host "    2) 1.7B  – 2,0 GB, für 16 GB, Grafikkarte mit 6 GB"
        Write-Host "    3) 4B    – 4,5 GB, ab 24 GB, Grafikkarte ab 8 GB"
        $a = Read-Host "  Auswahl [Enter = $EMPF empfohlen]"
        switch ($a) { "1" { $LM_SIZE = "0.6B" } "2" { $LM_SIZE = "1.7B" } "3" { $LM_SIZE = "4B" } }
    }
}
if ($LM_SIZE -notin @("0.6B", "1.7B", "4B")) { Abbruch "Unbekannte Modellgröße: $LM_SIZE (erlaubt: 0.6B, 1.7B, 4B)" }
$LM_FILE = "acestep-5Hz-lm-$LM_SIZE-Q8_0.gguf"
Write-Host "  -> Sprachmodell $LM_SIZE, Klangmodell SFT (hohe Qualität)"

# Dateien + Größe in MB (für die Speicherplatz-Prüfung)
$FILES = [ordered]@{}
foreach ($f in @("vae-BF16.gguf", "Qwen3-Embedding-0.6B-Q8_0.gguf", "acestep-v15-sft-Q8_0.gguf", $LM_FILE)) {
    if (-not $MODEL_SHA[$f]) { Abbruch "Datei $f fehlt in der Modell-Tabelle (scripts\common.sh)." }
    $FILES[$f] = $MODEL_MB[$f]
}
$NEED_MB = 0
foreach ($f in $FILES.Keys) { if (-not (Test-Path (Join-Path $MODELS $f))) { $NEED_MB += $FILES[$f] } }
$NEED_GB = [int][math]::Ceiling($NEED_MB / 1024) + 3   # + Modellserver, Python und etwas Platz für Songs
Write-Host "  Download: $([math]::Ceiling($NEED_MB / 1024)) GB · benötigt insgesamt etwa $NEED_GB GB frei"
if ($FREE_GB -lt $NEED_GB) { Abbruch "Zu wenig Speicherplatz: $FREE_GB GB frei, etwa $NEED_GB GB nötig." }
if (-not (Ja "  Installation starten?")) { Abbruch "Abgebrochen." }

# --- 3. Werkzeuge ---------------------------------------------------------------
Schritt "git"
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "  wird installiert (winget) ..."
    & winget install --id Git.Git -e --silent --accept-package-agreements --accept-source-agreements
    $env:PATH = "$env:ProgramFiles\Git\cmd;$env:LOCALAPPDATA\Programs\Git\cmd;$env:PATH"
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) { Abbruch "git konnte nicht installiert werden. Bitte von https://git-scm.com laden und 'Install' erneut starten." }
}
Write-Host "  $(& git --version)"

Schritt "uv (Python-Umgebung)"
if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    try { Invoke-RestMethod https://astral.sh/uv/install.ps1 | Invoke-Expression } catch { Abbruch "uv konnte nicht installiert werden: $_" }
    $env:PATH = "$env:USERPROFILE\.local\bin;$env:PATH"
    if (-not (Get-Command uv -ErrorAction SilentlyContinue)) { Abbruch "uv konnte nicht installiert werden." }
}
Write-Host "  $(& uv --version)"

Schritt "Python-Pakete der App"
# Python-Version festlegen: das neueste Python (3.15) hat noch keine fertigen Pakete (pydantic-core),
# uv würde sonst versuchen, es aus dem Quellcode zu bauen, und daran scheitern.
& uv sync --quiet --python 3.12
if ($LASTEXITCODE -ne 0) { Abbruch "Python-Pakete konnten nicht installiert werden." }
Write-Host "  ok"

# --- 4. Modellserver (acestep.cpp) ----------------------------------------------
Schritt "Modellserver acestep.cpp"
$ACE = Ace-Server
if ($ACE -and -not $Build) {
    Write-Host "  vorhanden: $ACE"
} elseif (-not $Build) {
    # Fertige Binaries des acestep.cpp-Autors (CUDA + Vulkan + CPU, Backend wird zur Laufzeit gewählt)
    Write-Host "  fertige Windows-Binaries werden geladen (ca. 180 MB) ..."
    $rel = Join-Path $ENGINE "build\Release"
    New-Item -ItemType Directory -Force -Path $rel | Out-Null
    $index = (Invoke-WebRequest -UseBasicParsing $PREBUILT_URL).Content
    $names = [regex]::Matches($index, 'href="([^"/?]+\.(exe|dll))"') | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
    if (-not $names) { Abbruch "Dateiliste der Binaries nicht lesbar ($PREBUILT_URL). Alternative: 'Install -Build' kompiliert selbst." }
    foreach ($n in $names) {
        Write-Host "  v $n"
        if (-not (Download "$PREBUILT_URL$n" (Join-Path $rel $n))) { Abbruch "Download von $n fehlgeschlagen. Install erneut starten setzt fort." }
    }
    "PREBUILT=$(Get-Date -Format s)" | Set-Content (Join-Path $rel "VERSION.txt")
    Write-Host "  fertig"
} else {
    # Selbst kompilieren: Quellcode in der getesteten Version, CUDA wenn vorhanden
    if (-not (Test-Path (Join-Path $ENGINE ".git"))) {
        New-Item -ItemType Directory -Force -Path (Join-Path $ROOT "engine") | Out-Null
        & git clone --quiet $ACESTEP_REPO $ENGINE
        if ($LASTEXITCODE -ne 0) { Abbruch "Download von acestep.cpp fehlgeschlagen." }
    }
    Push-Location $ENGINE
    & git cat-file -e "$ACESTEP_REV^{commit}" 2>$null
    if ($LASTEXITCODE -ne 0) { & git fetch --quiet origin }
    & git checkout --quiet $ACESTEP_REV
    if ($LASTEXITCODE -ne 0) { Pop-Location; Abbruch "acestep.cpp konnte nicht auf Version $ACESTEP_REV gebracht werden." }
    & git submodule update --init --recursive --quiet
    Pop-Location

    if ($ACE -and ((& $ACE --help 2>&1 | Select-Object -First 1) -match $ACESTEP_REV.Substring(0, 7))) {   # --help zeigt den kurzen Hash
        Write-Host "  bereits gebaut ($ACESTEP_REV)"
    } else {
        if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) { & uv tool install cmake | Out-Null }
        $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        $vs = if (Test-Path $vswhere) { & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath } else { "" }
        if (-not $vs) {
            Write-Host "  Visual Studio Build Tools fehlen. Installation (ca. 7 GB, Admin-Rechte):"
            Write-Host '    winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
            Abbruch "Danach 'Install -Build' erneut starten."
        }
        $cuda = $env:CUDA_PATH
        if (-not $cuda) { $cuda = [Environment]::GetEnvironmentVariable("CUDA_PATH", "Machine") }
        if ($NVIDIA -and -not $cuda) {
            Write-Host "  CUDA Toolkit fehlt. Installation (ca. 3 GB, Admin-Rechte):  winget install Nvidia.CUDA"
            Abbruch "Danach 'Install -Build' erneut starten (ohne CUDA: 'Install -Build' trotzdem möglich, dann nur CPU/Vulkan)."
        }
        $flags = @("-DGGML_CPU_ALL_VARIANTS=ON", "-DGGML_BACKEND_DL=ON")
        if ($cuda) { $flags += @("-DGGML_CUDA=ON", "-DCMAKE_CUDA_ARCHITECTURES=native") }
        Write-Host "  wird gebaut (dauert 10-30 Minuten) ..."
        Push-Location $ENGINE
        $log = Join-Path $DATA "build.log"
        & cmake -S . -B build @flags *> $log
        if ($LASTEXITCODE -eq 0) { & cmake --build build --config Release -j $env:NUMBER_OF_PROCESSORS *>> $log }
        $rc = $LASTEXITCODE
        Pop-Location
        if ($rc -ne 0) { Abbruch "Bauen fehlgeschlagen, Details in data\build.log" }
        Write-Host "  fertig"
    }
}
$ACE = Ace-Server
if (-not $ACE) { Abbruch "ace-server.exe nicht gefunden." }

# --- 5. Modelle -----------------------------------------------------------------
Schritt "Modelle laden"
New-Item -ItemType Directory -Force -Path $MODELS | Out-Null
foreach ($f in $FILES.Keys) {
    $ziel = Join-Path $MODELS $f
    if (Test-Path $ziel) { Write-Host "  ok $f"; continue }
    Write-Host "  v $f"
    if (-not (Download "$HF_BASE/$f" "$ziel.neu")) { Abbruch "Download von $f fehlgeschlagen. Install erneut starten setzt den Download fort." }
    Write-Host "    Prüfsumme ..."
    if ((Get-FileHash -Algorithm SHA256 "$ziel.neu").Hash.ToLower() -ne $MODEL_SHA[$f]) {
        Remove-Item -Force "$ziel.neu"
        Abbruch "Prüfsumme von $f stimmt nicht. Die Datei wurde gelöscht; Install erneut starten lädt sie neu."
    }
    Move-Item -Force "$ziel.neu" $ziel
}

# --- 6. Grafikkarte testen ------------------------------------------------------
# Prüft, ob das Sprachmodell auf der Grafikkarte brauchbar rechnet (auf dem Mac M4 tut es das nicht).
# Sonst läuft es auf der CPU (Modus "split"), sonst reicht ein Server (Modus "single").
Schritt "Grafikkarte testen"
$OLD = @{}
if (Test-Path $CONF) { foreach ($l in Get-Content $CONF) { if ($l -match '^(\w+)=(.*)$') { $OLD[$Matches[1]] = $Matches[2] } } }
$STAMP = if (Test-Path (Join-Path $ENGINE "build\Release\VERSION.txt")) { (Get-Content (Join-Path $ENGINE "build\Release\VERSION.txt") -First 1) } else { $ACESTEP_REV }
if ($OLD.MODE -and $OLD.LM_FILE -eq $LM_FILE -and $OLD.TESTED_REV -eq $STAMP) {
    $MODE = $OLD.MODE
    Write-Host "  schon getestet: Modus $MODE"
} else {
    $TEST_PORT = 8099
    if (Port-Laeuft $TEST_PORT) { Abbruch "Port $TEST_PORT ist belegt, bitte später erneut versuchen." }
    $args_ = @("--host", "127.0.0.1", "--port", $TEST_PORT, "--models", "`"$MODELS`"", "--max-batch", "1")
    if ($VAE_CHUNK) { $args_ += @("--vae-chunk", $VAE_CHUNK) }
    $proc = Start-Hintergrund $ACE $args_ (Join-Path $DATA "selftest.log")
    for ($i = 0; $i -lt 120; $i++) {
        try { Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$TEST_PORT/health" -TimeoutSec 2 | Out-Null; break } catch { Start-Sleep -Milliseconds 500 }
    }
    if ($proc.HasExited) {
        Get-Content (Join-Path $DATA "selftest.err.log") -Tail 15 -ErrorAction SilentlyContinue
        Abbruch "Der Modellserver startet nicht (siehe data\selftest.err.log). Fehlt eine DLL, hilft meist ein aktueller NVIDIA-Treiber."
    }
    Write-Host "  Sprachmodell probeweise auf der Grafikkarte (kann 1-2 Minuten dauern) ..."
    & uv run python scripts\metal_selftest.py $TEST_PORT $LM_FILE
    $MODE = if ($LASTEXITCODE -eq 0) { "single" } else { "split" }
    if ($proc.HasExited) {   # abgestürzt (z. B. Grafikspeicher voll): Ursache zeigen statt nur "Verbindung verweigert"
        Write-Host "  Der Modellserver ist beim Test abgestürzt (data\selftest.err.log):"
        Get-Content (Join-Path $DATA "selftest.err.log") -Tail 4 -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "    $_" }
    }
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Write-Host "  -> Modus $MODE"
}
@("MODE=$MODE", "LM_FILE=$LM_FILE", "TESTED_REV=$STAMP", "VAE_CHUNK=$VAE_CHUNK") | Set-Content $CONF

# --- 7. Verknüpfungen -----------------------------------------------------------
Schritt "Verknüpfungen (Programmordner und Desktop)"
$shell = New-Object -ComObject WScript.Shell
foreach ($e in @(@("Music Generator ON", "icon-on.ico"), @("Music Generator OFF", "icon-off.ico"))) {
    foreach ($dir in @($ROOT, [Environment]::GetFolderPath("Desktop"))) {
        $lnk = $shell.CreateShortcut((Join-Path $dir "$($e[0]).lnk"))
        $lnk.TargetPath = "powershell.exe"
        $lnk.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $ROOT "$($e[0]).ps1")`""
        $lnk.WorkingDirectory = $ROOT
        $lnk.IconLocation = (Join-Path $ROOT "assets\$($e[1])")
        $lnk.Save()
    }
}
Write-Host "  ok"

# --- Fertig ---------------------------------------------------------------------
Write-Host ""
Write-Host "==========================================="
Write-Host "  Fertig. Starten mit 'Music Generator ON',"
Write-Host "  beenden mit 'Music Generator OFF'."
Write-Host "==========================================="
if (-not $NoStart) {
    & (Join-Path $ROOT "Music Generator ON.ps1")
}
