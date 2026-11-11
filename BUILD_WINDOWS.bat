@echo off
setlocal EnableExtensions
cd /d "%~dp0"
title Build Valtic Launcher
color 0C
echo.
echo ====================================================
echo                  VALTIC LAUNCHER
 eCHO                     WINDOWS BUILDER
 eCHO ====================================================
echo.
where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust is not installed, or cargo is not in PATH.
    echo.
    echo Install Rust: https://rustup.rs/
    echo Install Visual Studio Build Tools with Desktop development with C++:
    echo https://visualstudio.microsoft.com/downloads/
    echo.
    echo Restart your computer or reopen this script after installing them.
    pause
    exit /b 1
)
if defined VALTIC_MSA_CLIENT_ID (
    set "HORIZON_MSA_CLIENT_ID=%VALTIC_MSA_CLIENT_ID%"
) else (
    echo Microsoft sign-in ID not set. The window can build and open,
    echo but you will need to configure HORIZON_MSA_CLIENT_ID for login.
    echo See MICROSOFT_SIGN_IN.md.
    echo.
)
set "RUSTFLAGS=-C target-feature=+crt-static"
pushd launcher
 echo Compiling the real Horizon-based Valtic Launcher...
 cargo build --release --locked --bin valtic-launcher
 if errorlevel 1 (
    popd
    echo.
    echo BUILD FAILED - copy the error shown above to ChatGPT for help.
    pause
    exit /b 1
 )
popd
if not exist BUILT_APP mkdir BUILT_APP
copy /Y "launcher\target\release\valtic-launcher.exe" "BUILT_APP\Valtic Launcher.exe" >nul
copy /Y "launcher\target\release\valtic-launcher.exe" "BUILT_APP\valtic-launcher-setup.exe" >nul
copy /Y "LICENSE" "BUILT_APP\LICENSE.txt" >nul
copy /Y "THIRD_PARTY_NOTICES.md" "BUILT_APP\THIRD_PARTY_NOTICES.md" >nul
echo.
echo ====================================================
echo SUCCESS! Your files are in the BUILT_APP folder.
echo Open Valtic Launcher.exe to launch the app.
echo ====================================================
explorer "%~dp0BUILT_APP"
pause
