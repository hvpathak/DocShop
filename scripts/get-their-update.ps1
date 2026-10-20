<#
"Get their update": bring the latest WordCraft (and, once it is in, PdfCraft) into HarshPad.

  1. Merges the upstream main branch into the `harshpad` branch of our fork, which carries
     HarshPad's small embedding patch (Help -> our GitHub, no community/Discord links, no
     account button, a light/dark switch).
  2. Guards the merge: if upstream added new community, Discord, account or upstream-website
     links to the UI, it stops and lists them, so they are hidden before anything ships.
  3. Pushes the fork and moves HarshPad's pinned revision to it.
Then build and test HarshPad (cargo test --workspace) and release as usual.

Usage:  pwsh scripts/get-their-update.ps1 [-Project wordcraft] [-Workdir D:\Reeverse\upstream]
#>
param(
    [ValidateSet('wordcraft', 'pdfcraft')]
    [string]$Project = 'wordcraft',
    [string]$Workdir = (Join-Path (Split-Path $PSScriptRoot -Parent) '..\upstream')
)
$ErrorActionPreference = 'Stop'
$repo = Join-Path $Workdir $Project
$harshpad = Split-Path $PSScriptRoot -Parent
$guardFile = Join-Path $PSScriptRoot "upstream-guard-$Project.json"

# Patterns that must not grow: each is a link or button HarshPad hides or redirects.
$patterns = [ordered]@{
    'discord'           = '(?i)discord'
    'account initials'  = 'user_initials'
    'upstream website'  = '(?i)getartcraft\.com|storyteller\.ai'
    'upstream source'   = '(?i)github\.com/storytold'
    'community wording' = '(?i)artcraft (community|team)'
}

function Count-Patterns([string]$dir) {
    $files = Get-ChildItem $dir -Recurse -Include *.rs | Where-Object { $_.FullName -notmatch '\\(tests?|i18n)\\' }
    $result = [ordered]@{}
    foreach ($name in $patterns.Keys) {
        $hits = $files | Select-String -Pattern $patterns[$name]
        $result[$name] = @($hits | ForEach-Object { '{0}:{1}: {2}' -f $_.Path.Substring($dir.Length + 1), $_.LineNumber, $_.Line.Trim() })
    }
    $result
}

if (-not (Test-Path $repo)) {
    git clone "https://github.com/hvpathak/$Project" $repo
}
Push-Location $repo
try {
    if (-not (git remote | Select-String -SimpleMatch 'upstream')) {
        git remote add upstream "https://github.com/storytold/$Project"
    }
    git fetch upstream main
    git fetch origin harshpad 2>$null
    git checkout harshpad
    $before = (git rev-parse upstream/main)
    git merge --no-edit upstream/main
    if ($LASTEXITCODE -ne 0) { throw "Merge conflict: resolve it in $repo, then run this script again." }

    $ui = Join-Path $repo 'crates\ui-egui\src'
    $now = Count-Patterns $ui
    if (Test-Path $guardFile) {
        $known = Get-Content $guardFile -Raw | ConvertFrom-Json -AsHashtable
        $new = @()
        foreach ($name in $now.Keys) {
            $old = @($known[$name])
            $added = @($now[$name] | Where-Object { ($_ -replace '^([^:]+):\d+:', '$1:') -notin @($old | ForEach-Object { $_ -replace '^([^:]+):\d+:', '$1:' }) })
            if ($added.Count) { $new += "== $name"; $new += $added }
        }
        if ($new.Count) {
            Write-Host "STOP: the update added links or buttons HarshPad hides. Gate them behind" -ForegroundColor Red
            Write-Host "Services::hide_community / theme_switch in the harshpad branch, then run again:" -ForegroundColor Red
            $new | ForEach-Object { Write-Host "  $_" }
            exit 1
        }
    }
    $now | ConvertTo-Json -Depth 3 | Set-Content $guardFile -Encoding utf8NoBOM

    git push origin harshpad
    $rev = (git rev-parse HEAD)
}
finally {
    Pop-Location
}

# Point HarshPad at the merged fork.
$toml = Join-Path $harshpad 'Cargo.toml'
$text = Get-Content $toml -Raw
$text = [regex]::Replace($text, "(git = `"https://github.com/hvpathak/$Project`", rev = `")[0-9a-f]{40}`"", "`${1}$rev`"")
Set-Content $toml $text -NoNewline -Encoding utf8NoBOM
Write-Host "HarshPad now uses $Project $rev (upstream $before). Next: cargo test --workspace, then release." -ForegroundColor Green
