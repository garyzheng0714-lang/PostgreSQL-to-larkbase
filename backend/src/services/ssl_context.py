"""SSL context builder for PostgreSQL connections.

Builds ssl.SSLContext objects based on ssl_mode, adapted from pgAdmin4's
libpq parameter model for use with asyncpg.

asyncpg limitation: 'allow' and 'prefer' modes are not natively supported.
They are mapped to 'require' (always use SSL, don't verify certs).
"""

import os
import ssl
import tempfile


def build_ssl_context(
    ssl_mode: str = "disable",
    ssl_root_cert: str | None = None,
    ssl_cert: str | None = None,
    ssl_key: str | None = None,
) -> ssl.SSLContext | None:
    """Build an SSL context based on the ssl_mode.

    Args:
        ssl_mode: One of disable, allow, prefer, require, verify-ca, verify-full.
        ssl_root_cert: PEM-encoded CA certificate string.
        ssl_cert: PEM-encoded client certificate string.
        ssl_key: PEM-encoded client key string.

    Returns:
        None if SSL is disabled, otherwise an ssl.SSLContext.

    Raises:
        ValueError: If ssl_mode is unknown or ssl_root_cert is invalid PEM.
        ssl.SSLError: If certificate loading fails.
    """
    if ssl_mode == "disable":
        return None

    if ssl_mode in ("allow", "prefer", "require"):
        check_hostname, verify_mode = False, ssl.CERT_NONE
    elif ssl_mode == "verify-ca":
        check_hostname, verify_mode = False, ssl.CERT_REQUIRED
    elif ssl_mode == "verify-full":
        check_hostname, verify_mode = True, ssl.CERT_REQUIRED
    else:
        raise ValueError(f"Unknown ssl_mode: {ssl_mode!r}")

    ctx = ssl.create_default_context()
    ctx.check_hostname = check_hostname
    ctx.verify_mode = verify_mode

    if ssl_root_cert:
        if not ssl_root_cert.strip().startswith("-----BEGIN"):
            raise ValueError("Invalid PEM: must start with -----BEGIN")
        ctx.load_verify_locations(cadata=ssl_root_cert)

    if ssl_cert and ssl_key:
        cert_fd, cert_path = tempfile.mkstemp(suffix=".pem")
        key_fd, key_path = tempfile.mkstemp(suffix=".pem")
        try:
            os.write(cert_fd, ssl_cert.encode())
            os.close(cert_fd)
            cert_fd = -1
            os.write(key_fd, ssl_key.encode())
            os.close(key_fd)
            key_fd = -1
            os.chmod(key_path, 0o600)
            ctx.load_cert_chain(certfile=cert_path, keyfile=key_path)
        finally:
            for fd in (cert_fd, key_fd):
                if fd >= 0:
                    os.close(fd)
            os.unlink(cert_path)
            os.unlink(key_path)

    return ctx
