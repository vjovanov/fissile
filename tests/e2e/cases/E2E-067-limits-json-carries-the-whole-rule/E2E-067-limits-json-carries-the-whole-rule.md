# E2E-067-limits-json-carries-the-whole-rule: the machine surface omits what it cannot describe

The ticket's ask is a stable, machine-readable interface, so JSON carries what
the text line drops: `priority`, the message ids, and the line-counting policy
(§FS-010-limits.4). The top level is an object keyed `rules` rather than a bare
array, so the inventory can gain a sibling section without breaking a consumer.

Omission is the contract, not nulling. The byte rule declares no `soft`, so no
`soft` key; it is measured in bytes, so `count_blank_lines` and
`count_comment_lines` — which describe how a line is counted — are absent
entirely. The notes rule declares no hard limit, so it carries no
`hard_message`, even though the rule internally borrows the soft template to
stay valid: emitting it would show a caller guidance attached to a limit that
does not exist.
