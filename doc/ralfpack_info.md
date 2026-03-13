## ralfpack info

Displays summary information about a RALF package, with various subcommands to show specific details.

### Synopsis

By default, the info command shows a summary of the package, including package id, version, size, content image type,
signing status etc.  There are also subcommands to show specific information about the package, see
[ralfpack info signing](ralfpack_info_signing.md) for details on the package signature and
[ralfpack info config](ralfpack_info_config.md) for details on the package meta-data configuration.

```
ralfpack info <RALF_PACKAGE>
```

### Examples

```
  # Dump summary information about a RALF package
  ralfpack info <<RALF_PACKAGE>>
```

### Sub Commands

```
  config    Display the package configuration information
  signing   Display information on the signature of a package
  help      Print this message or the help of the given subcommand(s)
```

### Options inherited from parent commands

```
  -v, --verbose...  Increase output verbosity, can be used multiple times
  -h, --help        Print help
```

### SEE ALSO

* [ralfpack](ralfpack.md) - A tool for Creating, Signing, Verifying and Inspecting RALF (RDK Application Layer Format) package files.
