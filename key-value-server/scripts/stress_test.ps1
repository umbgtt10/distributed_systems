#!/usr/bin/env pwsh

$ErrorActionPreference = "Stop"

Write-Host "Running stress tests for all KV server implementations..." -ForegroundColor Cyan
Write-Host ""

Write-Host "Running in-memory server test..." -ForegroundColor Yellow
& "..\server-in-memory\scripts\run_test.ps1"
Write-Host ""

Write-Host "Running flat-file server test..." -ForegroundColor Yellow
& "..\server-flat-file\scripts\run_test.ps1"
Write-Host ""

Write-Host "Running Sled DB server test..." -ForegroundColor Yellow
& "..\server-sled-db\scripts\run_test.ps1"
Write-Host ""

Write-Host "All stress tests completed!" -ForegroundColor Green