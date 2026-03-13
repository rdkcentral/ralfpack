## ralfpack sign

Signs or re-signs an existing RALF package.

### Synopsis

Signs or re-signs an existing RALF package.

The minimum signing requirement is a PEM encoded private key file.  Instead, you can also provide a PKCS12
(.p12) file containing the private key and optional signing certificate and certificate chain.

In a typical use case, you would provide a PEM encoded private key file along with a PEM encoded signing
certificate and a PEM encoded certificate chain file containing the intermediate certificates needed to build
the chain up to a trusted root certificate.

Passphrases for encrypted private keys can be provided via the `--passphrase` option, or if not provided
you may be prompted for it if the key is encrypted.  Rather than providing the passphrase on the command line,
it can also be set as an environment variable by calling `--passphrase env://[ENV_VAR_NAME]`.  Finally, if you
specify `--passphrase -` then the passphrase will be read from stdin.

```
ralfpack sign [flags] <RALF_PACKAGE>
```

### Examples

```
ralfpack sign --key <private key> --passphrase <key passphrase> <RALF_PACKAGE>
```

### Options

```
      --pkcs12 <PKCS12>                          The path to PKCS12 (.p12) file containing the certificate(s) and private key for signing
      --key <KEY>                                Path to the RSA PEM key file used for signing the package
      --passphrase <PASSPHRASE>                  The passphrase for the key file, if '-' then the passphrase is read from stdin. If no passphrase is provided
                                                 then you may be prompted for it if the key is encrypted. Passphrase may also be set as an environment variable
                                                 by specifying 'env://[ENV_VAR_NAME]'
      --certificate <CERTIFICATE>                Path to the X.509 certificate in PEM format to include in the OCI Signature
      --certificate-chain <CERTIFICATE_CHAIN>    Path to a PEM file containing one or more X.509 certificates to include in the OCI Signature to build the
                                                 certificate chain for verifying the signing certificate. This optional argument can be specified multiple times
                                                 to include multiple discrete X.509 certificates. These certificates ate included in the package signature
      --signature-identity <SIGNATURE_IDENTITY>  Manually set the .critical.docker-reference field in the Signature to the given value. By default, this is set
                                                 to the package id (e.g. com.example.myapp)
      --skip-certificate-expiry-check            Skip the check on the expiry of the signing certificate (and optional chain).  By default, the tool checks that
                                                 the signing certificate(s) have at least 3 years before expiry
  -h, --help                                     Print help
```

### Options inherited from parent commands

```
  -v, --verbose...  Increase output verbosity, can be used multiple times
  -h, --help        Print help
```

### SEE ALSO

* [ralfpack](ralfpack.md) - A tool for Creating, Signing, Verifying and Inspecting RALF (RDK Application Layer Format) package files.
