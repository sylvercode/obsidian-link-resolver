param(
    [string]$CandidatePath,
    [string]$BaselinePath,
    [string]$MigrationPath
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($CandidatePath) -or
    [string]::IsNullOrWhiteSpace($BaselinePath) -or
    [string]::IsNullOrWhiteSpace($MigrationPath)) {
    [Console]::Error.WriteLine("Usage: $($MyInvocation.MyCommand.Name) <candidate.json> <baseline.json|-> <MIGRATION.md>")
    exit 2
}

function Fail([string]$Message) {
    [Console]::Error.WriteLine("MCP contract compatibility error: $Message")
    exit 1
}

function Read-Contract([string]$Path) {
    try {
        $Contract = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json -AsHashtable
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

if ($BaselinePath -ceq '-') {
    Write-Output "No published MCP contract baseline; $($Candidate['contract_version']) establishes it."
    exit 0
}

$Baseline = Read-Contract $BaselinePath
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
