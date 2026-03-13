## ralfpack verify

Verifies an existing RALF package.

### Synopsis

Verifies an existing RALF package.

You can verify the package signature using either a public key or a trusted root certificate.  For the later case the
signature will need to either contain the signing certificate and intermediate certificates needed to build the chain
up to the trusted root, or you will need to provide the signing certificate and intermediate certificates via the 
`--certificate` and `--certificate-chain` options.

```
ralfpack verify [flags] <RALF_PACKAGE>
```

### Examples

```
ralfpack verify --key <public key> <RALF_PACKAGE>
```

```
ralfpack verify --ca-roots <ca bundle path> <RALF_PACKAGE>
```

### Options

```
      --ca-roots <CA_ROOTS>                    Path to a PEM file containing one or more trusted CA root certificates.  Use this option if the package signature
                                               contains a signing certificate and (optionally) a certificate chain to build a chain to one of the given CA
                                               roots.  If the package signature does not contain a signing certificate, this option must be used in conjunction
                                               with --certificate (or --key)
      --key <KEY>                              Path to a PEM file containing a trusted public key.  Use this option if the package signature does not contain a
                                               signing certificate, or if you want to verify the signature using a specific public key
      --certificate <CERTIFICATE>              Path to a PEM file containing a public certificate, which will be verified along with the (optional) certificate
                                               chain and given CA roots.  If the signature in the package already contains the signing certificate, this option
                                               is not required, but if given, it will override any certificate in the package signature
      --certificate-chain <CERTIFICATE_CHAIN>  Path to a PEM file containing an optional certificate chain, which will be used to build a certificate chain from
                                               the given certificate to one of the given CA roots. If the signature in the package already contains a
                                               certificate chain, this option is not required, but if given, it will override any certificate chain in the
                                               package signature
      --no-check-time                          Do not check the validity period of certificates when verifying, this includes any CA roots, the signing
                                               certificate and any certificates in the chain. This can be useful when verifying packages signed with
                                               certificates that have expired, but you still want to verify the signature and the certificate chain
  -h, --help                                   Print help

```

### Options inherited from parent commands

```
  -v, --verbose...  Increase output verbosity, can be used multiple times
  -h, --help        Print help
```

### SEE ALSO

* [ralfpack](ralfpack.md) - A tool for Creating, Signing, Verifying and Inspecting RALF (RDK Application Layer Format) package files.
