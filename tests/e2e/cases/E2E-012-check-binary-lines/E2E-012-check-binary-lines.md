# E2E-012-check-binary-lines: non-UTF-8 content still measures lines

A source file with a non-UTF-8 encoding is not an error: line budgets measure
physical lines from raw bytes, every line counting as content
(§FS-001-config.3.1). The fixture is 600 ISO-8859 lines under a 550-line hard
limit, so the gate blocks on size exactly as it would for UTF-8 — a stray
encoding changes nothing about the contract (§FS-004-check-audit.5).
