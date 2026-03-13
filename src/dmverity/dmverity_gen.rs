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

use hex;
use log::*;
use positioned_io::{ReadAt, WriteAt};
use std::fs;
use std::io;
use std::io::*;
use std::mem;
use uuid::Uuid;

#[repr(C, packed)]
struct _VeritySuperBlock {
    signature: [u8; 8],   // "verity\0\0"
    version: u32,         // superblock version
    hash_type: u32,       // 0 - Chrome OS, 1 - normal
    uuid: [u8; 16],       // UUID of hash device
    algorithm: [u8; 32],  // hash algorithm name
    data_block_size: u32, // data block in bytes
    hash_block_size: u32, // hash block in bytes
    data_blocks: u64,     // number of data blocks
    salt_size: u16,       // salt size
    _pad1: [u8; 6],
    salt: [u8; 256], // salt
    _pad2: [u8; 168],
}

/// The size of the SHA256 digest in bytes
const SHA256_DIGEST_LENGTH: usize = 32;

pub struct DmVerityOptions {
    inc_superblock: bool,
    data_block_size: u64,
    hash_block_size: u64,
    hash_algorithm: String,
    salt: Option<Vec<u8>>,
    uuid: Option<[u8; 16]>,
}

impl DmVerityOptions {
    /// Creates a new set of default dm-verity options.
    pub fn default() -> DmVerityOptions {
        DmVerityOptions {
            inc_superblock: true,
            data_block_size: 4096,
            hash_block_size: 4096,
            hash_algorithm: "sha256".to_string(),
            salt: None,
            uuid: None,
        }
    }

    /// Set to true to include the superblock at the start of the hash block.  This is not required
    /// and the kernel doesn't use this, however it is used on the device side to get the details
    /// to pass to the dm-verity mapper code when mounting the image.
    ///
    /// We could ignore the super block and instead include the information as annotations on the
    /// image file, but by keeping the standard superblock it's more consistent with existing code
    /// and tools that use dm-verity.
    ///
    pub fn set_inc_superblock(&mut self, inc_superblock: bool) {
        self.inc_superblock = inc_superblock;
    }

    /// Sets the data block size, currently only 4096 is supported.
    pub fn set_data_block_size(&mut self, block_size: u32) {
        self.data_block_size = block_size as u64;
    }

    /// Sets the hash block size, currently only 4096 is supported.
    pub fn set_hash_block_size(&mut self, block_size: u32) {
        self.hash_block_size = block_size as u64;
    }

    /// Set the hash algorithm to use, currently we only support "sha256".
    pub fn set_hash_algorithm(&mut self, algorithm: String) {
        self.hash_algorithm = algorithm;
    }

    /// Sets the Salt for the verity calculations.  A salt is not required but should be used to
    /// enhance the security of the hash.
    pub fn set_salt(&mut self, salt: &[u8]) {
        if salt.len() > 256 {
            panic!("Salt size exceeds 256 bytes");
        }
        self.salt = Some(salt.to_vec());
    }

    /// Sets the UUID for the verity superblock.
    ///
    /// This is not currently used, but we allow to set it as it could come in handy in the future
    /// for identifying a package from the devmapper.
    ///
    pub fn set_uuid(&mut self, uuid: &Uuid) {
        let mut uuid_bytes = [0; 16];
        uuid_bytes.copy_from_slice(uuid.as_bytes());
        self.uuid = Some(uuid_bytes);
    }
}

struct HashLayer {
    blocks: u64,
    write_offset: u64,
}

const fn div_round_up(dividend: u64, divisor: u64) -> u64 {
    (dividend + divisor - 1) / divisor
}

const fn round_up(dividend: u64, divisor: u64) -> u64 {
    div_round_up(dividend, divisor) * divisor
}

/// This resizes the file to the specified size, if the file size is increased it will be filled
/// with zeros.
fn _resize_file(file: &mut fs::File, size: u64) -> io::Result<()> {
    let cur_size = file.seek(io::SeekFrom::End(0))?;
    if cur_size < size {
        // Increasing the file size, we don't use truncate as that can leave holes in the file,
        // and we want to fill the file with zeros.
        let buffer = [0u8; 4096];
        let padding = size - cur_size;
        let mut written = 0;

        while written < padding {
            let bytes_to_write = std::cmp::min(buffer.len() as u64, padding - written) as usize;
            file.write_all(&buffer[..bytes_to_write])?;
            written += bytes_to_write as u64;
        }

        Ok(())
    } else if cur_size > size {
        // Decreasing the file size, we can use truncate
        file.set_len(size)
    } else {
        // No change needed
        Ok(())
    }
}

/// Populates and writes the superblock to the file at the specified offset.
///
pub fn _write_superblock_at(
    file: &mut fs::File,
    file_offset: u64,
    options: &DmVerityOptions,
    data_block_count: u64,
) -> io::Result<()> {
    // Populate the salt with options.salt
    let mut salt = [0; 256];
    let mut salt_size: u16 = 0;
    if let Some(s) = options.salt.as_ref() {
        salt[..s.len()].copy_from_slice(s);
        salt_size = s.len() as u16;
    }

    // Populate the hash algorithm
    let mut algorithm = [0; 32];
    algorithm[..options.hash_algorithm.len()].copy_from_slice(options.hash_algorithm.as_bytes());

    // Create the packed superblock structure
    let superblock = _VeritySuperBlock {
        signature: *b"verity\0\0",
        version: 1,
        hash_type: 1,
        uuid: options.uuid.unwrap_or([0; 16]),
        algorithm: algorithm,
        data_block_size: options.data_block_size as u32,
        hash_block_size: options.hash_block_size as u32,
        data_blocks: data_block_count,
        salt_size: salt_size,
        _pad1: [0; 6],
        salt: salt,
        _pad2: [0; 168],
    };

    assert_eq!(512, mem::size_of::<_VeritySuperBlock>());

    // TODO: remove the unsafe part and uses something better to write the superblock
    file.write_all_at(file_offset, unsafe {
        std::slice::from_raw_parts(
            &superblock as *const _ as *const u8,
            std::mem::size_of::<_VeritySuperBlock>(),
        )
    })
}

/// Calculate the SHA256 hash of the salt + data
fn _sha256_with_salt(salt: &Option<Vec<u8>>, data: &[u8]) -> [u8; SHA256_DIGEST_LENGTH] {
    let mut hasher = openssl::sha::Sha256::new();

    if let Some(salt) = salt {
        hasher.update(salt);
    }

    hasher.update(data);
    hasher.finish()
}

/// This reads blocks from file at read_offset and calculates the sha256 hash of the block, it
/// then writes that hash into the file at write_offset.
///
///
///
fn _hash_file_range(
    options: &DmVerityOptions,
    file: &mut fs::File,
    mut read_offset: u64,
    mut read_blocks: u64,
    read_block_size: u64,
    mut write_offset: u64,
) -> io::Result<()> {
    debug!(
        "Hashing file range: read_offset=0x{:x}, read_blocks={}, read_block_size=0x{:x}, write_offset=0x{:x}",
        read_offset, read_blocks, read_block_size, write_offset
    );

    // Create a buffer to hold the data block
    let mut buffer = vec![0; read_block_size as usize];

    while read_blocks > 0 {
        // Read a data block from the file
        file.read_exact_at(read_offset, buffer.as_mut_slice())?;

        // Calculate the hash of the data block
        let digest = _sha256_with_salt(&options.salt, &buffer);

        // Write the hash to the file at the specified offset
        file.write_all_at(write_offset, digest.as_slice())?;

        write_offset += digest.len() as u64;
        read_offset += read_block_size;
        read_blocks -= 1;
    }

    Ok(())
}

/// Simply performs a sha256 hash over a block in the supplied file
fn _hash_data_block(options: &DmVerityOptions, file: &fs::File, offset: u64, size: u64) -> io::Result<[u8; 32]> {
    let mut buffer = vec![0; size as usize];
    file.read_exact_at(offset, buffer.as_mut_slice())?;
    let digest = _sha256_with_salt(&options.salt, &buffer);
    Ok(digest)
}

/// This function builds the dm-verity hash tree and appends it to the file.
///
///
pub fn dmverity_build_and_append(file: &mut fs::File, options: &DmVerityOptions) -> io::Result<String> {
    // For now, we only support sha256 as hash algorithm
    if options.hash_algorithm != "sha256" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Only sha256 is supported"));
    }

    // The file needs to be a multiple of the options.data_block_size and the hashes are calculated
    // over any padding bytes, so fill the padding with 0 rather than using ftruncate which can
    // leave a hole in the file.
    let mut data_size = file.seek(io::SeekFrom::End(0))?;
    if data_size % options.data_block_size != 0 {
        data_size = round_up(data_size, options.data_block_size);
        _resize_file(file, data_size)?;
    }

    // Calculate the number of hash blocks needed to cover the image size.
    let data_block_count = data_size / options.data_block_size;

    // There is a special case where the data image is less than or equal to options.data_block_size
    // and therefore don't need to store a hash tree as the root hash is just the hash of the
    // single data block
    let total_file_size: u64;
    let mut hash_layers: Vec<HashLayer> = Vec::new();
    if data_block_count == 1 {
        // In this case we just append the superblock if requested
        if options.inc_superblock {
            total_file_size = data_size + 4096;
        } else {
            total_file_size = data_size;
        }
    } else {
        // Calculate the number hashes that will fit in a hash block
        let hashes_per_block = options.hash_block_size / SHA256_DIGEST_LENGTH as u64;

        // Level 0 hash blocks - this is the level at the bottom of the hash tree
        let mut level_hash_blocks = div_round_up(data_block_count, hashes_per_block);
        hash_layers.push(HashLayer {
            blocks: level_hash_blocks,
            write_offset: 0,
        });

        // The other levels, the write offset gets added later, once we know how many blocks
        // we need for each level.
        while level_hash_blocks > 1 {
            level_hash_blocks = div_round_up(level_hash_blocks, hashes_per_block);
            hash_layers.push(HashLayer {
                blocks: level_hash_blocks,
                write_offset: 0,
            });
        }

        // Calculate the file offsets, we have the number of hash blocks in each layer, ordered
        // from the bottom of the tree.  So iterate backwards through the layers so calculate the
        // file offset from the top of the tree.
        let mut level = (hash_layers.len() - 1) as i64;
        let mut write_offset = data_size;
        if options.inc_superblock {
            write_offset += 4096;
        }

        for it in hash_layers.iter_mut().rev() {
            it.write_offset = write_offset;
            write_offset += it.blocks * options.hash_block_size;

            info!("hash layer {} : 0x{:06x} : {}", level, it.write_offset, it.blocks);
            level -= 1;
        }

        // The final offset is the total file size
        total_file_size = write_offset;
    }

    debug!(
        "Resizing file to 0x{:x} to include dm-verity hashes and optional superblock",
        total_file_size
    );

    // Now resize the file once again so it includes the hash tree and optional superblock
    _resize_file(file, total_file_size)?;

    // Write the superblock if requested
    if options.inc_superblock {
        _write_superblock_at(file, data_size, options, data_block_count)?;
        debug!("Written dm-verity supperblock");
    }

    //
    let root_hash: [u8; SHA256_DIGEST_LENGTH];

    // There is a special case where if there is only one data block then we just generate the
    // superblock and return the hash of that single data block
    if hash_layers.is_empty() {
        root_hash = _hash_data_block(options, file, 0, options.data_block_size)?;
    } else {
        // Write the level 0 data blocks, this is the hashes of the actual data blocks
        _hash_file_range(
            options,
            file,
            0,
            data_block_count,
            options.data_block_size,
            hash_layers[0].write_offset,
        )?;

        // Now need to build the hash tree by iterating through the hash layers and calculating the
        // hashes of previous layer hash blocks.
        for i in 1..hash_layers.len() {
            let read_offset = hash_layers[i - 1].write_offset;
            let read_blocks = hash_layers[i - 1].blocks;
            let write_offset = hash_layers[i].write_offset;

            _hash_file_range(
                options,
                file,
                read_offset,
                read_blocks,
                options.hash_block_size,
                write_offset,
            )?;
        }

        // Finally generate the hash of the last level written
        let top_level = hash_layers.last().unwrap();
        assert_eq!(top_level.blocks, 1);
        root_hash = _hash_data_block(options, file, top_level.write_offset, options.hash_block_size)?;
    }

    // Return the root hash as a hex string
    Ok(hex::encode(root_hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to calculate the SHA256 of an open file
    fn calculate_sha256(file: &mut fs::File) -> String {
        let mut buffer = vec![0; 4096];
        let mut hasher = openssl::sha::Sha256::new();

        file.rewind().expect("failed to rewind file");

        #[allow(unused_variables)]
        let mut total_bytes_read = 0;

        loop {
            let bytes_read = file.read(&mut buffer).expect("failed to read file");
            if bytes_read == 0 {
                break;
            }

            total_bytes_read += bytes_read;
            hasher.update(&buffer[..bytes_read]);
        }

        // println!("total bytes read: {}", total_bytes_read);
        hex::encode(hasher.finish())
    }

    /// Not really a test, just a checker to ensure that the repo has been checked out with LFS
    /// enabled.  This is a common cause of these tests failing, because if you don't checkout with
    /// LFS enabled then the test data files will be just small text files with the LFS pointer
    /// rather than the actual binary files.
    #[test]
    fn test_lfs_checkout() {
        // Get the directory of the current source file
        let source_dir = std::path::Path::new(file!()).parent().unwrap();
        let test_data_dir = source_dir.join("../../testdata/dmverity/");

        // Check if the large.img file exists and is the correct size
        let large_img_path = test_data_dir.join("large.img");
        assert!(
            large_img_path.exists() && large_img_path.metadata().unwrap().len() == 29634560,
            "testdata/dmverity/large.img file does not exist or is the wrong size, please check if git LFS was enabled on checkout"
        );
    }

    /// Basic tests dm-verity "formatting" various file sizes and options.
    #[test]
    fn test_dmverity_format_and_append() {
        // Get the directory of the current source file
        let source_dir = std::path::Path::new(file!()).parent().unwrap();
        let test_data_dir = source_dir.join("../../testdata/dmverity/");

        struct TestCase {
            data_file_path: std::path::PathBuf,
            salt: Vec<u8>,
            uuid: Uuid,
            expected_root_hash: String,
            expected_appended_file_path: std::path::PathBuf,
        }
        let mut test_cases: Vec<TestCase> = Vec::new();

        // Command line to generate the file to compare against:
        //      cp small.img small.img.append.hashes
        //      veritysetup \
        //          -v --debug \
        //          --salt 33e1c0d53d92d3c85de31debc3b5e1958c3e384b8a23f35fcdf8a096cae88264 \
        //          --uuid 59168122-ea74-46d5-bf04-63df8ff1dded \
        //          --hash-offset=4096 \
        //          format small.img.append.hashes small.img.append.hashes
        test_cases.push(TestCase {
            data_file_path: test_data_dir.join("small.img").to_owned(),
            salt: hex::decode("33e1c0d53d92d3c85de31debc3b5e1958c3e384b8a23f35fcdf8a096cae88264").unwrap(),
            uuid: Uuid::parse_str("59168122-ea74-46d5-bf04-63df8ff1dded").unwrap(),
            expected_root_hash: "836dea5b8d911e2b4cf71c6f6f03f9620c5f0bc148881ad76ff6433455d5ed9d".to_string(),
            expected_appended_file_path: test_data_dir.join("small.img.append.hashes"),
        });

        // Command line to generate the file to compare against:
        //      cp medium.img medium.img.append.hashes
        //      veritysetup \
        //          -v --debug \
        //          --salt e6d4b34f353d6bf18b416159f380a627f28c696b1fde8c4394eb2bd778fa87d8 \
        //          --uuid aaa1d523-e31c-4a25-b5dd-aebff2af9f00 \
        //          --hash-offset=532480 \
        //          format medium.img.append.hashes medium.img.append.hashes
        test_cases.push(TestCase {
            data_file_path: test_data_dir.join("medium.img").to_owned(),
            salt: hex::decode("e6d4b34f353d6bf18b416159f380a627f28c696b1fde8c4394eb2bd778fa87d8").unwrap(),
            uuid: Uuid::parse_str("aaa1d523-e31c-4a25-b5dd-aebff2af9f00").unwrap(),
            expected_root_hash: "3ebf735436f71566d54700ba32e2dfb5595cdfa8abd90529c334b8f55b7de8e9".to_string(),
            expected_appended_file_path: test_data_dir.join("medium.img.append.hashes"),
        });

        // Command line to generate the file to compare against:
        //      cp large.img large.img.append.hashes
        //      veritysetup \
        //          -v --debug \
        //          --salt a3fd1fddf66c61a740be5478be35d3b818514d2aac05a7c668bd1ad4448da47e \
        //          --uuid 818a6d55-8635-45c9-8122-84a18d4c9dd1 \
        //          --hash-offset=29634560 \
        //          format large.img.append.hashes large.img.append.hashes
        test_cases.push(TestCase {
            data_file_path: test_data_dir.join("large.img").to_owned(),
            salt: hex::decode("a3fd1fddf66c61a740be5478be35d3b818514d2aac05a7c668bd1ad4448da47e").unwrap(),
            uuid: Uuid::parse_str("818a6d55-8635-45c9-8122-84a18d4c9dd1").unwrap(),
            expected_root_hash: "55645d1b26b8e859ecb7dab0411ed205548be285edee6d58f1f282caabe62e1d".to_string(),
            expected_appended_file_path: test_data_dir.join("large.img.append.hashes"),
        });

        // Loop through the test cases and run the dmverity_build_and_append function
        for test_case in test_cases {
            // Get the data file and make a temporary file copy so can append the hash tree to it
            let mut file = fs::File::open(&test_case.data_file_path).expect("Failed to open data file");
            let mut expected_file =
                fs::File::open(&test_case.expected_appended_file_path).expect("Failed to open expected appended file");
            let mut temp_file = tempfile::tempfile().expect("Failed to create temp file");
            io::copy(&mut file, &mut temp_file).expect("Failed to copy data file");

            let mut options = DmVerityOptions::default();
            options.set_salt(test_case.salt.as_slice());
            options.set_uuid(&test_case.uuid);
            options.set_data_block_size(4096);
            options.set_hash_block_size(4096);
            options.set_hash_algorithm("sha256".to_string());

            // Build the dm-verity hash tree and append it to the file
            let root_hash = dmverity_build_and_append(&mut temp_file, &options).expect("Failed to build dm-verity");

            // Check if the root hash matches the expected value
            assert_eq!(
                root_hash,
                test_case.expected_root_hash,
                "Root hash mismatch for {}",
                test_case.data_file_path.display()
            );

            // Check if the hash file exists and matches the expected hash file
            let actual_hash_file = calculate_sha256(&mut temp_file);
            let expected_hash_file = calculate_sha256(&mut expected_file);
            assert_eq!(
                actual_hash_file,
                expected_hash_file,
                "Expected file doesn't match actual generated file: {}",
                test_case.expected_appended_file_path.display()
            );
        }
    }
}
