#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{
    __m128i, _blsr_u32, _mm_cmpestrm, _mm_cmpistrm, _mm_extract_epi32, _mm_lddqu_si128,
    _mm_loadu_si128, _mm_shuffle_epi8, _mm_storeu_si128, _mm_tzcnt_32, _popcnt32, _SIDD_BIT_MASK,
    _SIDD_CMP_EQUAL_ANY, _SIDD_UWORD_OPS,
};

#[cfg(target_arch = "aarch64")]
use std::{
    arch::aarch64::{uint16x8_t, vceqq_u16, vld1q_dup_u16, vld1q_u16, vst1q_u16},
    mem::{self},
};

use ahash::AHashSet;

use crate::{
    add_result::add_result_multiterm_multifield,
    index::{Index, NonUniquePostingListObjectQuery, PostingListObjectQuery},
    search::{FilterSparse, ResultType, SearchResult},
};

/// Fast SIMD intersection and union partially ported and modified from Daniel Lemire's CRoaring project (which is dual licensed Apache/MIT).
#[cfg(target_arch = "x86_64")]
const SHUFFLE_MASK16: [u8; 4096] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 2, 3, 6, 7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1,
    2, 3, 6, 7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1,
    8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 8, 9, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 8, 9, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 2, 3, 4, 5, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 8, 9, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 8, 9, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    4, 5, 6, 7, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7,
    8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 8, 9, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0,
    1, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 10,
    11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 10, 11,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 10, 11, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 10, 11, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 6, 7, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 6, 7, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6,
    7, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 10,
    11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0,
    1, 2, 3, 4, 5, 6, 7, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 8, 9, 10, 11, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 8, 9, 10, 11, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 4, 5, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 8,
    9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 8, 9, 10, 11,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 2, 3, 6, 7, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6,
    7, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 8, 9, 10, 11, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4,
    5, 6, 7, 8, 9, 10, 11, 0xFF, 0xFF, 0xFF, 0xFF, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 2, 3, 4, 5, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2,
    3, 4, 5, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 12, 13, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 12, 13, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 4, 5, 6, 7, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6,
    7, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 12, 13,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 2, 3, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 2, 3, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 12,
    13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 8, 9, 12, 13, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 6,
    7, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 8, 9,
    12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 8, 9, 12, 13, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 4, 5, 6, 7, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4,
    5, 6, 7, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13, 0xFF, 0xFF, 0xFF,
    0xFF, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 10, 11,
    12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 10, 11, 12, 13,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 2, 3, 4, 5, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 10, 11, 12, 13,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 10, 11, 12, 13, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    4, 5, 6, 7, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7,
    10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF,
    8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 8, 9,
    10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 8, 9, 10, 11, 12, 13,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 4, 5, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 8, 9,
    10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13,
    0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 6, 7, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 8, 9,
    10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13,
    0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    12, 13, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 0xFF, 0xFF, 14,
    15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1,
    14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 14, 15, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 14, 15, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 6, 7, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 6, 7, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6,
    7, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 14,
    15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0,
    1, 2, 3, 4, 5, 6, 7, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 8, 9, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 8, 9, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0, 1, 4, 5, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 8,
    9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 8, 9, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 2, 3, 6, 7, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6,
    7, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 8, 9, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4,
    5, 6, 7, 8, 9, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    4, 5, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5,
    10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 10, 11, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 10, 11, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0, 1, 6, 7, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3,
    6, 7, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 10, 11,
    14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    2, 3, 4, 5, 6, 7, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7,
    10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 2, 3, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1,
    2, 3, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 10, 11, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 8, 9, 10, 11, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 8, 9, 10, 11, 14,
    15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5,
    6, 7, 8, 9, 10, 11, 14, 15, 0xFF, 0xFF, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0, 1, 2, 3, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5,
    12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 12, 13,
    14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 12, 13, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 6, 7, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0, 1, 6, 7, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 12, 13,
    14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 12, 13, 14, 15, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5,
    6, 7, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 12, 13, 14,
    15, 0xFF, 0xFF, 0xFF, 0xFF, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0, 1, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    2, 3, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 8, 9,
    12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 2, 3, 4, 5, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3,
    4, 5, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 2, 3, 6, 7, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3,
    6, 7, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 8, 9, 12, 13, 14, 15, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    2, 3, 4, 5, 6, 7, 8, 9, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    12, 13, 14, 15, 0xFF, 0xFF, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0, 1, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    2, 3, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 10,
    11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 10, 11, 12, 13, 14, 15, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1,
    2, 3, 4, 5, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 6, 7, 10, 11, 12, 13, 14, 15, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1,
    2, 3, 6, 7, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF,
    0xFF, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 6, 7,
    10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0, 1, 8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3,
    8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 8, 9, 10, 11, 12,
    13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0, 1, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 4, 5, 8, 9,
    10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15,
    0xFF, 0xFF, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 6, 7,
    8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 4, 5, 6, 7,
    8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0, 1, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
    14, 15, 0xFF, 0xFF, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0xFF, 0xFF, 0, 1, 2, 3, 4,
    5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const CMPESTRM_CTRL: i32 = _SIDD_UWORD_OPS | _SIDD_CMP_EQUAL_ANY | _SIDD_BIT_MASK;

#[cfg(target_arch = "x86_64")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn intersection_vector16(
    a: &[u16],
    s_a: usize,
    b: &[u16],
    s_b: usize,
    result_count: &mut i32,
    block_id: usize,
    index: &Index,
    search_result: &mut SearchResult,
    top_k: usize,
    result_type: &ResultType,
    field_filter_set: &AHashSet<u16>,
    facet_filter: &[FilterSparse],
    non_unique_query_list: &mut [NonUniquePostingListObjectQuery],
    query_list: &mut [PostingListObjectQuery],
    not_query_list: &mut [PostingListObjectQuery],
    phrase_query: bool,
    all_terms_frequent: bool,
) {
    unsafe {
        let c = [0u16; 8];

        let mut i_a = 0;
        let mut i_b = 0;
        let vectorlength = size_of::<__m128i>() / size_of::<u16>();
        let vectorlength_i32 = vectorlength as i32;
        let st_a = (s_a / vectorlength) * vectorlength;
        let st_b = (s_b / vectorlength) * vectorlength;
        if (i_a < st_a) && (i_b < st_b) {
            let mut v_a = _mm_loadu_si128(a[i_a..].as_ptr() as *const __m128i);
            let mut v_b = _mm_loadu_si128(b[i_b..].as_ptr() as *const __m128i);

            while (a[i_a] == 0) || (b[i_b] == 0) {
                let res_v =
                    _mm_cmpestrm(v_b, vectorlength_i32, v_a, vectorlength_i32, CMPESTRM_CTRL);
                let r = _mm_extract_epi32(res_v, 0);

                let sm16 =
                    _mm_loadu_si128((SHUFFLE_MASK16.as_ptr() as *const __m128i).add(r as usize));
                let p = _mm_shuffle_epi8(v_a, sm16);
                _mm_storeu_si128(c[0..].as_ptr() as *mut __m128i, p);

                let num = _popcnt32(r);
                let mut mask = r as u32;
                let mut bit_pos2 = 0usize;
                for item in c.iter().take(num as usize) {
                    if phrase_query || (result_type != &ResultType::Count) {
                        let bit_pos = _mm_tzcnt_32(mask) as usize;
                        mask = _blsr_u32(mask);
                        while *item != b[i_b + bit_pos2] {
                            bit_pos2 += 1
                        }
                        query_list[0].p_docid = i_a + bit_pos;
                        query_list[1].p_docid = i_b + bit_pos2;
                        bit_pos2 += 1
                    }
                    add_result_multiterm_multifield(
                        index,
                        (block_id << 16) | *item as usize,
                        result_count,
                        search_result,
                        top_k,
                        result_type,
                        field_filter_set,
                        facet_filter,
                        non_unique_query_list,
                        query_list,
                        not_query_list,
                        phrase_query,
                        f32::MAX,
                        all_terms_frequent,
                    );
                }

                let a_max = a[i_a + vectorlength - 1];
                let b_max = b[i_b + vectorlength - 1];
                if a_max <= b_max {
                    i_a += vectorlength;
                    if i_a == st_a {
                        break;
                    }
                    v_a = _mm_lddqu_si128(a[i_a..].as_ptr() as *const __m128i);
                }
                if b_max <= a_max {
                    i_b += vectorlength;
                    if i_b == st_b {
                        break;
                    }
                    v_b = _mm_lddqu_si128(b[i_b..].as_ptr() as *const __m128i);
                }
            }

            if (i_a < st_a) && (i_b < st_b) {
                loop {
                    let res_v = _mm_cmpistrm(
                        v_b,
                        v_a,
                        _SIDD_UWORD_OPS | _SIDD_CMP_EQUAL_ANY | _SIDD_BIT_MASK,
                    );
                    let r = _mm_extract_epi32(res_v, 0);

                    let sm16 = _mm_loadu_si128(
                        (SHUFFLE_MASK16.as_ptr() as *const __m128i).add(r as usize),
                    );
                    let p = _mm_shuffle_epi8(v_a, sm16);
                    _mm_storeu_si128(c[0..].as_ptr() as *mut __m128i, p);

                    let num = _popcnt32(r);
                    let mut mask = r as u32;
                    let mut bit_pos2 = 0usize;
                    for item in c.iter().take(num as usize) {
                        if phrase_query || (result_type != &ResultType::Count) {
                            let bit_pos = _mm_tzcnt_32(mask) as usize;
                            mask = _blsr_u32(mask);
                            while *item != b[i_b + bit_pos2] {
                                bit_pos2 += 1
                            }
                            query_list[0].p_docid = i_a + bit_pos;
                            query_list[1].p_docid = i_b + bit_pos2;
                            bit_pos2 += 1
                        }
                        add_result_multiterm_multifield(
                            index,
                            (block_id << 16) | *item as usize,
                            result_count,
                            search_result,
                            top_k,
                            result_type,
                            field_filter_set,
                            facet_filter,
                            non_unique_query_list,
                            query_list,
                            not_query_list,
                            phrase_query,
                            f32::MAX,
                            all_terms_frequent,
                        );
                    }

                    let a_max = a[i_a + vectorlength - 1];
                    let b_max = b[i_b + vectorlength - 1];
                    if a_max <= b_max {
                        i_a += vectorlength;
                        if i_a == st_a {
                            break;
                        }
                        v_a = _mm_lddqu_si128(a[i_a..].as_ptr() as *const __m128i);
                    }
                    if b_max <= a_max {
                        i_b += vectorlength;
                        if i_b == st_b {
                            break;
                        }
                        v_b = _mm_lddqu_si128(b[i_b..].as_ptr() as *const __m128i);
                    }
                }
            }
        }
        while i_a < s_a && i_b < s_b {
            let a = a[i_a];
            let b = b[i_b];
            match a.cmp(&b) {
                std::cmp::Ordering::Less => {
                    i_a += 1;
                }
                std::cmp::Ordering::Greater => {
                    i_b += 1;
                }
                std::cmp::Ordering::Equal => {
                    query_list[0].p_docid = i_a;
                    query_list[1].p_docid = i_b;
                    add_result_multiterm_multifield(
                        index,
                        (block_id << 16) | a as usize,
                        result_count,
                        search_result,
                        top_k,
                        result_type,
                        field_filter_set,
                        facet_filter,
                        non_unique_query_list,
                        query_list,
                        not_query_list,
                        phrase_query,
                        f32::MAX,
                        all_terms_frequent,
                    );

                    i_a += 1;
                    i_b += 1;
                }
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
pub(crate) fn intersection_vector16(
    a: &[u16],
    s_a: usize,
    b: &[u16],
    s_b: usize,
    result_count: &mut i32,
    block_id: usize,
    index: &Index,
    search_result: &mut SearchResult,
    top_k: usize,
    result_type: &ResultType,
    field_filter_set: &AHashSet<u16>,
    facet_filter: &[FilterSparse],
    non_unique_query_list: &mut [NonUniquePostingListObjectQuery],
    query_list: &mut [PostingListObjectQuery],
    not_query_list: &mut [PostingListObjectQuery],
    phrase_query: bool,
    all_terms_frequent: bool,
) {
    unsafe {
        let mut i_a = 0;
        let mut i_b = 0;
        let vectorlength = mem::size_of::<uint16x8_t>() / mem::size_of::<u16>();
        let st_b = (s_b / vectorlength) * vectorlength;
        while i_a < s_a && i_b < st_b {
            if a[i_a] < b[i_b] {
                i_a += 1;
                continue;
            } else if a[i_a] > b[i_b + vectorlength - 1] {
                i_b += vectorlength;
                continue;
            }
            let v_a = vld1q_dup_u16(a[i_a..].as_ptr());
            let v_b = vld1q_u16(b[i_b..].as_ptr());
            let res_v = vceqq_u16(v_a, v_b);
            let mut res = [0u16; 8];
            vst1q_u16(res.as_mut_ptr(), res_v);
            for i in 0..res.len() {
                if res[i] == 0 {
                    continue;
                }
                query_list[0].p_docid = i_a;
                query_list[1].p_docid = i_b + i;
                add_result_multiterm_multifield(
                    index,
                    (block_id << 16) | a[i_a] as usize,
                    result_count,
                    search_result,
                    top_k,
                    result_type,
                    field_filter_set,
                    facet_filter,
                    non_unique_query_list,
                    query_list,
                    not_query_list,
                    phrase_query,
                    f32::MAX,
                    all_terms_frequent,
                );
                break;
            }
            i_a += 1;
        }
        while i_a < s_a && i_b < s_b {
            let a = a[i_a];
            let b = b[i_b];
            match a.cmp(&b) {
                std::cmp::Ordering::Less => {
                    i_a += 1;
                }
                std::cmp::Ordering::Greater => {
                    i_b += 1;
                }
                std::cmp::Ordering::Equal => {
                    query_list[0].p_docid = i_a;
                    query_list[1].p_docid = i_b;
                    add_result_multiterm_multifield(
                        index,
                        (block_id << 16) | a as usize,
                        result_count,
                        search_result,
                        top_k,
                        result_type,
                        field_filter_set,
                        facet_filter,
                        non_unique_query_list,
                        query_list,
                        not_query_list,
                        phrase_query,
                        f32::MAX,
                        all_terms_frequent,
                    );

                    i_a += 1;
                    i_b += 1;
                }
            }
        }
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(crate) fn intersection_vector16(
    a: &[u16],
    s_a: usize,
    b: &[u16],
    s_b: usize,
    result_count: &mut i32,
    block_id: usize,
    index: &Index,
    search_result: &mut SearchResult,
    top_k: usize,
    result_type: &ResultType,
    field_filter_set: &AHashSet<u16>,
    facet_filter: &[FilterSparse],
    non_unique_query_list: &mut [NonUniquePostingListObjectQuery],
    query_list: &mut [PostingListObjectQuery],
    not_query_list: &mut [PostingListObjectQuery],
    phrase_query: bool,
    all_terms_frequent: bool,
) {
    let mut i_a = 0;
    let mut i_b = 0;
    while i_a < s_a && i_b < s_b {
        let a = a[i_a];
        let b = b[i_b];
        match a.cmp(&b) {
            std::cmp::Ordering::Less => {
                i_a += 1;
            }
            std::cmp::Ordering::Greater => {
                i_b += 1;
            }
            std::cmp::Ordering::Equal => {
                query_list[0].p_docid = i_a;
                query_list[1].p_docid = i_b;
                add_result_multiterm_multifield(
                    index,
                    (block_id << 16) | a as usize,
                    result_count,
                    search_result,
                    top_k,
                    result_type,
                    field_filter_set,
                    facet_filter,
                    non_unique_query_list,
                    query_list,
                    not_query_list,
                    phrase_query,
                    f32::MAX,
                    all_terms_frequent,
                );

                i_a += 1;
                i_b += 1;
            }
        }
    }
}
