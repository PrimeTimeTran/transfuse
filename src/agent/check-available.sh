#!/usr/bin/env bash

commands=(
  # FILESYSTEM
  pwd
  ls
  tree
  find
  fd
  rg
  grep
  cat
  head
  tail
  less
  wc
  sort
  uniq
  cut
  tr
  sed
  awk
  xargs
  file
  stat
  realpath
  du
  df
  diff
  cmp
  patch
  tee
  mkdir
  touch
  cp
  mv
  rm
  ln
  chmod
  chown
  tar
  zip
  unzip

  # SHELL / SYSTEM
  sh
  bash
  zsh
  printf
  test
  env
  printenv
  which
  type
  command
  date
  uname
  hostname
  whoami
  id
  ps
  kill
  sleep
  time
  timeout

  # SEARCH / DATA
  jq
  yq
  xxd
  base64

  # NETWORK
  curl
  wget
  ssh
  scp
  rsync
  ping
  nc
  dig

  # VERSION CONTROL
  git

  # RUST
  cargo
  rustc
  rustup
  rustfmt
  clippy

  # JAVASCRIPT / WEB
  node
  npm
  npx
  pnpm
  yarn
  bun
  deno
  vite
  tsc
  eslint
  prettier

  # BUILD / COMPILERS
  make
  cmake
  ninja
  gcc
  clang
  clang++
  swift
  swiftc

  # MACOS
  open
  defaults
  plutil
  osascript
  launchctl
  codesign
  xcrun
  otool
  lsof
  system_profiler
  pbcopy
  pbpaste
)

available=()
missing=()

for cmd in "${commands[@]}"; do
  if command -v "$cmd" >/dev/null 2>&1; then
    available+=("$cmd")
  else
    missing+=("$cmd")
  fi
done

echo "========================================"
echo " AVAILABLE"
echo "========================================"

for cmd in "${available[@]}"; do
  printf '✓ %s' "$cmd"

  path="$(command -v "$cmd")"
  printf ' -> %s\n' "$path"
done

echo
echo "========================================"
echo " MISSING"
echo "========================================"

if ((${#missing[@]} == 0)); then
  echo "None 🎉"
else
  for cmd in "${missing[@]}"; do
    printf '✗ %s\n' "$cmd"
  done
fi

echo
echo "========================================"
printf 'Available: %d\n' "${#available[@]}"
printf 'Missing:   %d\n' "${#missing[@]}"
printf 'Total:     %d\n' "${#commands[@]}"
echo "========================================"