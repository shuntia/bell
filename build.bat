@echo off
setlocal EnableDelayedExpansion

where git >nul 2>&1
if errorlevel 1 (
  echo Missing dependency: git
  exit /b 1
)
where cargo >nul 2>&1
if errorlevel 1 (
  echo Missing dependency: cargo
  exit /b 1
)

set "REPO_URL=%SCHEDULE_REPO_URL%"
if "%REPO_URL%"=="" set "REPO_URL=https://github.com/nicolaschan/bell-schedules.git"
set "SCHEDULE_DIR=%SCHEDULE_DIR%"
if "%SCHEDULE_DIR%"=="" set "SCHEDULE_DIR=schedules"

if exist "%SCHEDULE_DIR%\.git" (
  pushd "%SCHEDULE_DIR%"
  git pull --ff-only
  popd
) else (
  if exist "%SCHEDULE_DIR%" (
    dir /b "%SCHEDULE_DIR%" >nul 2>&1
    if %errorlevel%==0 (
      echo Directory "%SCHEDULE_DIR%" exists but is not a git repo.
      exit /b 1
    )
  )
  git clone "%REPO_URL%" "%SCHEDULE_DIR%"
)

set /a count=0
for /d %%D in ("%SCHEDULE_DIR%\*") do (
  if exist "%%D\schedules.bell" (
    set /a count+=1
    set "opt!count!=%%~nxD"
    echo !count!) %%~nxD
  )
)

if %count%==0 (
  echo No schedules found in "%SCHEDULE_DIR%".
  exit /b 1
)

set /p choice=Select schedule (name or number): 
set "selected="
for /f "delims=0123456789" %%A in ("%choice%") do set "isnum=%%A"
if "%isnum%"=="" (
  for /f %%A in ("!opt%choice%!") do set "selected=%%A"
) else (
  set "selected=%choice%"
)

if "%selected%"=="" (
  echo Unknown schedule "%choice%".
  exit /b 1
)

if not exist "%SCHEDULE_DIR%\%selected%" (
  echo Unknown schedule "%choice%".
  exit /b 1
)

set "SELECTED_SCHEDULE=%selected%"
set "SCHEDULE_DIR=%SCHEDULE_DIR%"
cargo build --release
copy /y "target\release\bell.exe" "bell.exe" >nul
