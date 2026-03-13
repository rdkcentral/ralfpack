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

use crate::erofs::erofs_sys::erofs_create_from_tarball;
use std::io::{Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::SystemTime;
use std::{fs, io};
use tempfile::tempfile;

#[derive(Clone, Copy, PartialEq)]
pub enum CompressionAlgo {
    None = 0,
    Lz4 = 1,
    Zstd = 2,
}

///
pub struct ErofsImageBuilder {
    /// Options for the image generation
    options: ErofsImageBuilderOptions,

    /// The erofs-utils C code does not support building an image from a series of files, but it
    /// does support building an image from a tar archive. So building an image is a two-step
    /// process:
    /// 1. Create a tar archive of the files to be included in the image.
    /// 2. Create the image from the tar archive.
    /// This field stores the tar builder that writes to a temporary file.
    tar_builder: Option<tar::Builder<fs::File>>,
}

#[derive(Clone, Copy)]
struct ErofsImageBuilderOptions {
    compression: CompressionAlgo,
    mtime: u64,
}

impl ErofsImageBuilder {
    /// Creates the default EROFS image builder, this sets the default compression algorithm and
    /// other configuration.
    pub fn new() -> ErofsImageBuilder {
        // By default, for mtime, use the current time in seconds since the UNIX epoch
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
        let mtime = now.map(|d| d.as_secs()).unwrap_or(0);

        // Create a temporary file for creating the intermediate tar archive
        let temp_file =
            tempfile().expect("Failed to create temporary file for EROFS image generation");

        ErofsImageBuilder {
            options: ErofsImageBuilderOptions {
                compression: CompressionAlgo::Lz4,
                mtime: mtime,
            },
            tar_builder: Some(tar::Builder::new(temp_file)),
        }
    }

    /// Sets the compression algorithm to use for the EROFS image.
    pub fn compression(&mut self, algo: CompressionAlgo) {
        self.options.compression = algo;
    }

    /// Sets the modification time for all files and directories in the EROFS image.
    #[allow(dead_code)]
    pub fn modtime(&mut self, mtime: u64) {
        self.options.mtime = mtime;
    }

    /// Appends a file to the EROFS image with the given path, mode and data.
    ///
    /// This just adds the file to the tar archive, the actual EROFS image is built when the
    /// `build` method is called.
    pub fn append_data<P: AsRef<Path>, R: Read>(
        &mut self,
        path: P,
        data: R,
        size: usize,
        mode: u32,
    ) -> io::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mtime(self.options.mtime);
        header.set_mode(mode);
        header.set_size(size as u64);
        header.set_uid(0);
        header.set_gid(0);
        header.set_cksum();

        if let Some(tar_builder) = self.tar_builder.as_mut() {
            tar_builder.append_data(&mut header, path, data)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Builder already finished",
            ))
        }
    }

    /// Appends a symlink to the EROFS image with the given path and target.
    ///
    /// This just adds the symlink to the tar archive, the actual EROFS image is built when the
    /// `build` method is called.
    pub fn append_link<P: AsRef<Path>, T: AsRef<Path>>(
        &mut self,
        path: P,
        target: T,
    ) -> io::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_mtime(self.options.mtime);
        header.set_mode(0o777);
        header.set_uid(0);
        header.set_gid(0);
        header.set_size(0);

        if let Some(tar_builder) = self.tar_builder.as_mut() {
            tar_builder.append_link(&mut header, path, target)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Builder already finished",
            ))
        }
    }

    /// Appends a directory to the EROFS image at the given path.
    ///
    /// This just adds the directory to the tar archive, the actual EROFS image is built when the
    /// `build` method is called.
    pub fn append_dir<P: AsRef<Path>>(&mut self, path: P, mode: u32) -> io::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_mtime(self.options.mtime);
        header.set_mode(mode);
        header.set_size(0);
        header.set_uid(0);
        header.set_gid(0);
        header.set_cksum();

        // Empty data for the directory
        let data: &[u8] = &[];

        if let Some(tar_builder) = self.tar_builder.as_mut() {
            tar_builder.append_data(&mut header, path, data)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Builder already finished",
            ))
        }
    }

    /// Builds the actual EROFS image. This is where the tar archive we've built up is given to
    /// erofs-utils C code to generate us a EROFS image.
    ///
    ///
    ///
    pub fn build(&mut self, output: &mut fs::File) -> io::Result<()> {
        // Rewind and truncate the output file to 0 bytes
        output.rewind()?;
        output.set_len(0)?;

        // Finish writing the tar archive
        let tar_builder = self.tar_builder.take();
        if tar_builder.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Builder already finished",
            ));
        }

        let mut tar_file = tar_builder.unwrap().into_inner()?;
        tar_file.flush()?;

        // Rewind the tar file and get the raw fd, this is needed for the erofs-utils C code
        tar_file.rewind()?;
        let tar_fd = tar_file.as_raw_fd();

        // Get the output object fd
        let img_fd = output.as_raw_fd();

        // Build the EROFS image from the tar archive
        erofs_create_from_tarball(tar_fd, img_fd, self.options.compression)?;

        // Flush and rewind the output file
        output.flush()?;
        output.rewind()?;

        Ok(())
    }
}
