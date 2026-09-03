# E2E-075-a-ceiling-on-the-hard-limit-is-not-named: a finding withholds a ceiling the command would refuse

`fissile exception add --severity soft` refuses a ceiling at or above the rule's
hard limit for a file still under it, because the hard finding fires there and
the soft entry would never match (§DF-010-stated-ceilings-are-exact.2). A soft
finding whose step lands on that limit therefore names no ceiling at all rather
than a number the command would decline (§FS-004-check-audit.1).

One run shows both halves of the rule. Under a 10-line step against a 35/55-line
rule, the 41-line file rounds to 50 and is named; the 51-line file rounds to 60,
which is the refusal, and its detail line ends at the budget.
