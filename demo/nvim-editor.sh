#!/usr/bin/env bash
# Wrapper script for nvim editor with minimal config
exec nvim -u NONE --noplugin "$@"
