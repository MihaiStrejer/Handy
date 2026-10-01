param(
    [ValidateSet('cpu', 'opencl')]
    [string]$Backend = 'opencl'
)

$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$logicalProcessors = [Environment]::ProcessorCount
$job = Start-Job -ScriptBlock {
    param($root, $backend)
    Set-Location -LiteralPath $root
    $env:EXPERIMENT_BACKEND = $backend
    uv run python experiments/run.py
} -ArgumentList $workspace, $Backend

$samples = [System.Collections.Generic.List[object]]::new()
$previousCpu = $null
$previousTime = $null
try {
    while ($job.State -eq 'Running') {
        $runtimeDirectory = if ($Backend -eq 'opencl') { 'runtime_opencl' } else { 'runtime' }
        $process = Get-Process -Name 'llama-server' -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "*\experiments\$runtimeDirectory\*" } | Select-Object -First 1
        if ($process) {
            $now = Get-Date
            $processCpu = $process.CPU
            $cpuPercent = $null
            if ($null -ne $previousCpu) {
                $seconds = ($now - $previousTime).TotalSeconds
                if ($seconds -gt 0) {
                    $cpuPercent = 100 * ($processCpu - $previousCpu) / ($seconds * $logicalProcessors)
                }
            }
            $previousCpu = $processCpu
            $previousTime = $now

            $gpuPercent = $null
            try {
                $gpuSamples = (Get-Counter '\GPU Engine(*)\Utilization Percentage' -ErrorAction Stop).CounterSamples
                $gpuPercent = ($gpuSamples | Where-Object { $_.Path -like "*pid_$($process.Id)_*" } | Measure-Object -Property CookedValue -Sum).Sum
            } catch {
                # GPU performance counters are optional on this driver.
            }

            $samples.Add([pscustomobject]@{
                Time = $now.ToString('o')
                Pid = $process.Id
                CpuPercentOfMachine = if ($null -eq $cpuPercent) { '' } else { [math]::Round($cpuPercent, 1) }
                GpuEnginePercentSum = if ($null -eq $gpuPercent) { '' } else { [math]::Round($gpuPercent, 1) }
                WorkingSetMiB = [math]::Round($process.WorkingSet64 / 1MB, 1)
            })
        }
        Start-Sleep -Milliseconds 500
    }
    Receive-Job -Job $job -Wait
} finally {
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    $output = Join-Path $PSScriptRoot "results\utilization-$Backend.csv"
    $samples | Export-Csv -LiteralPath $output -NoTypeInformation
    Write-Output "Saved $($samples.Count) samples to $output"
}
