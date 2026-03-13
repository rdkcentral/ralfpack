# ralfpack - Create RALF (RDK Application Layer Format) Packages

## What is this?
Command line tool to create and sign RALF packages for RDK applications. It takes an archive or directory
containing the package contents, some optional signing credentials, and a JSON configuration file; and it
will produce a RALF package.

RALF packages are [OCI artifacts][2] with defined media types and a JSON configuration schema.
The format is described in detail [here][1].

This tool is expected to be used by application developers to create packages for their applications,
and also by CI/CD pipelines to automate package creation and signing as part of the build and release
process.  It is not intended to be included as part of the RDK itself, but rather to be used externally
by application developers and build systems.

_Note_: This tool currently uses the [colog][3] crate for logging, which is a Rust logging library,
licensed under LGPL-3.0.

## Documentation
Documentation for the tool is available in the `doc` directory:
* [ralfpack](doc/ralfpack.md) - Global options and commands.
* [ralfpack create](doc/ralfpack_create.md) - Create a new package from image content and configuration file.
* [ralfpack convert](doc/ralfpack_convert.md) - Convert a EntOS widget to RALF package format
* [ralfpack sign](doc/ralfpack_sign.md) - Sign or resign an existing RALF package
* [ralfpack verify](doc/ralfpack_verify.md) - Verify a RALF package's signature
* [ralfpack info](doc/ralfpack_info.md) - Display information about a RALF package

## How to use it
Once built you can run the tool with the following command:
```bash
ralfpack create --pkcs12=<PATH_TO_SIGNING_CREDS> --content=<PATH_TO_CONTENTS_DIR_OR_ARCHIVE> --config=<PATH_TO_JSON_CONFIG_FILE> <RALF_PACKAGE>
```
Example:
```bash
ralfpack create --pkcs12=unsecure-signing.p12 --content=myapp-files.zip --config=myapp-config.json myapp.ralf
```

## EntOS Widget Conversion
The tool supports converting EntOS widget packages to RALF packages. To do this, simply provide the path to the
`.wgt` file as the `--widget` argument. The tool will extract the contents of the widget, generate the necessary
OCI configuration, and create a RALF package.  You will need to also provide signing credentials, as normal, for
the RALF package to be signed.

Example:
```bash
ralfpack convert --pkcs12=signing-creds.p12 --widget=com.sky.sports.wgt com.sky.sports.ralf
```


[1]: https://github.com/rdkcentral/oci-package-spec
[2]: https://edu.chainguard.dev/open-source/oci/what-are-oci-artifacts/
[3]: https://crates.io/crates/colog
