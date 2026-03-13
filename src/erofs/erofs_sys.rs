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

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use crate::erofs::erofs_image::CompressionAlgo;
use libc::c_int;
use log::*;
use std::ffi::CString;
use std::io;
use std::os::fd::RawFd;
use std::os::raw::c_void;
use std::time::SystemTime;
use uuid::Uuid;

pub fn erofs_global_config(algo: CompressionAlgo) -> io::Result<()> {
    let fs_uuid = Uuid::new_v4();

    let fs_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
    let fs_time_secs = fs_time.map(|d| d.as_secs()).unwrap_or(0);

    unsafe {
        // the erofs-utils uses two global variables / structures defined in <erofs-utils>/config.c
        //  struct erofs_configure cfg;
        //  struct erofs_sb_info sbi;

        // set initial config
        erofs_init_configure();

        // TODO: redirect logging to the rust log crate
        match log::max_level() {
            LevelFilter::Off => cfg.c_dbg_lvl = EROFS_MSG_MIN as c_int,
            LevelFilter::Error => cfg.c_dbg_lvl = EROFS_ERR as c_int,
            LevelFilter::Warn => cfg.c_dbg_lvl = EROFS_WARN as c_int,
            LevelFilter::Info => cfg.c_dbg_lvl = EROFS_INFO as c_int,
            LevelFilter::Debug => cfg.c_dbg_lvl = EROFS_DBG as c_int,
            LevelFilter::Trace => cfg.c_dbg_lvl = EROFS_MSG_MAX as c_int,
        }

        // don't set a source path as we'll be reading from a tar file
        cfg.c_src_path = std::ptr::null_mut();

        // set our custom config
        cfg.c_showprogress = false;
        cfg.c_legacy_compress = false;
        cfg.c_inline_data = true;
        cfg.c_ignore_mtime = true;
        cfg.c_xattr_name_filter = false;

        // force all files to be owned by 'root'
        cfg.c_uid = 0;
        cfg.c_gid = 0;

        // don't yet support compressed tail packing
        cfg.c_ztailpacking = false;

        // don't yet support deduping of compressed data
        cfg.c_dedupe = false;
        cfg.c_fragments = false;

        // disable xattr support, s/w extractor doesn't yet support it
        cfg.c_xattr_name_filter = false;
        cfg.c_inline_xattr_tolerance = -1;

        // set a 4k pcluster size
        cfg.c_mkfs_pclustersize_max = 4096;
        cfg.c_mkfs_pclustersize_def = cfg.c_mkfs_pclustersize_max;

        // enable compression if requested
        if algo == CompressionAlgo::Lz4 {
            cfg.c_compr_opts[0].alg = libc::strdup(CString::new("lz4hc").unwrap().as_ptr());
            cfg.c_compr_opts[0].level = -1;
        } else if algo == CompressionAlgo::Zstd {
            cfg.c_compr_opts[0].alg = libc::strdup(CString::new("zstd").unwrap().as_ptr());
            cfg.c_compr_opts[0].level = -1; // 12 is max compression
        } else {
            cfg.c_compr_opts[0].alg = std::ptr::null_mut();
            cfg.c_compr_opts[0].level = 0;
        }

        g_sbi.blkszbits = 12; // 4096 block size
        g_sbi.feature_incompat = EROFS_FEATURE_INCOMPAT_ZERO_PADDING;
        g_sbi.feature_compat = EROFS_FEATURE_COMPAT_SB_CHKSUM | EROFS_FEATURE_COMPAT_MTIME;

        // generate a unique UUID, not that we use it
        let uuid_bytes = fs_uuid.into_bytes();
        g_sbi.uuid = uuid_bytes;

        // set the build time
        g_sbi.build_time = fs_time_secs;
        g_sbi.build_time_nsec = 0;

        Ok(())
    }
}

pub fn erofs_create_from_tarball(
    src_tarball_fd: RawFd,
    dst_erofs_fd: RawFd,
    compression_algo: CompressionAlgo,
) -> io::Result<()> {
    debug!("Creating EROFS image from tarball...");

    unsafe {
        // The erofs-utils code uses two global variables / structures defined in <erofs-utils>/config.c
        //  struct erofs_configure cfg;
        //  struct erofs_sb_info sbi;

        // Set initial config
        erofs_global_config(compression_algo)?;

        // This is a cutdown version of the erofs_dev_open() C function, which allows us to
        // write to dst_erofs_fd file descriptor
        {
            // This will leak memory, but we don't care as this is a one-off process
            g_sbi.devname = libc::strdup(CString::new("tempfile").unwrap().as_ptr());

            // The following is equivalent to this C code:
            //  g_sbi.bdev.fd = fd;
            g_sbi.bdev.__bindgen_anon_1.__bindgen_anon_1.fd = dst_erofs_fd;
        }

        // Create a new erofs_iostream structure which is used to read from the tarball
        let mut src_tarfile: erofs_tarfile = std::mem::zeroed();
        src_tarfile.global.xattrs.next = &mut src_tarfile.global.xattrs;
        src_tarfile.global.xattrs.prev = src_tarfile.global.xattrs.next;
        src_tarfile.dev = 0;
        src_tarfile.fd = -1;
        src_tarfile.index_mode = false;
        src_tarfile.headeronly_mode = false;
        src_tarfile.rvsp_mode = false;
        src_tarfile.aufs = false;
        src_tarfile.ddtaridx_mode = false;
        src_tarfile.try_no_reorder = false;
        src_tarfile.ios.dumpfd = -1;
        src_tarfile.ios.feof = false;

        let mut err = erofs_iostream_open(&mut src_tarfile.ios, src_tarball_fd, EROFS_IOS_DECODER_NONE as c_int);
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to open tarball"));
        }

        // Initialise the buffer manager
        g_sbi.bmgr = erofs_buffer_init(&raw mut g_sbi, 0);
        if g_sbi.bmgr == std::ptr::null_mut() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to initialize buffer manager",
            ));
        }

        let buffer_head = erofs_reserve_sb(g_sbi.bmgr);
        if IS_ERR(buffer_head as *const c_void) != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to get buffer header"));
        }

        // More boilerplate
        err = erofs_diskbuf_init(1);
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to initialize disk buffer"));
        }

        // Setup the compression options
        err = erofs_load_compress_hints(&raw mut g_sbi);
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to load compress hints"));
        }

        err = z_erofs_compress_init(&raw mut g_sbi, buffer_head);
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to initialize compressor"));
        }

        erofs_inode_manager_init();

        let root_inode = erofs_rebuild_make_root(&raw mut g_sbi);
        if IS_ERR(root_inode as *const c_void) != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to create root inode"));
        }

        err = loop {
            let e = tarerofs_parse_tar(root_inode, &mut src_tarfile);
            if e != 0 {
                break e;
            }
        };
        //while err = tarerofs_parse_tar(root_inode, mut src_stream) == 0 {
        //    continue;
        // }

        if err < 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to parse tarball"));
        }

        err = erofs_rebuild_dump_tree(root_inode, false);
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to finalise EROFS image"));
        }

        g_sbi.primarydevice_blocks = erofs_mapbh(g_sbi.bmgr, std::ptr::null_mut()) as u64;
        err = erofs_write_device_table(&raw mut g_sbi);
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to write device table"));
        }

        // Flush all buffers except for the superblock
        err = erofs_bflush(g_sbi.bmgr, std::ptr::null_mut());
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to flush all buffers"));
        }

        erofs_fixup_root_inode(root_inode);
        erofs_iput(root_inode);
        // root_inode = std::ptr::null_mut();

        err = erofs_writesb(&raw mut g_sbi, buffer_head);
        if err != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to write superblock"));
        }

        // Flush all remaining buffers
        err = erofs_bflush(g_sbi.bmgr, std::ptr::null_mut());
        if err != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to flush remaining buffers",
            ));
        }

        let nblocks: erofs_blk_t = g_sbi.primarydevice_blocks as erofs_blk_t;
        err = erofs_dev_resize(&raw mut g_sbi, nblocks);
        if err != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to resize output image file",
            ));
        }

        let mut crc: u32 = 0;
        if erofs_sb_has_sb_chksum(&raw mut g_sbi) {
            err = erofs_enable_sb_chksum(&raw mut g_sbi, &mut crc);
            if err != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to write superblock checksum",
                ));
            }

            info!("superblock checksum {:#x} written", crc);
        }
    }

    Ok(())
}

pub fn erofs_get_version() -> &'static str {
    unsafe {
        let version_cstr = std::ffi::CStr::from_ptr(cfg.c_version);
        version_cstr.to_str().unwrap_or("unknown")
    }
}

pub fn erofs_get_available_compressors() -> Vec<String> {
    let mut compressors = Vec::new();

    unsafe {
        let mut idx = 0;

        loop {
            let alg_ptr = z_erofs_list_available_compressors(&mut idx);
            if alg_ptr.is_null() {
                break;
            }

            let alg_cstr = std::ffi::CStr::from_ptr((*alg_ptr).name);
            if let Ok(alg_str) = alg_cstr.to_str() {
                compressors.push(alg_str.to_string());
            }
        }
    }

    compressors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_erofs_config() {
        unsafe {
            erofs_init_configure();

            let version = std::ffi::CStr::from_ptr(cfg.c_version);
            assert_eq!(version, c"1.8.10");
        }
    }

    #[test]
    fn test_erofs_global_config() {
        let result = erofs_global_config(CompressionAlgo::Lz4);
        assert!(result.is_ok());
    }

    #[test]
    fn test_available_compressors() {
        let result = erofs_get_available_compressors();
        assert!(result.contains(&"lz4".to_string()));
        assert!(result.contains(&"lz4hc".to_string()));
        assert!(result.contains(&"zstd".to_string()));
    }
}
