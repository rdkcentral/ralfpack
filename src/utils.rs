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

use log::warn;
use std::path::Path;
use std::{fs, io};

/// Helper function to get the decompressed size of a zip archive.
fn zip_decompressed_size(mut archive: zip::read::ZipArchive<fs::File>) -> Option<u64> {
    let mut total_size = 0u64;
    for i in 0..archive.len() {
        let file = archive.by_index(i).ok()?;
        total_size = total_size.checked_add(file.size())?;
    }
    Some(total_size)
}

/// Calculates the total extracted size of an archive file (tar or zip).
/// This is used to estimate the size of the EROFS image that will be created from
/// the archive.
pub fn archive_extracted_size<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    // Get the type of the file
    let info = infer::Infer::new();
    let file_type = info.get_from_path(&path)?;
    if file_type.is_none() {
        return Err(io::Error::new(io::ErrorKind::Other, "Unknown archive format"));
    }

    let mime_type = file_type.unwrap().mime_type();

    // Open the archive file
    let file = fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    // Check if the type is one we support
    if mime_type == "application/zip" {
        let archive = zip::read::ZipArchive::new(file)?;
        let size = zip_decompressed_size(archive);
        if let Some(size) = size {
            Ok(size as u64)
        } else {
            warn!("Zip archive does not have decompressed size, using file size instead");
            Ok(file_size)
        }
    } else if mime_type == "application/x-tar" {
        Ok(file_size)
    } else if mime_type == "application/gzip" || mime_type == "application/x-tgz" {
        let mut decoder = flate2::read::GzDecoder::new(file);
        let size = io::copy(&mut decoder, &mut io::sink())?;
        Ok(size as u64)
    } else if mime_type == "application/zstd" {
        let mut decoder = zstd::stream::read::Decoder::new(file)?;
        let size = io::copy(&mut decoder, &mut io::sink())?;
        Ok(size as u64)
    } else if mime_type == "application/x-xz" {
        let mut decoder = xz2::read::XzDecoder::new(file);
        let size = io::copy(&mut decoder, &mut io::sink())?;
        Ok(size as u64)
    } else {
        return Err(io::Error::new(io::ErrorKind::Other, "Unknown archive format"));
    }
}
