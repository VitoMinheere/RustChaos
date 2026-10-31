#!/usr/bin/env bash
# Runs the texture upscale unattended, as the systemd user unit texture-upscale:
#  - it survives the terminal, SSH or desktop session ending (a user unit, with lingering on
#    so systemd keeps user units running after logout);
#  - the laptop doesn't sleep, idle-suspend or suspend on lid close while it runs
#    (systemd-inhibit --what=sleep:idle:handle-lid-switch).
# Finished textures are skipped, so after a stop or a crash just run this again.
#
#   tools/upscale/run_unattended.sh            start (or continue)
#   tail -f ~/uc/upscale.log                   progress
#   systemctl --user status texture-upscale    state
#   systemctl --user stop texture-upscale      stop
set -euo pipefail

SRC="$HOME/uc/textures"                  # from extract_textures.py
DST="$HOME/uc/textures_4x"
LOG="$HOME/uc/upscale.log"
PY="$HOME/.venvs/upscale/bin/python"     # Pillow + numpy
GPU="0"                                  # realesrgan's index of the T1000
MODEL="realesrgan-x4plus"
UNIT="texture-upscale"

HERE="$(cd "$(dirname "$0")" && pwd)"

if systemctl --user is-active --quiet "$UNIT"; then
  echo "$UNIT is already running: tail -f $LOG"
  exit 1
fi
systemctl --user reset-failed "$UNIT" 2>/dev/null || true

if [[ "$(loginctl show-user "$USER" -p Linger --value)" != "yes" ]]; then
  echo "Turning on lingering so the job outlives the session (undo: loginctl disable-linger)"
  loginctl enable-linger "$USER"
fi

systemd-run --user --unit="$UNIT" --collect \
  --description="Upscale Urban Chaos textures" \
  --property=Nice=5 \
  --setenv=PYTHONUNBUFFERED=1 \
  systemd-inhibit --what=sleep:idle:handle-lid-switch --who="$UNIT" \
    --why="Upscaling textures" --mode=block \
  "$PY" "$HERE/upscale_textures.py" \
    --src "$SRC" --dst "$DST" --log "$LOG" \
    --gpu "$GPU" --model "$MODEL" --tile 256 --chunk 50 --fill edge \
    --no-wrap 'Textures/*' --no-wrap 'data/*' --no-wrap 'server/*'

echo "Started $UNIT. Progress: tail -f $LOG"
