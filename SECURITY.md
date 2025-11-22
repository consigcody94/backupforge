# Security Policy

## Supported Versions

We release patches for security vulnerabilities for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to: security@backupforge.example.com
(Replace with actual contact email)

You should receive a response within 48 hours. If for some reason you do not, please follow up via email to ensure we received your original message.

Please include the following information:

- Type of vulnerability
- Full paths of source file(s) related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability, including how an attacker might exploit it

## Security Best Practices

### For Users

1. **Encryption**
   - Always use encryption for sensitive data
   - Use strong, unique passwords for encryption keys
   - Store encryption keys securely (password manager, key vault)
   - Never commit encryption keys to version control

2. **Authentication**
   - Use strong passwords for API authentication
   - Rotate JWT secrets regularly
   - Use HTTPS for all API communication
   - Enable 2FA where available (planned feature)

3. **Access Control**
   - Follow principle of least privilege
   - Restrict file permissions on backup directories (700)
   - Protect configuration files (600)
   - Use separate service accounts for backup operations

4. **Network Security**
   - Use SSH key-based authentication instead of passwords
   - Disable root SSH access on backup targets
   - Use VPN for remote backups over untrusted networks
   - Enable TLS for API server (use reverse proxy)

5. **Storage**
   - Enable versioning on S3 buckets
   - Use bucket policies to restrict access
   - Enable S3 encryption at rest
   - Regularly test backup restoration

### For Developers

1. **Code Security**
   - Never use `unwrap()` in production code paths
   - Validate all user input
   - Use parameterized queries (no SQL injection)
   - Avoid unsafe code unless absolutely necessary
   - Run `cargo clippy` and address all warnings

2. **Dependency Management**
   - Regularly update dependencies
   - Review dependency security advisories
   - Use `cargo audit` to check for vulnerabilities
   - Pin dependency versions in production

3. **Secrets Management**
   - Never commit secrets to version control
   - Use environment variables or secure vaults
   - Rotate secrets regularly
   - Use `.gitignore` for sensitive files

4. **Testing**
   - Write security-focused tests
   - Test authentication and authorization
   - Validate encryption/decryption
   - Test input validation

## Security Features

### Encryption

- **Algorithm**: AES-256-GCM (authenticated encryption)
- **Key Derivation**: Argon2 (memory-hard, GPU-resistant)
- **Nonce**: 96-bit random nonce per chunk
- **Authentication**: 128-bit authentication tag

### Hashing

- **Algorithm**: BLAKE3 (faster and more secure than SHA-256)
- **Use**: Chunk identification, deduplication

### Authentication

- **Method**: JWT tokens (planned: configurable expiry)
- **Password Storage**: bcrypt hashing (planned: Argon2)
- **Session Management**: Token-based (stateless)

### Data Protection

- **At Rest**: Optional AES-256-GCM encryption
- **In Transit**: TLS 1.3 recommended (via reverse proxy)
- **Metadata**: Can be encrypted separately

### Access Control

- **Multi-tenancy**: Tenant isolation for MSPs
- **RBAC**: Role-based access control (planned)
- **API Keys**: Scoped API keys (planned)

## Known Limitations

### Current Version (0.1.0)

1. **In-Memory Deduplication Index**
   - Dedup index is not persisted
   - Must be rebuilt on restart
   - **Mitigation**: Save index to disk (planned for v0.2.0)

2. **Password Storage**
   - Placeholder authentication in API
   - **Mitigation**: Implement proper password hashing (v0.2.0)

3. **No Rate Limiting**
   - API has no rate limiting
   - **Mitigation**: Use reverse proxy with rate limiting

4. **No Audit Logging**
   - Limited audit trail
   - **Mitigation**: Implement comprehensive audit logs (v0.3.0)

5. **SSH Password Authentication**
   - Supports password auth for SSH
   - **Recommendation**: Use key-based authentication only

## Vulnerability Disclosure Timeline

1. **Day 0**: Vulnerability reported
2. **Day 1-2**: Acknowledgment sent to reporter
3. **Day 3-7**: Vulnerability assessed and confirmed
4. **Day 7-30**: Fix developed and tested
5. **Day 30**: Security advisory published
6. **Day 30**: Patch released
7. **Day 45**: Public disclosure (if not disclosed earlier)

## Security Advisories

Security advisories will be published at:
- GitHub Security Advisories
- Project website
- Release notes

## Bug Bounty Program

Currently, we do not have a bug bounty program. However, we greatly appreciate security researchers who responsibly disclose vulnerabilities.

## Acknowledgments

We would like to thank the following security researchers who have helped improve BackupForge's security:

*(List will be maintained as contributions are made)*

## Security Updates

Subscribe to security updates:
- Watch the GitHub repository
- Follow releases
- Join our security mailing list (planned)

## Additional Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [CWE Top 25](https://cwe.mitre.org/top25/)

## Contact

For security-related questions or concerns, please contact:
security@backupforge.example.com

Last Updated: 2024-01-21
