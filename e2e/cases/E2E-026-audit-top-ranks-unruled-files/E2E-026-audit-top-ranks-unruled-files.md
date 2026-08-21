# E2E-026-audit-top-ranks-unruled-files: the largest file ranks even when no rule reaches it

The only rule in this repository measures `src/**/*.rs`. The largest file by a
wide margin is a `.txt` file no rule selects, and it is the first thing
`audit --top` has to say: the command is the adoption surface, and a repository
whose rules do not yet reach its largest file is the repository that most needs
to be told about it (§FS-004-check-audit.2).

Where a rule does measure the file, the ranked value is the one that rule counts,
so a `--top` number and a finding never disagree; everything else is counted
under the default line policy (§FS-001-config.3.1).
