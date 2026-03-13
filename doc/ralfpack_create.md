## ralfpack create

Creates and optionally signs a new package from image content and configuration file.

### Synopsis

Given a directory or an archive with app image content, and a configuration file, creates a new package file.
If signing options are provided, the package will be signed during creation.

The content can be provided as a directory or an archive file (tar, tar.gz, tar.zst, or zip).

The config file must be a JSON file that conforms to the RALF package configuration schema.

If you want the package signed then you must provide either a PEM encoded RSA private key file or a PKCS12
(.p12) file containing the certificate(s) and private key for signing.  See [ralfpack sign](ralfpack_sign.md) for
more details on signing options.

```
ralfpack create [flags] <RALF_PACKAGE>
```

### Examples

```
ralfpack create --content <path to directory or archive> --config <path to json config file> <RALF_PACKAGE>
```

### Options

```
  -i, --content <CONTENT>                        The path to a directory or archive containing the content to be packaged
  -c, --config <CONFIG>                          The path to a JSON file containing the package configuration
      --no-schema-check                          Disabled the JSON schema check on the configuration file
      --image-format <IMAGE_FORMAT>              The format to use for the package content image. The tool supports tar with optional compression (gzip or zstd)
                                                 and EROFS images.  By default, the tool will use tar for small packages and EROFS for larger packages. Possible
                                                 values are: 'tar', 'tar.gz', 'tar.zst', 'erofs' (alias for 'erofs.lz4'), 'erofs.lz4' & 'erofs.zstd'
      --annotations <ANNOTATIONS>                TODO: Include extra key=value annotations in the package
      --auxiliary-content <AUXILIARY_CONTENT>    TODO: Include an auxiliary metadata file in the package
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
