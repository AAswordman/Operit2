"""Optional live DNS and certificate-verified HTTPS acceptance."""
import json
import socket
import ssl
import sys

hostname = sys.argv[1] if len(sys.argv) > 1 else 'example.com'
addresses = socket.getaddrinfo(hostname, 443, type=socket.SOCK_STREAM)
assert addresses
context = ssl.create_default_context(cafile='/etc/ssl/certs/ca-certificates.crt')
with socket.create_connection((hostname, 443), timeout=10) as transport:
    with context.wrap_socket(transport, server_hostname=hostname) as stream:
        assert stream.getpeercert() and stream.version()
        stream.sendall(f'HEAD / HTTP/1.1\r\nHost: {hostname}\r\nConnection: close\r\n\r\n'.encode())
        response = stream.recv(4096)
        assert response.startswith(b'HTTP/1.')
        print(json.dumps({'test': 'python-external-network', 'ok': True, 'hostname': hostname,
                          'address_count': len(addresses), 'tls': stream.version(),
                          'status': response.split(b'\r\n', 1)[0].decode()}))
