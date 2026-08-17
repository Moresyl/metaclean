# Security policy

## Supported versions

Security fixes are applied to the latest release.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting for this repository. Do not open a public issue containing exploit details or sensitive sample files.

Include the affected format, a minimal reproduction when safe, expected behavior, and impact. MetaClean processes attacker-controlled files, so denial-of-service, path traversal, archive expansion, parser confusion, and source-file corruption reports are especially valuable.

Never upload documents containing personal or confidential data. Replace them with a minimized synthetic fixture.

## Update trust

Installed MetaClean builds accept updates only when the package matches the
minisign public key embedded in the application. The release manifest cannot
disable verification or replace that trust root. Windows portable packages and
Linux DEB/RPM builds do not modify themselves; they direct users to the official
GitHub Releases page instead.

Please report a missing or mismatched signature, an updater target that can
escape the official `Moresyl/metaclean` release path, or any update flow that
can overwrite a portable or system-managed installation as a security issue.
