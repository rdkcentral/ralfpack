/*
 * If not stated otherwise in this file or this component's LICENSE file the
 * following copyright and licenses apply:
 *
 * Copyright 2025 Comcast Cable Communications Management, LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <stdlib.h>

#include <erofs/config.h>
#include <erofs/io.h>
#include <erofs/err.h>
#include <erofs/tar.h>
#include <erofs/print.h>
#include <erofs/inode.h>
#include <erofs/cache.h>
#include <erofs/xattr.h>
#include <erofs/dedupe.h>
#include <erofs/exclude.h>
#include <erofs/compress.h>
#include <erofs/compress_hints.h>
#include <erofs/diskbuf.h>
#include <erofs/internal.h>
#include <erofs/block_list.h>

// This is hack to work around the missing erofs_algorithm structure definition in the public headers.
// The actual definition is in <erofs-utils>/include/erofs/compressor.h
struct erofs_algorithm {
        char *name;
        const struct erofs_compressor *c;
        unsigned int id;

        /* its name won't be shown as a supported algorithm */
        bool optimisor;
};
