"""Tests for SSL context builder."""

import ssl

import pytest

from src.services.ssl_context import build_ssl_context


def _generate_self_signed_ca_pem() -> str:
    import subprocess
    result = subprocess.run(
        ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-keyout", "/dev/null",
         "-out", "/dev/stdout", "-days", "1", "-nodes", "-subj", "/CN=testca", "-batch"],
        capture_output=True, text=True,
    )
    assert result.returncode == 0, f"openssl failed: {result.stderr}"
    return result.stdout.strip()


TEST_CA_PEM = _generate_self_signed_ca_pem()


def test_disable_returns_none():
    assert build_ssl_context(ssl_mode="disable") is None


def test_require_returns_ssl_context():
    result = build_ssl_context(ssl_mode="require")
    assert isinstance(result, ssl.SSLContext)
    assert result.check_hostname is False


def test_verify_full_checks_hostname():
    result = build_ssl_context(ssl_mode="verify-full")
    assert isinstance(result, ssl.SSLContext)
    assert result.check_hostname is True
    assert result.verify_mode == ssl.CERT_REQUIRED


def test_verify_ca_no_hostname_check():
    result = build_ssl_context(ssl_mode="verify-ca")
    assert isinstance(result, ssl.SSLContext)
    assert result.check_hostname is False
    assert result.verify_mode == ssl.CERT_REQUIRED


def test_ca_cert_loads():
    result = build_ssl_context(ssl_mode="verify-full", ssl_root_cert=TEST_CA_PEM)
    assert isinstance(result, ssl.SSLContext)


def test_invalid_pem_raises():
    with pytest.raises((ssl.SSLError, ValueError)):
        build_ssl_context(ssl_mode="verify-full", ssl_root_cert="not a pem")


def test_allow_mapped_to_require():
    result = build_ssl_context(ssl_mode="allow")
    assert isinstance(result, ssl.SSLContext)
    assert result.check_hostname is False


def test_prefer_mapped_to_require():
    result = build_ssl_context(ssl_mode="prefer")
    assert isinstance(result, ssl.SSLContext)


def test_unknown_ssl_mode_raises():
    with pytest.raises(ValueError, match="Unknown ssl_mode"):
        build_ssl_context(ssl_mode="bogus")
