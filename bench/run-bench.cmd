@echo off
REM Markit Phase A1 first-round benchmark (Windows host) - windowed for BOTH MVPs.
REM Usage: bench\run-bench.cmd [10k|100k|1m]  -- default 10k
setlocal EnableDelayedExpansion
set CORPUS=%1
if "%CORPUS%"=="" set CORPUS=10k
set ROOT=%~dp0..
set EXE_PJS=%ROOT%\mvp-pocketjs.exe
set EXE_GPUI=%ROOT%\gpui\target\release\mvp-gpui.exe
set DIST=%ROOT%\dist
set PARSER=%ROOT%\parse-trace.py
set OUT=%ROOT%\results
if not exist "%OUT%" mkdir "%OUT%"

REM Shared workload: 100 single-char inserts (one per frame), a backspace,
REM a scroll. Windowed for both (real window + present on the same GPU).
REM PocketJS: 500 ticks, auto-quit at 9s. GPUI: --smoke (106 frames,
REM typing x100 + IME + end + backspace x2 + enter + select-all + scroll).
set TYPING=--type "a"@340 --type "a"@341 --type "a"@342 --type "a"@343 --type "a"@344 --type "a"@345 --type "a"@346 --type "a"@347 --type "a"@348 --type "a"@349 --type "a"@350 --type "a"@351 --type "a"@352 --type "a"@353 --type "a"@354 --type "a"@355 --type "a"@356 --type "a"@357 --type "a"@358 --type "a"@359 --type "a"@360 --type "a"@361 --type "a"@362 --type "a"@363 --type "a"@364 --type "a"@365 --type "a"@366 --type "a"@367 --type "a"@368 --type "a"@369 --type "a"@370 --type "a"@371 --type "a"@372 --type "a"@373 --type "a"@374 --type "a"@375 --type "a"@376 --type "a"@377 --type "a"@378 --type "a"@379 --type "a"@380 --type "a"@381 --type "a"@382 --type "a"@383 --type "a"@384 --type "a"@385 --type "a"@386 --type "a"@387 --type "a"@388 --type "a"@389 --type "a"@390 --type "a"@391 --type "a"@392 --type "a"@393 --type "a"@394 --type "a"@395 --type "a"@396 --type "a"@397 --type "a"@398 --type "a"@399 --type "a"@400 --type "a"@401 --type "a"@402 --type "a"@403 --type "a"@404 --type "a"@405 --type "a"@406 --type "a"@407 --type "a"@408 --type "a"@409 --type "a"@410 --type "a"@411 --type "a"@412 --type "a"@413 --type "a"@414 --type "a"@415 --type "a"@416 --type "a"@417 --type "a"@418 --type "a"@419 --type "a"@420 --type "a"@421 --type "a"@422 --type "a"@423 --type "a"@424 --type "a"@425 --type "a"@426 --type "a"@427 --type "a"@428 --type "a"@429 --type "a"@430 --type "a"@431 --type "a"@432 --type "a"@433 --type "a"@434 --type "a"@435 --type "a"@436 --type "a"@437 --type "a"@438 --type "a"@439 --key "Backspace"@450 --scroll 56@460

set FILE=%ROOT%\%CORPUS%.txt
for /L %%R in (0,1,5) do (
  echo === %CORPUS% run %%R pocketjs ===
  "%EXE_PJS%" --js "%DIST%\markit-editor.js" --pak "%DIST%\markit-editor.pak" --file "%FILE%" --width 1000 --height 700 --auto-quit 9 !TYPING! > "%OUT%\pjs-%CORPUS%-%%R.log" 2>&1
  python "%PARSER%" < "%OUT%\pjs-%CORPUS%-%%R.log" >> "%OUT%\pjs-%CORPUS%-%%R.log"
  echo === %CORPUS% run %%R gpui ===
  "%EXE_GPUI%" --smoke --file "%FILE%" > "%OUT%\gpui-%CORPUS%-%%R.log" 2>&1
  python "%PARSER%" < "%OUT%\gpui-%CORPUS%-%%R.log" >> "%OUT%\gpui-%CORPUS%-%%R.log"
)
echo done. results in %OUT%
