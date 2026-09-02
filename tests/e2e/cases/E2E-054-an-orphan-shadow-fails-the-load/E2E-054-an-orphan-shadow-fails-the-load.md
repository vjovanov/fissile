# E2E-054-an-orphan-shadow-fails-the-load: deleting the original takes the twin with it

The twin's promise is that it retires with the entry it shadows
(§FS-003-exceptions.2.3). A convention cannot keep that promise — the two
entries live in different files and either can be deleted alone — so the load
enforces it: a `shadows = "hard"` entry that resolves to no hard entry is a
schema error (§FS-003-exceptions.4).

Here the hard registry is still on disk and the entry it held is gone, which is
what a repository looks like the moment someone retires the hard acceptance and
forgets the twin. `check` exits 2 and the message leads with the twin's own
site, names the registry the missing entry belongs in, and offers the other
edit: drop the pointer and state the entry's own reason and until.
