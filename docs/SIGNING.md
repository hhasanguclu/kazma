# Code signing

`kz.exe` is not code-signed yet, so Windows SmartScreen may show
"Windows protected your PC" the first time it runs. Signing removes that.

[SignPath Foundation](https://signpath.org) signs open-source projects for free.
CI is already wired for it: once the settings below exist, every release exe is
signed before it is zipped, hashed and published. Until then the signing step
is skipped and releases stay unsigned.

## 1. Apply

1. Read the conditions at <https://signpath.org/terms> (OSI license, public repo,
   builds from CI, no malware-like behaviour). kazma is MIT and built by GitHub Actions.
2. Apply at <https://signpath.org/apply> with the repository URL.
3. After approval, SignPath creates an organization for you. In it:
   - create a project with slug `kazma`, linked to this GitHub repository,
   - add the predefined GitHub trusted build system,
   - create a signing policy with slug `release-signing`,
   - set the artifact configuration to sign a single PE file (`kz.exe`) inside the artifact zip.

## 2. Connect the repository

GitHub, repository **Settings -> Secrets and variables -> Actions**:

| Kind | Name | Value |
|---|---|---|
| Secret | `SIGNPATH_API_TOKEN` | API token of a SignPath CI user with submitter rights |
| Variable | `SIGNPATH_ORGANIZATION_ID` | your SignPath organization id |
| Variable | `SIGNPATH_PROJECT_SLUG` | `kazma` (optional, this is the default) |
| Variable | `SIGNPATH_POLICY_SLUG` | `release-signing` (optional, this is the default) |

## 3. Release

Bump `version` in `Cargo.toml` and push. The **Sign with SignPath** step now runs,
waits for the signed exe, and the release, zip, checksums and Scoop/winget
manifests all use the signed file.

## Notes

- New signatures build SmartScreen reputation over time; the first signed
  releases may still show a warning for a short while.
- Paid alternative: Azure Trusted Signing (about $10/month), which also works
  from GitHub Actions.
