#!/bin/bash

if [[ "$DISPLAY" == "" ]] && [[ "$1" != "--noauto" ]]; then
  exec xvfb-run --auto-servernum -- "$0" --noauto
fi

cd "$(dirname "$0")"

for name in $(find src -name '*.drawio')
do
  for theme in dark light
  do
    drawio -x $name -o ${name%.drawio}-$theme.svg --svg-theme $theme --embed-svg-fonts false -f svg &
  done
done

wait
