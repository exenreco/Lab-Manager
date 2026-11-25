param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$version = '0.1.0-alpha.1'
$signTool = (Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' | Sort-Object FullName -Descending | Select-Object -First 1).FullName
$iscc = @('C:\Program Files (x86)\Inno Setup 6\ISCC.exe', "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (!$signTool -or !$iscc) { throw 'Windows SDK signing tools and Inno Setup 6 are required.' }
New-Item -ItemType Directory -Force dist,release-private | Out-Null
$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Where-Object { $_.Subject -eq 'CN=Lab Manager Prerelease, O=Exenreco Bell' -and $_.NotAfter -gt (Get-Date).AddDays(30) } | Select-Object -First 1
if (!$cert) {
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=Lab Manager Prerelease, O=Exenreco Bell' -CertStoreLocation Cert:\CurrentUser\My -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 -KeyExportPolicy NonExportable -NotAfter (Get-Date).AddYears(2)
}
Export-Certificate -Cert $cert -FilePath dist\Lab-Manager-Prerelease.cer | Out-Null
$cert.Thumbprint | Set-Content release-private\signer-thumbprint.txt
if (!$SkipBuild) {
    $env:CARGO_INCREMENTAL = '0'
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    cargo build --release --locked
    if ($LASTEXITCODE) { throw 'Release build failed.' }
}
python scripts\third_party.py
if ($LASTEXITCODE) { throw 'Dependency notices failed.' }
& $signTool sign /sha1 $cert.Thumbprint /s My /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 target\release\lab-manager.exe
if ($LASTEXITCODE) { throw 'App signing failed.' }
$signCommand = '$q' + $signTool + '$q sign /sha1 ' + $cert.Thumbprint + ' /s My /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $f'
& $iscc "/DAppVersion=$version" "/Srelease=$signCommand" packaging\windows.iss
if ($LASTEXITCODE) { throw 'Installer build/signing failed.' }
Compress-Archive -Force -Path target\release\lab-manager.exe,LICENSE,NOTICE,packaging\TERMS.txt,dist\THIRD-PARTY-NOTICES.txt,dist\Lab-Manager-Prerelease.cer -DestinationPath "dist\Lab-Manager-$version-windows-x64-portable.zip"
Get-ChildItem dist -File | Where-Object Name -ne 'SHA256SUMS.txt' | Sort-Object Name | ForEach-Object { '{0}  {1}' -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name } | Set-Content dist\SHA256SUMS.txt -Encoding ascii
Write-Host 'Signed installer and portable package ready in dist.'
