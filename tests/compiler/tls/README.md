# A certificate for TLS tests (#118)

`localhost.cert.pem` and `localhost.key.pem` are a self-signed certificate
and its key for `localhost` and `127.0.0.1`, for test programs only: the key
is public, so nothing may trust this certificate but a test. Made once, valid
until 2126, with:

```sh
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -days 36500 -subj /CN=localhost \
  -addext subjectAltName=DNS:localhost,IP:127.0.0.1 \
  -addext basicConstraints=critical,CA:FALSE \
  -keyout localhost.key.pem -out localhost.cert.pem
```

`CA:FALSE`, because `rustls` refuses a CA certificate as a server's own.
