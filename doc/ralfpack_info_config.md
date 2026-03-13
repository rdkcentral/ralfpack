## ralfpack info config

Displays information on meta-data / config of the package.

### Synopsis

Displays the JSON configuration meta-data stored in the RALF package.  This includes information such as package name,
version, permissions, feature configuration etc.

It also supports converting the meta-data to other legacy formats, such as W3C widget config format.

```
ralfpack info config [options] <RALF_PACKAGE>
```

### Examples

```
  # Dump the config JSON file to stdout
  ralfpack info config <RALF_PACKAGE>

  # Convert the config JSON to EntOS widget config.xml format, this may be lossy as not all fields
  # have a direct mapping in config.xml
  ralfpack info config --format configxml <RALF_PACKAGE>
```

### Options

```
  -f, --format <FORMAT>  The format to output the configuration in, either 'raw', 'json' or 'configxml', defaults to 'json' [default: json]
  -h, --help             Print help
```

### Options inherited from parent commands

```
  -v, --verbose...  Increase output verbosity, can be used multiple times
  -h, --help        Print help
```

### SEE ALSO

* [ralfpack](ralfpack.md) - A tool for Creating, Signing, Verifying and Inspecting RALF (RDK Application Layer Format) package files.
