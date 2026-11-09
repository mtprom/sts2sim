//! The two `xxHash` variants the build hashes with.
//!
//! `XXH64` turns strings into seeds: the run seed, every named stream, and
//! every content-local stream are offset by the 64-bit hash of a UTF-8 name.
//! `XXH32` is what the model-ID serialization cache folds its inventory
//! into. Both are the reference algorithms with a zero seed by default; the
//! build calls the .NET `System.IO.Hashing` implementations, which follow
//! the reference exactly.

const PRIME64_1: u64 = 0x9E37_79B1_85EB_CA87;
const PRIME64_2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const PRIME64_3: u64 = 0x1656_67B1_9E37_79F9;
const PRIME64_4: u64 = 0x85EB_CA77_C2B2_AE63;
const PRIME64_5: u64 = 0x27D4_EB2F_1656_67C5;

const PRIME32_1: u32 = 0x9E37_79B1;
const PRIME32_2: u32 = 0x85EB_CA77;
const PRIME32_3: u32 = 0xC2B2_AE3D;
const PRIME32_4: u32 = 0x27D4_EB2F;
const PRIME32_5: u32 = 0x1656_67B1;

fn read_u64(bytes: &[u8]) -> u64 {
    let mut word = [0_u8; 8];
    word.copy_from_slice(&bytes[..8]);
    u64::from_le_bytes(word)
}

fn read_u32(bytes: &[u8]) -> u32 {
    let mut word = [0_u8; 4];
    word.copy_from_slice(&bytes[..4]);
    u32::from_le_bytes(word)
}

const fn round64(accumulator: u64, input: u64) -> u64 {
    accumulator
        .wrapping_add(input.wrapping_mul(PRIME64_2))
        .rotate_left(31)
        .wrapping_mul(PRIME64_1)
}

const fn merge64(hash: u64, lane: u64) -> u64 {
    (hash ^ round64(0, lane))
        .wrapping_mul(PRIME64_1)
        .wrapping_add(PRIME64_4)
}

/// `XXH64` of `bytes` under `seed`.
#[must_use]
pub fn xxh64(bytes: &[u8], seed: u64) -> u64 {
    let length = bytes.len() as u64;
    let mut rest = bytes;
    let mut hash = if rest.len() >= 32 {
        let mut lanes = [
            seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2),
            seed.wrapping_add(PRIME64_2),
            seed,
            seed.wrapping_sub(PRIME64_1),
        ];
        while rest.len() >= 32 {
            for (index, lane) in lanes.iter_mut().enumerate() {
                *lane = round64(*lane, read_u64(&rest[index * 8..]));
            }
            rest = &rest[32..];
        }
        let mut hash = lanes[0]
            .rotate_left(1)
            .wrapping_add(lanes[1].rotate_left(7))
            .wrapping_add(lanes[2].rotate_left(12))
            .wrapping_add(lanes[3].rotate_left(18));
        for lane in lanes {
            hash = merge64(hash, lane);
        }
        hash
    } else {
        seed.wrapping_add(PRIME64_5)
    };
    hash = hash.wrapping_add(length);
    while rest.len() >= 8 {
        hash ^= round64(0, read_u64(rest));
        hash = hash
            .rotate_left(27)
            .wrapping_mul(PRIME64_1)
            .wrapping_add(PRIME64_4);
        rest = &rest[8..];
    }
    if rest.len() >= 4 {
        hash ^= u64::from(read_u32(rest)).wrapping_mul(PRIME64_1);
        hash = hash
            .rotate_left(23)
            .wrapping_mul(PRIME64_2)
            .wrapping_add(PRIME64_3);
        rest = &rest[4..];
    }
    for &byte in rest {
        hash ^= u64::from(byte).wrapping_mul(PRIME64_5);
        hash = hash.rotate_left(11).wrapping_mul(PRIME64_1);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(PRIME64_2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(PRIME64_3);
    hash ^ (hash >> 32)
}

const fn round32(accumulator: u32, input: u32) -> u32 {
    accumulator
        .wrapping_add(input.wrapping_mul(PRIME32_2))
        .rotate_left(13)
        .wrapping_mul(PRIME32_1)
}

/// `XXH32` of `bytes` under `seed`.
#[must_use]
pub fn xxh32(bytes: &[u8], seed: u32) -> u32 {
    // The reference adds the length modulo 2^32.
    let length = bytes.len() as u32;
    let mut rest = bytes;
    let mut hash = if rest.len() >= 16 {
        let mut lanes = [
            seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2),
            seed.wrapping_add(PRIME32_2),
            seed,
            seed.wrapping_sub(PRIME32_1),
        ];
        while rest.len() >= 16 {
            for (index, lane) in lanes.iter_mut().enumerate() {
                *lane = round32(*lane, read_u32(&rest[index * 4..]));
            }
            rest = &rest[16..];
        }
        lanes[0]
            .rotate_left(1)
            .wrapping_add(lanes[1].rotate_left(7))
            .wrapping_add(lanes[2].rotate_left(12))
            .wrapping_add(lanes[3].rotate_left(18))
    } else {
        seed.wrapping_add(PRIME32_5)
    };
    hash = hash.wrapping_add(length);
    while rest.len() >= 4 {
        hash = hash.wrapping_add(read_u32(rest).wrapping_mul(PRIME32_3));
        hash = hash.rotate_left(17).wrapping_mul(PRIME32_4);
        rest = &rest[4..];
    }
    for &byte in rest {
        hash = hash.wrapping_add(u32::from(byte).wrapping_mul(PRIME32_5));
        hash = hash.rotate_left(11).wrapping_mul(PRIME32_1);
    }
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(PRIME32_2);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(PRIME32_3);
    hash ^ (hash >> 16)
}

#[cfg(test)]
mod tests {
    use super::{xxh32, xxh64};

    /// Reference vectors, checked against the `xxhash` C library through its
    /// Python binding. The lengths walk every tail branch on both sides of
    /// each stripe threshold.
    #[test]
    fn reference_vectors_are_pinned() {
        let long: Vec<u8> = (0_u8..100).collect();
        let cases: [(&[u8], u64, u32); 12] = [
            (b"", 0xef46_db37_51d8_e999, 0x02cc_5d05),
            (b"a", 0xd24e_c4f1_a98c_6e5b, 0x550d_7456),
            (b"abc", 0x44bc_2cf5_ad77_0999, 0x32d1_53ff),
            (b"rewards", 0x20f3_bf55_0414_6bb3, 0x9292_fde6),
            (b"N3Z9EE2CXB", 0xd540_44f6_8e95_fd88, 0x7ef8_351a),
            (
                b"combat_card_generation",
                0xa2a2_1eef_4421_6b8e,
                0xf7fd_cce6,
            ),
            (&long[..15], 0xa948_f5f0_f6ab_ac2d, 0x57c0_1ece),
            (&long[..16], 0x44b6_ef2f_b841_69f7, 0xb728_37f4),
            (&long[..31], 0xc346_d2b5_9b4d_8ee1, 0xef24_f709),
            (&long[..32], 0xcbf5_9c51_16ff_32b4, 0x8307_41c1),
            (&long[..33], 0x0c53_5d1a_cafb_8ead, 0xf1e6_a545),
            (&long, 0x6ac1_e580_3216_6597, 0x7f89_ba44),
        ];
        for (bytes, expected64, expected32) in cases {
            assert_eq!(xxh64(bytes, 0), expected64, "xxh64 of {bytes:?}");
            assert_eq!(xxh32(bytes, 0), expected32, "xxh32 of {bytes:?}");
        }
        assert_eq!(xxh64(b"abc", 0x1234_5678_9abc_def0), 0x628e_181b_1c6c_4783);
        assert_eq!(xxh32(b"abc", 0x9abc_def0), 0xe544_dfde);
        assert_eq!(xxh64(&long, 0x1234_5678_9abc_def0), 0x31da_3653_6b45_1675);
        assert_eq!(xxh32(&long, 0x9abc_def0), 0xebd1_bc62);
    }
}
