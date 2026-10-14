param(
    [string]$CandidatePath,
    [string]$BaselinePath,
    [string]$MigrationPath,
    [string]$Repository = $env:GITHUB_REPOSITORY
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($CandidatePath) -or
    [string]::IsNullOrWhiteSpace($BaselinePath) -or
    [string]::IsNullOrWhiteSpace($MigrationPath)) {
    [Console]::Error.WriteLine("Usage: $($MyInvocation.MyCommand.Name) <candidate.json> <baseline.json|published|-> <MIGRATION.md> [-Repository owner/repo]")
    exit 2
}

function Fail([string]$Message) {
    [Console]::Error.WriteLine("MCP contract compatibility error: $Message")
    exit 1
}

function Read-Contract([string]$Path, [string]$Json) {
    try {
        if ([string]::IsNullOrWhiteSpace($Json)) {
            $Json = Get-Content -Raw -LiteralPath $Path
        }
        $Contract = $Json | ConvertFrom-Json -AsHashtable
    }
    catch {
        Fail "cannot read contract $Path`: $($_.Exception.Message)"
    }

    if ($Contract -isnot [System.Collections.IDictionary]) {
        Fail "contract $Path must contain a JSON object"
    }
    if ($Contract['contract_version'] -isnot [string] -or
        $Contract['contract_version'] -cnotmatch '^\d+\.\d+\.\d+$') {
        Fail "contract $Path must declare a semantic contract_version"
    }

    $Tool = $Contract['tool']
    if ($Tool -isnot [System.Collections.IDictionary]) {
        Fail "contract $Path must define a tool"
    }
    if ($Tool['arguments'] -isnot [System.Collections.IDictionary]) {
        Fail "contract $Path must define tool.arguments"
    }
    if ($Tool['result'] -isnot [System.Collections.IDictionary]) {
        Fail "contract $Path must define tool.result"
    }

    $Result = $Tool['result']
    if ($Result['fields'] -isnot [System.Collections.IDictionary] -or
        $Result['outcomes'] -isnot [System.Collections.IDictionary]) {
        Fail "contract $Path must define result fields and outcomes"
    }
    return $Contract
}

function Test-JsonEqual([object]$Left, [object]$Right) {
    $LeftJson = ConvertTo-Json -InputObject $Left -Depth 100 -Compress
    $RightJson = ConvertTo-Json -InputObject $Right -Depth 100 -Compress
    return $LeftJson -ceq $RightJson
}

$Candidate = Read-Contract $CandidatePath

if ($BaselinePath -ceq 'published' -or $BaselinePath -ceq '-') {
    try {
        if ([string]::IsNullOrWhiteSpace($Repository)) {
            $Repository = gh repo view --json nameWithOwner --jq .nameWithOwner
            if ($LASTEXITCODE -ne 0) {
                throw 'cannot determine GitHub repository'
            }
        }
        if ($Repository -cnotmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') {
            throw 'repository must be owner/repo'
        }
        $ReleaseJson = gh api --paginate --slurp "repos/$Repository/releases?per_page=100"
        if ($LASTEXITCODE -ne 0) {
            throw 'cannot list published GitHub releases'
        }
        $ReleasePages = ($ReleaseJson -join "`n") | ConvertFrom-Json -AsHashtable -NoEnumerate
        if ($ReleasePages -isnot [array] -or $ReleasePages.Count -eq 0) {
            throw 'release listing must contain paginated JSON arrays'
        }
        $Published = @(
            foreach ($Page in $ReleasePages) {
                if ($Page -isnot [array]) {
                    throw 'each release page must be a JSON array'
                }
                foreach ($Release in $Page) {
                    if ($Release -isnot [System.Collections.IDictionary] -or $Release['assets'] -isnot [array]) {
                        throw 'each release must contain an assets array'
                    }
                    if ($Release['draft'] -or $Release['prerelease']) {
                        continue
                    }
                    $Assets = @($Release['assets'] | Where-Object {
                            $_['name'] -ceq 'mcp-contract.json'
                        })
                    if ($Assets.Count -eq 0) {
                        continue
                    }
                    if ($Release['tag_name'] -cnotmatch '^v\d+\.\d+\.\d+$' -or $Assets.Count -ne 1) {
                        throw 'published MCP contract must have one asset and a vX.Y.Z release tag'
                    }
                    @{
                        Version = [version]$Release['tag_name'].Substring(1)
                        AssetId = $Assets[0]['id']
                        Tag     = $Release['tag_name']
                    }
                }
            }
        )
        if ($Published.Count -eq 0) {
            Write-Output "No published MCP contract baseline; $($Candidate['contract_version']) establishes it."
            exit 0
        }
        if ($BaselinePath -ceq '-') {
            throw "'-' is only allowed when no published MCP contract release exists; use 'published'"
        }
        $SameMajor = @($Published | Where-Object {
                $_.Version.Major -eq ([version]$Candidate['contract_version']).Major
            })
        $Choices = if ($SameMajor.Count -gt 0) { $SameMajor } else { $Published }
        $Selected = $Choices | Sort-Object -Property Version -Descending | Select-Object -First 1
        $BaselineJson = gh api -H 'Accept: application/octet-stream' "repos/$Repository/releases/assets/$($Selected.AssetId)"
        if ($LASTEXITCODE -ne 0) {
            throw "cannot download published MCP contract for $($Selected.Tag)"
        }
        $Baseline = Read-Contract $Selected.Tag ($BaselineJson -join "`n")
        if ([version]$Baseline['contract_version'] -ne $Selected.Version) {
            throw 'published MCP contract version does not match its release tag'
        }
        Write-Output "Selected published MCP contract $($Selected.Tag)."
    }
    catch {
        Fail "published baseline discovery failed`: $($_.Exception.Message)"
    }
}
else {
    $Baseline = Read-Contract $BaselinePath
}

$CandidateVersion = [version]$Candidate['contract_version']
$BaselineVersion = [version]$Baseline['contract_version']
$CandidateTool = $Candidate['tool']
$BaselineTool = $Baseline['tool']
$Breaking = [System.Collections.Generic.List[string]]::new()

$OldArguments = $BaselineTool['arguments']
$NewArguments = $CandidateTool['arguments']
foreach ($Name in $OldArguments.Keys) {
    if (-not $NewArguments.Contains($Name)) {
        $Breaking.Add("removed argument '$Name'")
        continue
    }

    $OldArgument = $OldArguments[$Name]
    $NewArgument = $NewArguments[$Name]
    foreach ($Key in @('type', 'required', 'meaning')) {
        if (-not (Test-JsonEqual $OldArgument[$Key] $NewArgument[$Key])) {
            $Breaking.Add("changed argument '$Name' $Key")
        }
    }
}
foreach ($Name in $NewArguments.Keys) {
    if (-not $OldArguments.Contains($Name) -and $NewArguments[$Name]['required'] -eq $true) {
        $Breaking.Add("added required argument '$Name'")
    }
}

$OldFields = $BaselineTool['result']['fields']
$NewFields = $CandidateTool['result']['fields']
foreach ($Name in $OldFields.Keys) {
    if (-not $NewFields.Contains($Name)) {
        $Breaking.Add("removed result field '$Name'")
        continue
    }

    $OldField = $OldFields[$Name]
    $NewField = $NewFields[$Name]
    foreach ($Key in @('type', 'required', 'meaning')) {
        if (-not (Test-JsonEqual $OldField[$Key] $NewField[$Key])) {
            $Breaking.Add("changed result field '$Name' $Key")
        }
    }
}
foreach ($Name in $NewFields.Keys) {
    if (-not $OldFields.Contains($Name) -and $NewFields[$Name]['required'] -eq $true) {
        $Breaking.Add("added required result field '$Name'")
    }
}

$OldOutcomes = $BaselineTool['result']['outcomes']
$NewOutcomes = $CandidateTool['result']['outcomes']
foreach ($Name in $OldOutcomes.Keys) {
    if (-not $NewOutcomes.Contains($Name) -or
        -not (Test-JsonEqual $OldOutcomes[$Name] $NewOutcomes[$Name])) {
        $Breaking.Add("removed or changed outcome '$Name'")
    }
}
foreach ($Name in $NewOutcomes.Keys) {
    if (-not $OldOutcomes.Contains($Name)) {
        $Breaking.Add("added outcome '$Name'")
    }
}

if ($CandidateVersion.Major -lt $BaselineVersion.Major) {
    Fail 'candidate contract version is older than the published baseline'
}

if ($Breaking.Count -gt 0 -and $CandidateVersion.Major -le $BaselineVersion.Major) {
    Fail "breaking changes require a new major version`: $($Breaking -join '; ')"
}

if ($Breaking.Count -gt 0) {
    try {
        $MigrationNotes = Get-Content -Raw -LiteralPath $MigrationPath
    }
    catch {
        Fail "cannot read migration notes`: $($_.Exception.Message)"
    }

    $OldMajor = $BaselineVersion.Major
    $NewVersion = $Candidate['contract_version']
    $HeadingPattern = "(?m)^##\s+$OldMajor\.x\s+to\s+$([regex]::Escape($NewVersion))\s*$"
    $HasMigrationHeading = [regex]::IsMatch([string]$MigrationNotes, $HeadingPattern)
    if (-not $HasMigrationHeading) {
        Fail "breaking changes from $OldMajor.x to $NewVersion require a matching migration heading in MIGRATION.md"
    }
}

Write-Output "MCP contract $($Candidate['contract_version']) is compatible with $($Baseline['contract_version'])."
