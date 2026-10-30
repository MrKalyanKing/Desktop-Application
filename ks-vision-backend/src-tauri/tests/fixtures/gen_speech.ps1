# Regenerates the spoken-question fixtures used by the listening tests
# (src/modules/audio/listening_tests.rs). Windows TTS, 16 kHz mono 16-bit.
#   powershell -ExecutionPolicy Bypass -File tests/fixtures/gen_speech.ps1

Add-Type -AssemblyName System.Speech
$out = Join-Path $PSScriptRoot "speech"
New-Item -ItemType Directory -Force $out | Out-Null

$fmt = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$voices = @($synth.GetInstalledVoices() | Where-Object { $_.Enabled } | ForEach-Object { $_.VoiceInfo.Name })

# name = text. "|" marks a pause: "|450|" = 450 ms break.
$clips = [ordered]@{
  "q_selenium_playwright" = "What is the difference between Selenium and Playwright?"
  "q_explicit_waits"      = "Can you explain how explicit waits work in Selenium WebDriver?"
  "q_about_yourself"      = "Tell me about yourself."
  "q_postman_api"         = "How do you test a REST API using Postman?"
  "q_page_object"         = "What is the Page Object Model, and why do we use it?"
  "q_flaky_tests"         = "How would you handle a flaky test in your regression suite?"
  "q_pause_short"         = "How do you run tests |450| in parallel with TestNG?"
  "q_pause_long"          = "What is the difference between |900| TestNG and JUnit?"
  "filler_hmm"            = "Hmm, okay."
}

$i = 0
foreach ($name in $clips.Keys) {
  $voice = $voices[$i % $voices.Count]
  $i++
  $synth.SelectVoice($voice)
  $text = [System.Security.SecurityElement]::Escape($clips[$name]) -replace '\|(\d+)\|', '<break time="$1ms"/>'
  $ssml = "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>$text</speak>"
  $path = Join-Path $out "$name.wav"
  $synth.SetOutputToWaveFile($path, $fmt)
  $synth.SpeakSsml($ssml)
  $synth.SetOutputToNull()
  Write-Host "$name ($voice)"
}
$synth.Dispose()
