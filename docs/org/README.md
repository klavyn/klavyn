# Org migration — staging area

These files are drafts for the `klavyn` GitHub organization, staged here
until the org and its repos exist. Nothing here is served by the website
(`site/`) or compiled into the crate — it's plain documentation.

## Where each file goes

| File in this folder      | Destination                                             |
| ------------------------ | ------------------------------------------------------- |
| `profile-README.md`      | `klavyn/.github` → `profile/README.md` (org landing page) |
| `CONTRIBUTING.md`        | `klavyn/.github` → `CONTRIBUTING.md` (inherited org-wide)  |
| `dictionaries-README.md` | `klavyn/dictionaries` → `README.md` (when that repo is created) |

## Migration checklist

1. Create the org: <https://github.com/organizations/plan> → Free → name `klavyn`.
2. Transfer `nickvigilante/klavyn` → the `klavyn` org
   (repo Settings → Danger Zone → Transfer ownership). The old URL keeps redirecting.
3. Merge the branch that canonicalizes URLs to `klavyn/klavyn`.
4. Create `klavyn/.github`; move `profile-README.md` and `CONTRIBUTING.md` into it.
5. When ready, create `klavyn/dictionaries`; move `dictionaries-README.md` in as its `README.md`.
6. Delete this `docs/org/` folder once everything has landed.
