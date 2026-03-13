//
// If not stated otherwise in this file or this component's LICENSE file the
// following copyright and licenses apply:
//
// Copyright 2025 Comcast Cable Communications Management, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use crate::dmverity::dmverity_gen::{DmVerityOptions, dmverity_build_and_append};
use crate::erofs::erofs_image;
use crate::erofs::erofs_image::{CompressionAlgo, ErofsImageBuilder};
use crate::package::PackageBlob;
use log::debug;
use oci_spec::image::{MediaType, Sha256Digest};
use std::collections::HashMap;
use std::io::{BufRead, Read, Seek, Write};
use std::path::Path;
use std::str::FromStr;
use std::time::SystemTime;
use std::{fmt, fs, io};
use tempfile::tempfile;

/// The name of the annotation that contains the root hash - for obvious reasons this annotation
/// should be covered by the package signature.
pub const ANNOTATION_DMVERITY_ROOTHASH: &str = "org.rdk.package.content.dmverity.roothash";

/// The name of the annotation that contains the offset of the dm-verity superblock in the image.
/// By convention the dm-verity data is appended onto the end of the image, and the first 4k of
/// data is used for the superblock.
pub const ANNOTATION_DMVERITY_OFFSET: &str = "org.rdk.package.content.dmverity.offset";

/// The name of the annotation that contains the dm-verity salt value.  This is optional and a
/// duplicate of the salt value in the dm-verity superblock.
pub const ANNOTATION_DMVERITY_SALT: &str = "org.rdk.package.content.dmverity.salt";

#[derive(PartialEq, Clone, Debug)]
pub enum PackageContentFormat {
    Tar = 1,
    TarGz = 2,
    TarZstd = 4,

    ErofsUncompressed = 10,
    ErofsLz4 = 11,
    ErofsZstd = 12,
}

impl fmt::Display for PackageContentFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PackageContentFormat::Tar => "tar",
            PackageContentFormat::TarGz => "tar.gz",
            PackageContentFormat::TarZstd => "tar.zstd",
            PackageContentFormat::ErofsLz4 => "erofs.lz4",
            PackageContentFormat::ErofsZstd => "erofs.zstd",
            PackageContentFormat::ErofsUncompressed => "erofs.nocmpr",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for PackageContentFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tar" => Ok(PackageContentFormat::Tar),
            "tar.gz" => Ok(PackageContentFormat::TarGz),
            "tar.zstd" => Ok(PackageContentFormat::TarZstd),
            "erofs" => Ok(PackageContentFormat::ErofsLz4),
            "erofs.lz4" => Ok(PackageContentFormat::ErofsLz4),
            "erofs.zstd" => Ok(PackageContentFormat::ErofsZstd),
            "erofs.nocmpr" => Ok(PackageContentFormat::ErofsUncompressed),
            _ => Err(format!("Invalid content format: {}", s)),
        }
    }
}

/// Stores the built package content.  Typically this is just a wrapper around a tar or an EROFS
/// image temporary file.  It also stores the length in bytes, the SHA256 digest and the media type
/// of the content.
pub struct PackageContent {
    /// The mediaType of the content.
    media_type: MediaType,

    /// The length of the content in bytes.
    size: u64,

    /// The SHA256 digest of the content.
    digest: Sha256Digest,

    /// The actual content temporary file.
    file: fs::File,

    /// Annotations on the content.  This is used to store the dm-verity root hash and salt.
    annotations: HashMap<String, String>,
}

impl PackageContent {
    /// Creates a new PackageContent from supplied file and media type.  The file is expected to
    /// be at the start of the file, and the size and digest are calculated.
    pub fn new(media_type: &MediaType, mut file: fs::File) -> io::Result<PackageContent> {
        // Seek to the start of the file
        file.rewind()?;

        // Calculate the size and digest of the file
        let mut hasher = openssl::sha::Sha256::new();
        let mut buffer = vec![0; 8192];
        let mut total_bytes = 0;
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
            total_bytes += bytes_read as u64;
        }

        let sha256 = hasher.finish();
        let digest = Sha256Digest::from_str(hex::encode(sha256).as_str())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to parse SHA256 digest: {}", e)))?;

        // Seek back to the start of the file
        file.rewind()?;

        // Create the PackageContent
        Ok(PackageContent {
            media_type: media_type.clone(),
            size: total_bytes,
            digest,
            file,
            annotations: HashMap::new(),
        })
    }

    /// Returns the media type of the content.
    #[allow(dead_code)]
    pub fn media_type(&self) -> MediaType {
        self.media_type.clone()
    }

    /// Returns the file name extension for the content based on the media type.  This is mainly
    /// used to et the 'image.title' annotation on the OCI descriptor.
    #[allow(dead_code)]
    pub fn extension(&self) -> String {
        match self.media_type {
            MediaType::Other(ref s) if s.contains("tar+gzip") => "tar.gz".to_string(),
            MediaType::Other(ref s) if s.contains("tar+zstd") => "tar.zst".to_string(),
            MediaType::Other(ref s) if s.contains("tar") => "tar".to_string(),
            MediaType::Other(ref s) if s.contains("erofs") => "erofs.img".to_string(),
            _ => "bin".to_string(),
        }
    }

    /// Adds an annotation to the content.  This is typically used to store the dm-verity root
    /// hash and salt.
    ///
    /// The annotations are uss when the descriptor is generated.
    #[allow(dead_code)]
    pub fn add_annotation(&mut self, key: &str, value: &str) {
        self.annotations.insert(key.to_string(), value.to_string());
    }
}

impl Clone for PackageContent {
    fn clone(&self) -> Self {
        let file_clone = self.file.try_clone().expect("Failed to clone temporary content file");

        PackageContent {
            media_type: self.media_type.clone(),
            size: self.size,
            digest: self.digest.clone(),
            file: file_clone,
            annotations: self.annotations.clone(),
        }
    }
}

impl io::Read for PackageContent {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl io::Seek for PackageContent {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl PackageBlob for PackageContent {
    fn digest(&self) -> Sha256Digest {
        self.digest.clone()
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn descriptor(&self) -> oci_spec::image::Descriptor {
        // Create annotations for the content descriptor
        let ext = match self.media_type {
            MediaType::Other(ref s) if s.ends_with("tar+gzip") => "tar.gz".to_string(),
            MediaType::Other(ref s) if s.ends_with("tar+zstd") => "tar.zst".to_string(),
            MediaType::Other(ref s) if s.ends_with("tar") => "tar".to_string(),
            MediaType::Other(ref s) if s.ends_with("erofs") => "erofs.img".to_string(),
            _ => "".to_string(),
        };

        let mut content_annotations = HashMap::new();
        if !ext.is_empty() {
            content_annotations.insert(
                oci_spec::image::ANNOTATION_TITLE.to_string(),
                format!("package-data.{}", ext).to_string(),
            );
        }

        // Add any other optional annotations
        for (key, value) in &self.annotations {
            content_annotations.insert(key.clone(), value.clone());
        }

        // Create the content descriptor
        oci_spec::image::DescriptorBuilder::default()
            .media_type(self.media_type.clone())
            .digest(self.digest.clone())
            .size(self.size)
            .annotations(content_annotations)
            .build()
            .expect("Failed to create content descriptor")
    }
}

pub struct PackageContentBuilder {
    /// Options for the builder
    options: PackageContentBuilderOptions,

    /// The tar archive build, this is used to create the package content if the format chosen is
    /// tar (with or without compression).
    tar_builder: Option<tar::Builder<fs::File>>,

    /// The erofs image builder, this is used to create the package content if the format chosen is
    /// erofs.
    erofs_builder: Option<ErofsImageBuilder>,

    /// The current number of entries written
    total_entries: usize,

    /// The current size of the content written
    total_size: usize,
}

struct PackageContentBuilderOptions {
    /// The format of the contents of the package.
    format: PackageContentFormat,

    /// The time in seconds since the UNIX epoch that all files and directories in the package
    /// content should be set to.
    mtime: u64,

    /// The maximum size of the package content.  This is used as a sanity check to avoid
    /// creating packages that are too large to actually run on devices.
    size_limit: Option<usize>,

    /// The maximum number of file entries to be written to the content.  This is used as a sanity
    /// check to avoid creating packages that are too large to actually run on devices.
    entry_limit: Option<usize>,

    /// The list of file paths to exclude from the content.  This is used to exclude files from
    /// zip or tar archives that are not needed.
    exclusion_list: Vec<glob::Pattern>,
}

impl PackageContentBuilder {
    /// Creates a new PackageBuilder unpopulated with any data. At a minimum you need to supply
    /// a config and some content.
    pub fn new(format: &PackageContentFormat) -> PackageContentBuilder {
        // Set the defaults
        let mut builder = PackageContentBuilder {
            options: PackageContentBuilderOptions {
                format: format.clone(),
                mtime: 0,
                size_limit: None,
                entry_limit: None,
                exclusion_list: Vec::new(),
            },
            tar_builder: None,
            erofs_builder: None,
            total_entries: 0,
            total_size: 0,
        };

        // By default, for mtime, use the current time in seconds since the UNIX epoch
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
        builder.options.mtime = now.map(|d| d.as_secs()).unwrap_or(0);

        // If the format is plain tar file, then can just create a tar writer around the Write
        // object. If format is tar with compression then we also create a tar writer, but around
        // a temporary file that we will compress later.
        if builder.options.format == PackageContentFormat::Tar
            || builder.options.format == PackageContentFormat::TarGz
            || builder.options.format == PackageContentFormat::TarZstd
        {
            // Create a temporary file to hold the tar archive
            let temp_file = tempfile().expect("Failed to create temporary file");

            // Create a tar builder that writes to the temporary file
            builder.tar_builder = Some(tar::Builder::new(temp_file));
        } else if builder.options.format == PackageContentFormat::ErofsUncompressed
            || builder.options.format == PackageContentFormat::ErofsLz4
            || builder.options.format == PackageContentFormat::ErofsZstd
        {
            // Create an erofs image builder
            let mut erofs_builder = erofs_image::ErofsImageBuilder::new();

            // Set the desired compression algorithm
            match builder.options.format {
                PackageContentFormat::ErofsUncompressed => {
                    erofs_builder.compression(CompressionAlgo::None);
                }
                PackageContentFormat::ErofsLz4 => {
                    erofs_builder.compression(CompressionAlgo::Lz4);
                }
                PackageContentFormat::ErofsZstd => {
                    erofs_builder.compression(CompressionAlgo::Zstd);
                }
                _ => {}
            }

            // Set the erofs builder
            builder.erofs_builder = Some(erofs_builder);
        }

        builder
    }

    /// Sets the maximum amount of uncompressed data to be written to the content.  This is used as
    /// a sanity check to avoid people creating packages that are too large to actually run on
    /// devices, it is however optional.
    pub fn set_size_limit(&mut self, limit: usize) -> &mut Self {
        self.options.size_limit = Some(limit);
        self
    }

    /// Sets the maximum number of file entries to be written to the content.
    /// This is used as a sanity check
    pub fn set_entry_limit(&mut self, limit: usize) -> &mut Self {
        self.options.entry_limit = Some(limit);
        self
    }

    /// Adds a file name to the excluded list.  This is used to exclude files from the content that
    /// are not needed from a zip or tar archive.
    pub fn exclude_file(&mut self, path: &str) -> &mut Self {
        let pattern = glob::Pattern::new(path);
        if pattern.is_err() {
            log::warn!("Invalid exclude glob pattern: {}", pattern.unwrap_err());
        } else {
            self.options.exclusion_list.push(pattern.unwrap());
        }

        self
    }

    /// Increments the size and entry limits for the content.  And then checks if the limits have
    /// been exceeded.  If they have then an error is returned.
    fn _increment_and_check_limits(&mut self, size: usize, entries: usize) -> io::Result<()> {
        // Increment the size and entry limits
        self.total_size += size;
        self.total_entries += entries;

        // Check the size limit
        if let Some(size_limit) = self.options.size_limit {
            if self.total_size > size_limit {
                return Err(io::Error::new(io::ErrorKind::Other, "Size limit exceeded"));
            }
        }

        // Check the entry limit
        if let Some(entry_limit) = self.options.entry_limit {
            if self.total_entries > entry_limit {
                return Err(io::Error::new(io::ErrorKind::Other, "Entry limit exceeded"));
            }
        }

        Ok(())
    }

    /// Internal helper function to guess the file mode based on the first 8 bytes of the file,
    /// this checks for either the ELF header or a shebang line.
    #[allow(unused_variables)]
    fn _guess_file_mode<P: AsRef<Path>>(path: &P, buf: &[u8]) -> u32 {
        if buf.len() >= 4 && &buf[0..4] == b"\x7fELF" {
            // ELF file
            return 0o755;
        } else if buf.len() >= 2 && &buf[0..2] == b"#!" {
            // Shebang line
            return 0o755;
        }

        // Default mode
        0o644
    }

    /// Internal helper to check if the supplied path matches something in the exclusion set.
    fn _is_excluded(&self, path: &Path) -> bool {
        for pattern in &self.options.exclusion_list {
            if pattern.matches_path(path) {
                return true;
            }
        }

        false
    }

    /// Appends a file to the content with the given path.
    ///
    /// The unix mode of the file (the permissions and other attributes) are overridden and guessed
    /// for the given data using the first few bytes in the data to identify the type of file.
    /// This is because when the content is run on a device we don't want some files to be
    /// unreadable by unprivileged users.
    ///
    /// This checks the exclusion list and if the file is in the exclusion list then it is skipped.
    ///
    /// This function will also check the size limit and entry limit and if they are exceeded then
    /// an error is returned.
    pub fn append_file<P: AsRef<Path>, R: io::Read>(&mut self, path: P, data: &mut R, size: usize) -> io::Result<()> {
        // Check the entry limit
        self._increment_and_check_limits(size, 1)?;

        // Check if the file is in the exclusion list and silently skip it
        if self._is_excluded(path.as_ref()) {
            return Ok(());
        }

        // Reject absolute paths
        if path.as_ref().is_absolute() {
            return Err(io::Error::new(io::ErrorKind::Other, "Absolute paths are not allowed"));
        }

        // Create a buffer reader, so we can read the file header to guess if the file should be
        // executable and therefore what the mode should be.
        let mut buf_reader = io::BufReader::new(data);
        let mut mode = 0o644;
        if size >= 8 {
            let buf = buf_reader.fill_buf()?;
            mode = Self::_guess_file_mode(&path, &buf);
        }

        // Write the data into the content
        if let Some(tar_builder) = self.tar_builder.as_mut() {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mtime(self.options.mtime);
            header.set_mode(mode);
            header.set_uid(0);
            header.set_gid(0);
            header.set_size(size as u64);
            header.set_cksum();

            tar_builder.append_data(&mut header, path, buf_reader)
        } else if let Some(erofs_builder) = self.erofs_builder.as_mut() {
            erofs_builder.append_data(path, buf_reader, size, mode)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "No builder initialized"))
        }
    }

    /// Appends a symlink to the content.
    ///
    /// If the path matches a path in the exclusion list then the symlink is skipped.
    pub fn append_link<P: AsRef<Path>, T: AsRef<Path>>(&mut self, path: P, target: T) -> io::Result<()> {
        // Check the size limit and entry limit
        let target_len = path.as_ref().to_str().unwrap().len();
        self._increment_and_check_limits(target_len, 1)?;

        // Check if the directory is in the exclusion list and silently skip it
        if self._is_excluded(path.as_ref()) {
            return Ok(());
        }

        // Reject absolute paths - but we allow symlinks to point to absolute paths, the code on
        // the device will need to ensure that it doesn't follow symlinks when extracting
        if path.as_ref().is_absolute() {
            return Err(io::Error::new(io::ErrorKind::Other, "Absolute paths are not allowed"));
        }

        // Add the symlink to tha tarball
        if let Some(tar_builder) = self.tar_builder.as_mut() {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_mtime(self.options.mtime);
            header.set_uid(0);
            header.set_gid(0);
            header.set_size(0);

            tar_builder.append_link(&mut header, path, target)
        } else if let Some(erofs_builder) = self.erofs_builder.as_mut() {
            erofs_builder.append_link(path, target)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "No builder initialized"))
        }
    }

    /// Adds a directory to the content.  Note this doesn't add the contents of the directory,
    /// it just adds a directory entry to the archive.  The contents of the directory should be
    /// added with append_data, append_file or append_link.
    ///
    /// It's not required to add a directory entry, but it is recommended to do so because you
    /// can explicitly set the mode of the directory.  This may also be required if want to add
    /// an empty directory to the archive.
    pub fn append_dir<P: AsRef<Path>>(&mut self, path: P, mode: u32) -> io::Result<()> {
        // Check the entry limit
        self._increment_and_check_limits(0, 1)?;

        // Check if the directory is in the exclusion list and silently skip it
        if self._is_excluded(path.as_ref()) {
            return Ok(());
        }

        // Reject absolute paths
        if path.as_ref().is_absolute() {
            return Err(io::Error::new(io::ErrorKind::Other, "Absolute paths are not allowed"));
        }

        // Add the directory to the tarball
        if let Some(tar_builder) = self.tar_builder.as_mut() {
            // Can't use tar::Builder::append_dir here as it sets the mode to match the source
            // directory mode, and we want to set our own mode.
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_mode(mode);
            header.set_mtime(self.options.mtime);
            header.set_uid(0);
            header.set_gid(0);
            header.set_size(0);
            header.set_cksum();

            // Empty data for the directory
            let data: &[u8] = &[];

            tar_builder.append_data(&mut header, path, data)
        } else if let Some(erofs_builder) = self.erofs_builder.as_mut() {
            erofs_builder.append_dir(path, mode)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "No builder initialized"))
        }
    }

    /// Recursively reads the contents of the given directory and writes all the files, symlinks
    /// and directories to the content.
    ///
    /// The contents of the directory will be added to the top level in the directory tree, that
    /// is to say that no subdirectory will be created to store the directory contents.
    ///
    /// Size and entry limits are checked for each file and directory added to the content, if
    /// exceeded then an error is returned.
    ///
    pub fn append_dir_contents<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        // We can't use tar::Builder::append_dir_all here as it adds the files in a
        // subdirectory, and we want to set our mode values for the directories and files.
        for entry in walkdir::WalkDir::new(&path) {
            // Get the entry details
            let entry = entry?;
            let file_type = entry.file_type();
            let file_path = entry.path();

            // Skip the root directory
            if file_path == path.as_ref() {
                continue;
            }

            let relative_path = file_path.strip_prefix(&path).unwrap_or(file_path);

            // Check if the file is in the exclusion list
            if self._is_excluded(&relative_path) {
                continue;
            }

            // Not excluded, so add to the contents
            if file_type.is_file() {
                let size = entry.metadata()?.len() as usize;
                self.append_file(relative_path, &mut fs::File::open(file_path)?, size)?;
            } else if file_type.is_dir() {
                self.append_dir(relative_path, 0o755)?;
            } else if file_type.is_symlink() {
                let symlink_target = file_path.read_link()?;
                self.append_link(relative_path, symlink_target)?;
            } else {
                log::warn!(
                    "Ignoring file {} in directory - don't currently support that file type",
                    file_path.display()
                );
            }
        }

        Ok(())
    }

    /// Appends the content of the given zip file to the content.  This will extract the files from
    /// the zip file and add them to the content.  The zip file is not added as a single entry.
    ///
    /// The contents of the zip are checked against the exclusion list and any files that are in the
    /// exclusion list are skipped.
    ///
    pub fn append_zip<R: io::Read + io::Seek>(&mut self, zip: &mut zip::read::ZipArchive<R>) -> io::Result<()> {
        // Iterate over the files in the zip archive
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;

            let path = entry.enclosed_name();
            if path.is_none() {
                log::warn!("Ignoring zip entry with no name");
                continue;
            }

            let path = path.unwrap();

            // Check if the file is in the exclusion list
            if self._is_excluded(&path) {
                continue;
            }

            // Add the file or directory to the content
            if entry.is_file() {
                let size = entry.size() as usize;
                self.append_file(&path, &mut entry, size)?
            } else if entry.is_dir() {
                self.append_dir(&path, 0o755)?
            } else if entry.is_symlink() {
                // Although zip files can sorta contain symlinks, it's never been supported in the
                // Sky stack, so we just ignore them for now with a warning
                log::warn!("Ignoring symlink {} in zip file", path.display());
            } else {
                log::warn!("Ignoring file {} in zip", path.display());
            }
        }

        Ok(())
    }

    /// Appends the content of the given tar file to the content.
    ///
    pub fn append_tar<R: io::Read>(&mut self, tarball: &mut tar::Archive<R>) -> io::Result<()> {
        // Iterate over the files in the tar archive
        for entry in tarball.entries()? {
            let mut entry = entry?;

            // Get the path and make non-absolute
            let path = entry.path()?.to_path_buf();
            let path = path.strip_prefix("/").unwrap_or(&path);

            // Check if the file is in the exclusion list
            if self._is_excluded(&path) {
                continue;
            }

            // Add the file or directory to the content
            let entry_type = entry.header().entry_type();
            if entry_type == tar::EntryType::Regular {
                let entry_size = entry.size() as usize;
                self.append_file(&path, &mut entry, entry_size)?;
            } else if entry_type == tar::EntryType::Directory {
                self.append_dir(&path, 0o755)?
            } else if entry_type == tar::EntryType::Symlink {
                let link_name = entry.link_name()?;
                if link_name.is_none() {
                    log::warn!("Ignoring symlink {} in tar file", path.display());
                } else {
                    self.append_link(&path, link_name.unwrap())?;
                }
            } else {
                log::warn!("Ignoring file {} in tar", path.display());
            }
        }

        Ok(())
    }

    /// Appends the contents of the given zip or tar file to the package contents.  This will
    /// attempt to identify the format of the archive file using the file header, it supports the
    /// following formats:
    ///   - zip
    ///   - tar
    ///   - tar.gz
    ///   - tar.xz
    ///   - tar.zstd
    ///
    /// Internally the function calls the append_zip or append_tar functions depending on the
    /// identified format of the given file.  If it cannot identify the format, or it is not
    /// supported then an error is returned.
    ///
    /// The contents of the archive are checked against the exclusion list and any files that are in
    /// the exclusion list are skipped.
    ///
    pub fn append_archive_contents<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        // Get the type of the file
        let info = infer::Infer::new();
        let file_type = info.get_from_path(&path)?;
        if file_type.is_none() {
            return Err(io::Error::new(io::ErrorKind::Other, "Unknown archive format"));
        }

        let mime_type = file_type.unwrap().mime_type();

        // Open the archive file
        let file = fs::File::open(path)?;

        // Check if the type is one we support
        if mime_type == "application/zip" {
            let mut archive = zip::read::ZipArchive::new(file).expect("Failed to read zip file");
            self.append_zip(&mut archive)
        } else if mime_type == "application/x-tar" {
            let mut tarball = tar::Archive::new(file);
            self.append_tar(&mut tarball)
        } else if mime_type == "application/gzip" || mime_type == "application/x-tgz" {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut tarball = tar::Archive::new(decoder);
            self.append_tar(&mut tarball)
        } else if mime_type == "application/zstd" {
            let decoder = zstd::stream::read::Decoder::new(file)?;
            let mut tarball = tar::Archive::new(decoder);
            self.append_tar(&mut tarball)
        } else if mime_type == "application/x-xz" {
            let decoder = xz2::read::XzDecoder::new(file);
            let mut tarball = tar::Archive::new(decoder);
            self.append_tar(&mut tarball)
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "Unknown archive format"));
        }
    }

    /// Internal method to create a PackageContent object from a file and format.  This calculates
    /// the SHA256 digest of the file and sets the media type of the content.
    fn _content_from_file(
        mut file: fs::File,
        format: &PackageContentFormat,
        annotations: Option<HashMap<String, String>>,
    ) -> io::Result<PackageContent> {
        debug!("Creating package content from file for format {}", format);

        // Get the size of the temporary file
        let file_size = file.seek(io::SeekFrom::End(0))?;

        // Rewind the file to the beginning and calculate the SHA256 digest of it
        file.rewind()?;
        let (len, digest) = Self::_hash_content(&mut file)?;

        // Check the size of the file matches the amount of data we hashed
        if file_size != len {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "File size mismatch when creating package content (calculated: {}, actual: {})",
                    len, file_size
                ),
            ));
        }

        // Rewind the file to the beginning again
        file.rewind()?;

        // Create the media type based on the format
        let media_type = match format {
            PackageContentFormat::Tar => {
                MediaType::Other("application/vnd.rdk.package.content.layer.v1.tar".to_string())
            }
            PackageContentFormat::TarGz => {
                MediaType::Other("application/vnd.rdk.package.content.layer.v1.tar+gzip".to_string())
            }
            PackageContentFormat::TarZstd => {
                MediaType::Other("application/vnd.rdk.package.content.layer.v1.tar+zstd".to_string())
            }
            PackageContentFormat::ErofsUncompressed
            | PackageContentFormat::ErofsLz4
            | PackageContentFormat::ErofsZstd => {
                MediaType::Other("application/vnd.rdk.package.content.layer.v1.erofs+dmverity".to_string())
            }
        };

        debug!(
            "Built package content (media type: {}, size: {}, digest: {})",
            media_type, file_size, digest,
        );

        Ok(PackageContent {
            media_type,
            size: len,
            digest,
            file,
            annotations: annotations.unwrap_or(HashMap::new()),
        })
    }

    /// Internal method to calculate the SHA256 digest of a file.  This returns the hex encoded 32
    /// byte digest.
    fn _hash_content<R: io::Read>(content: &mut R) -> io::Result<(u64, Sha256Digest)> {
        let mut hasher = openssl::sha::Sha256::new();
        let mut buffer = vec![0; 8192];
        let mut total_bytes: u64 = 0;
        loop {
            let bytes_read = content
                .read(&mut buffer)
                .map_err(|_e| io::Error::new(io::ErrorKind::Other, "Failed to read content for hashing"))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);

            total_bytes += bytes_read as u64;
        }

        let sha256 = hasher.finish();

        let digest = Sha256Digest::from_str(hex::encode(sha256).as_str())
            .map_err(|_e| io::Error::new(io::ErrorKind::Other, "Failed to convert sha256 digest to string"))?;

        Ok((total_bytes, digest))
    }

    /// Internal method to append dm-verity hashes to the given file.  This uses the default values
    /// expected for the dm-verity hashes.
    ///
    /// Returns the dm-verity details as a HashMap of annotations.
    ///
    fn _append_dm_verity_hashes(file: &mut fs::File) -> io::Result<HashMap<String, String>> {
        // Get the size of the source file, if not a multiple of 4096 then pad with zeros to the
        // next multiple of 4096.
        let mut image_size = file.seek(io::SeekFrom::End(0))?;
        if image_size % 4096 != 0 {
            let padding = 4096 - (image_size % 4096);
            let zeros = vec![0u8; padding as usize];
            file.write_all(zeros.as_slice())?;

            image_size += padding;
        }

        // Generate a random salt
        let mut salt = [0u8; 32];
        openssl::rand::rand_bytes(&mut salt)
            .map_err(|_e| io::Error::new(io::ErrorKind::Other, "Failed to generate random salt"))?;

        // Generate a UUID for the dm-verity hash, it's not currently used, but it's a good idea to
        // have it in the future.
        let uuid = uuid::Uuid::new_v4();

        // Set the dm-verity options
        let mut options = DmVerityOptions::default();
        options.set_inc_superblock(true);
        options.set_data_block_size(4096);
        options.set_hash_block_size(4096);
        options.set_hash_algorithm("sha256".to_string());
        options.set_salt(&salt);
        options.set_uuid(&uuid);

        // Create the dm-verity hash tree and append to the file
        let root_hash = dmverity_build_and_append(file, &options)?;

        debug!(
            "Appended dm-verity hash tree to image (root hash: {}, salt: {}, size: {})",
            root_hash,
            hex::encode(salt),
            image_size,
        );

        // Return the details as annotations
        let mut annotations = HashMap::new();
        annotations.insert(ANNOTATION_DMVERITY_ROOTHASH.to_string(), root_hash);
        annotations.insert(ANNOTATION_DMVERITY_SALT.to_string(), hex::encode(salt));
        annotations.insert(ANNOTATION_DMVERITY_OFFSET.to_string(), image_size.to_string());

        Ok(annotations)
    }

    /// Finalizes the content and writes it to the given path, if the format is tar with compression
    /// then after we finalize the tar we will compress it.
    pub fn build(&mut self) -> io::Result<PackageContent> {
        if self.tar_builder.is_some() {
            debug!("Building package content tarball");

            // Take the tar builder and finish it
            let tar_builder = self.tar_builder.take().unwrap();
            let mut archive = tar_builder.into_inner()?;
            archive.rewind()?;

            // The output should be a temporary file, so can now compress it if needed otherwise
            // just copy to the final location.
            if self.options.format == PackageContentFormat::Tar {
                // Wrap the tar archive
                return Self::_content_from_file(archive, &self.options.format, None);
            } else if self.options.format == PackageContentFormat::TarGz {
                // Create a temporary file to hold the compressed tar archive
                let output = tempfile()?;

                // Create a GzEncoder to write to the output file
                let mut gz = flate2::write::GzEncoder::new(output, flate2::Compression::default());

                // Copy the tar archive to the GzEncoder
                io::copy(&mut archive, &mut gz)?;

                // Finish the GzEncoder
                let archive = gz.finish()?;

                // Wrap the gzipped tar archive
                return Self::_content_from_file(archive, &self.options.format, None);
            } else if self.options.format == PackageContentFormat::TarZstd {
                // Create a temporary file to hold the compressed tar archive
                let output = tempfile()?;

                // Create a zstd encoder to write to the output file
                let mut zstd = zstd::stream::Encoder::new(output, 0)?;

                // Copy the tar archive to the Zstd encoder
                io::copy(&mut archive, &mut zstd)?;

                // Finish the Zstd encoding
                let archive = zstd.finish()?;

                // Wrap the gzipped tar archive
                return Self::_content_from_file(archive, &self.options.format, None);
            }

            Err(io::Error::new(
                io::ErrorKind::Other,
                "Internal error - have tar builder but format is not specified as a tar archive",
            ))
        } else if self.erofs_builder.is_some() {
            debug!("Building package content EROFS image");

            // Take the erofs builder and finish it
            let mut erofs_builder = self.erofs_builder.take().unwrap();

            // Create a temporary file to hold the erofs image
            let mut temp_file = tempfile()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create temporary file: {}", e)))?;

            // Build the erofs image in the temporary file
            erofs_builder.build(&mut temp_file)?;

            debug!("Build EROFS image complete, appending dm-verity hashes");

            // Rewind the file to the beginning and then append dm-verity hashes
            temp_file.rewind()?;

            // Create the dm-verity hashes
            let annotations = Self::_append_dm_verity_hashes(&mut temp_file)?;

            debug!("Appended dm-verity hashes to EROFS image, finalizing package content");

            // Convert the erofs + dmverity image to a PackageContent object
            Self::_content_from_file(temp_file, &self.options.format, Some(annotations))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Tar builder is not initialized"))
        }
    }
}
