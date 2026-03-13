## ralfpack convert

Convert an EntOS widget to RALF package format.

### Synopsis

Convert a EntOS widget to RALF package format with optional signing.


```
ralfpack convert [flags] --widget <WIDGET> <RALF_PACKAGE>
```

### Examples

```
ralfpack convert --content <path to directory or archive> --config <path to json config file> <RALF_PACKAGE>
```

### Options

```
      --widget <WIDGET>                            Path to the widget file to convert
      --widget-version <WIDGET_VERSION>            The semantic version of the widget file when converting to a RALF package. If not specified the tool will
                                                   attempt to guess the semantic version from the version string in the config.xml
      --version-name-suffix <VERSION_NAME_SUFFIX>  Optional argument to append a suffix to the `versionName` field in the package config, for example if the
                                                   widget version is `1.2.3` and the suffix is `-beta1` then the resulting `versionName` in the package config
                                                   will be `1.2.3-beta1`
      --image-format <IMAGE_FORMAT>                The format to use for the package content image. The tool supports tar with optional compression (gzip or
                                                   zstd) and EROFS images.  By default, the tool will use tar for small packages and EROFS for larger packages.
                                                   Possible values are: 'tar', 'tar.gz', 'tar.zst', 'erofs' (alias for 'erofs.lz4'), 'erofs.lz4' & 'erofs.zstd'
      --pkcs12 <PKCS12>                            The path to PKCS12 (.p12) file containing the certificate(s) and private key for signing
      --key <KEY>                                  Path to the RSA PEM key file used for signing the package
      --passphrase <PASSPHRASE>                    The passphrase for the key file, if '-' then the passphrase is read from stdin. If no passphrase is provided
                                                   then you may be prompted for it if the key is encrypted. Passphrase may also be set as an environment
                                                   variable by specifying 'env://[ENV_VAR_NAME]'
      --certificate <CERTIFICATE>                  Path to the X.509 certificate in PEM format to include in the OCI Signature
      --certificate-chain <CERTIFICATE_CHAIN>      Path to a PEM file containing one or more X.509 certificates to include in the OCI Signature to build the
                                                   certificate chain for verifying the signing certificate. This optional argument can be specified multiple
                                                   times to include multiple discrete X.509 certificates. These certificates ate included in the package
                                                   signature
      --signature-identity <SIGNATURE_IDENTITY>    Manually set the .critical.docker-reference field in the Signature to the given value. By default, this is
                                                   set to the package id (e.g. com.example.myapp)
      --skip-certificate-expiry-check              Skip the check on the expiry of the signing certificate (and optional chain).  By default, the tool checks
                                                   that the signing certificate(s) have at least 3 years before expiry
      --remove-configxml                           If not specified then the output package will contain a copy of the original config.xml from the widget. If
                                                   this flag is set then the config.xml will be omitted from the package. The config.xml is not required in a
                                                   RALF package, it is provided for backwards compatibility with EntOS apps that expect it to be present at
                                                   runtime
  -h, --help                                       Print help

```

### Options inherited from parent commands

```
  -v, --verbose...  Increase output verbosity, can be used multiple times
  -h, --help        Print help
```

### SEE ALSO

* [ralfpack](ralfpack.md) - A tool for Creating, Signing, Verifying and Inspecting RALF (RDK Application Layer Format) package files.
