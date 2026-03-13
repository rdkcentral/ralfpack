## ralfpack info signing

Displays information on the signature of a RALF package.

### Synopsis

Prints information on the signature of a RALF package, including details of the signing certificate and any
certificate chain included in the signature.  This is useful to inspect the signature who signed the package and
when the signing certificate(s) expire.

It is possible to sign a RALF package without including the signing certificate or certificate chain in the signature,
in which case this tool will indicate that no signing certificate is present.

```
ralfpack info signing [options] <RALF_PACKAGE>
```

### Examples

```
  # Dump the signing certificate in PEM format to stdout
  ralfpack info signing <RALF_PACKAGE>

  # Dump the signing certificate and certificate chain (if present) in PEM format to stdout
  ralfpack info signing --cert-chain <RALF_PACKAGE>

  # Dump the signing certificate and certificate chain (if present) in openssl style readable text format to stdout
  ralfpack info signing --cert-chain --text <RALF_PACKAGE>
```

### Options

```
      --certificate-chain  If set, also output the full certificate chain (if present) after the signing certificate
      --text               When outputting certificates, if this flag is set then pretty print the certificate(s) in openssl style text format, otherwise output
                           in raw PEM format
  -h, --help               Print help
```

### Options inherited from parent commands

```
  -v, --verbose...  Increase output verbosity, can be used multiple times
  -h, --help        Print help
```

### SEE ALSO

* [ralfpack](ralfpack.md) - A tool for Creating, Signing, Verifying and Inspecting RALF (RDK Application Layer Format) package files.
