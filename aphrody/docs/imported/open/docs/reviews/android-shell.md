# Import review: apps/android-shell

Moved from the Aphrody monorepo (3a4463101), checked against decision D25.

| Check | Finding | Outcome |
|---|---|---|
| Purification | no host, path, account or identity in the build files | private-data gate clean |
| Naming (D23) | `applicationId` and `namespace` `dev.aphrody.shell` | kept: an Android application id is a store coordinate (reverse domain of the publisher), as rule 2 treats registry coordinates |
