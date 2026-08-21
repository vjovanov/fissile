#!/bin/sh
# The e2e token counter, with a ledger: every invocation appends one line to
# `counted.log` before printing the count, so a scenario can assert how many
# times fissile reached for it (§DA-001-token-external-command).
echo "$1" >> counted.log
exec wc -w < "$1"
