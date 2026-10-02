$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Add-Type -AssemblyName System.Speech
$request = [Console]::In.ReadToEnd() | ConvertFrom-Json
$speech = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    if ($request.action -eq 'list') {
        $voices = @($speech.GetInstalledVoices() | Where-Object { $_.Enabled } | ForEach-Object {
            @{ name = $_.VoiceInfo.Name; language = $_.VoiceInfo.Culture.Name }
        })
        ConvertTo-Json -InputObject $voices -Compress
    } elseif ($request.action -eq 'speak') {
        $speech.SelectVoice($request.voice)
        # System.Speech exposes relative rate [-10,10], not words per minute.
        $speech.Rate = [Math]::Max(-10, [Math]::Min(10, [int][Math]::Round(10 * [Math]::Log($request.rate / 180.0, 3))))
        $speech.SetOutputToWaveFile($request.output)
        $speech.Speak($request.text)
    } else { throw 'Unknown voice action' }
} finally { $speech.Dispose() }
