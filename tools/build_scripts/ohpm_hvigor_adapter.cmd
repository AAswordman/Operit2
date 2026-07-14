@ECHO OFF
SETLOCAL ENABLEDELAYEDEXPANSION

IF "%OHPM_REAL_BIN%"=="" (
  ECHO OHPM_REAL_BIN is not set.
  EXIT /B 1
)

SET "ARGS="

:collect_args
IF "%~1"=="" GOTO run_ohpm
IF "%~1"=="--all" (
  SHIFT
  GOTO collect_args
)
IF "%~1"=="--target_path" (
  SHIFT
  SHIFT
  GOTO collect_args
)
SET "ARGS=!ARGS! "%~1""
SHIFT
GOTO collect_args

:run_ohpm
CALL "%OHPM_REAL_BIN%" %ARGS%
EXIT /B %ERRORLEVEL%
