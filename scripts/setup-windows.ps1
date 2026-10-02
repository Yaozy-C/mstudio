# Install pinned build-only SDKs. End users receive private runtimes in the app.
param([string]$DependencyRoot = "$PSScriptRoot/../desktop/native/vendor/windows")
$ErrorActionPreference = 'Stop'
$DependencyRoot = [IO.Path]::GetFullPath($DependencyRoot)
New-Item -ItemType Directory -Force $DependencyRoot | Out-Null
function Download-Verified($Url, $Destination, $Sha256) {
    if (!(Test-Path $Destination) -or (Get-FileHash $Destination -Algorithm SHA256).Hash -ne $Sha256) {
        Invoke-WebRequest $Url -OutFile $Destination
    }
    if ((Get-FileHash $Destination -Algorithm SHA256).Hash -ne $Sha256) { throw "Checksum mismatch: $Destination" }
}
$installer = Join-Path $DependencyRoot 'gstreamer-1.28.7.exe'
Download-Verified 'https://gstreamer.freedesktop.org/data/pkg/windows/1.28.7/msvc/gstreamer-1.0-msvc-x86_64-1.28.7.exe' $installer '032fc6062b8539838fc8da22589cb9b24c5d820baa7f8cc160af9ea08395badf'
$gst = Join-Path $DependencyRoot 'gstreamer'
if (!(Test-Path "$gst/lib/pkgconfig/gst-editing-services-1.0.pc")) {
    $process = Start-Process $installer -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', '/TYPE=devel', "/DIR=`"$gst`"") -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "GStreamer installation failed: $($process.ExitCode)" }
}
$archive = Join-Path $DependencyRoot 'ffmpeg-9.0.2.zip'
Download-Verified 'https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.zip' $archive '60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba'
$ffmpeg = Join-Path $DependencyRoot 'ffmpeg-9.0.2-essentials_build'
if (!(Test-Path "$ffmpeg/bin/ffmpeg.exe")) { Expand-Archive $archive -DestinationPath $DependencyRoot -Force }
$env:GSTREAMER_1_0_ROOT_MSVC_X86_64 = $gst
$env:MSTUDIO_FFMPEG_DIR = $ffmpeg
$env:PKG_CONFIG_PATH = "$gst/lib/pkgconfig"
$env:PATH = "$gst/bin;$ffmpeg/bin;$env:PATH"
if ($env:GITHUB_ENV) {
    @("GSTREAMER_1_0_ROOT_MSVC_X86_64=$gst", "MSTUDIO_FFMPEG_DIR=$ffmpeg", "PKG_CONFIG_PATH=$gst/lib/pkgconfig") | Out-File $env:GITHUB_ENV -Append -Encoding utf8
    @("$gst/bin", "$ffmpeg/bin") | Out-File $env:GITHUB_PATH -Append -Encoding utf8
}
& pkg-config --modversion gst-editing-services-1.0
if ($LASTEXITCODE -ne 0) { throw 'GES SDK is unavailable' }
