@echo off
set PATH=%PATH%;C:\Program Files\CMake\bin;%USERPROFILE%\.cargo\bin
cargo build --release -p goose-cli
