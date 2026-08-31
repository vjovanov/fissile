#!/bin/sh
# Minimal external token counter for the e2e token-rule case: print the word
# count of the file, which fissile reads as the token total. The real opt-in
# command is a tokenizer (§DA-001-token-external-command); word count is a
# portable stand-in. Reading from stdin keeps the filename out of the output so
# the checker parses a bare integer.
exec wc -w < "$1"
