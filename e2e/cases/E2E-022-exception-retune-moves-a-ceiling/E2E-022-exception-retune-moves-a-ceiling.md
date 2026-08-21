# E2E-022-exception-retune-moves-a-ceiling: retune moves a recorded ceiling to the quantized value

The file has outgrown the ceiling its entry records, but the entry's reason still
holds — only the number is wrong. `fissile exception retune` moves that number
without asking for a new rationale, and the value is the measurement rounded up
to the configured step rather than the measurement itself
(§FS-008-exception-retune.1, §DF-006-quantized-ceilings.1).

The registry's comment and the entry's other fields survive the rewrite: the
diff is the one line that changed (§FS-008-exception-retune.3).
