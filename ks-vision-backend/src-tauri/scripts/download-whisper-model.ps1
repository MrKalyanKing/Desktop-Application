# Download bundled Whisper model if missing (dev / CI).
$ErrorActionPreference = "Stop"
$srcTauri = Split-Path -Parent $PSScriptRoot
$modelDir = Join-Path $srcTauri "resources\models"
$model = Join-Path $modelDir "ggml-tiny.en.bin"
New-Item -ItemType Directory -Force -Path $modelDir | Out-Null
if (Test-Path $model) {
  Write-Host "Model already present: $model"
  exit 0
}
Write-Host "Downloading ggml-tiny.en.bin (~75MB)..."
Invoke-WebRequest `
  -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin" `
  -OutFile $model `
  -UseBasicParsing
Write-Host "Saved: $model"
