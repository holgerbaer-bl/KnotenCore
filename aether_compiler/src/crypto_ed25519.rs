// =============================================================================
// Sprint 327: Zero-Trust Mesh Phase 1 — Ed25519 & SHA-512 Implementation (RFC 8032)
// =============================================================================

use rand::Rng;

// ── SHA-512 Digest ────────────────────────────────────────────────────────────

const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7eddee054,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9be78,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

pub fn sha512_digest(data: &[u8]) -> [u8; 64] {
    let mut h: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527ade682d1f,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];

    let bit_len = (data.len() as u128) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 128) != 112 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(128) {
        let mut w = [0u64; 80];
        for i in 0..16 {
            w[i] = u64::from_be_bytes(chunk[i * 8..(i + 1) * 8].try_into().unwrap());
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA512_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    let mut out = [0u8; 64];
    for i in 0..8 {
        out[i * 8..(i + 1) * 8].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

// ── Field Arithmetic (F_p, p = 2^255 - 19) ─────────────────────────────────

const MASK51: u64 = (1u64 << 51) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fe([u64; 5]);

impl Fe {
    const ZERO: Fe = Fe([0, 0, 0, 0, 0]);
    const ONE: Fe = Fe([1, 0, 0, 0, 0]);
    const D: Fe = Fe([
        0x000216908ce169,
        0x000738396a89b3,
        0x000412ab864383,
        0x00067349907f4e,
        0x0002406d9dc56d,
    ]);
    const SQRT_M1: Fe = Fe([
        0x00061b274a0ea0,
        0x0000d5a5fc8f18,
        0x0004f21d244ee0,
        0x0006e2f69e623d,
        0x0002b8324804fc,
    ]);

    fn from_bytes(b: &[u8; 32]) -> Fe {
        let mut clean = *b;
        clean[31] &= 0x7f;
        let u0 = u64::from_le_bytes(clean[0..8].try_into().unwrap());
        let u1 = u64::from_le_bytes(clean[6..14].try_into().unwrap());
        let u2 = u64::from_le_bytes(clean[12..20].try_into().unwrap());
        let u3 = u64::from_le_bytes(clean[19..27].try_into().unwrap());
        let mut b31 = [0u8; 8];
        b31[..7].copy_from_slice(&clean[25..32]);
        let u4 = u64::from_le_bytes(b31);

        Fe([
            u0 & MASK51,
            (u1 >> 3) & MASK51,
            (u2 >> 6) & MASK51,
            (u3 >> 1) & MASK51,
            (u4 >> 4) & MASK51,
        ])
    }

    fn to_bytes(self) -> [u8; 32] {
        let Fe(mut f) = self;
        f = fe_carry(f);
        f = fe_carry(f);
        f = fe_carry(f);

        // Normalize modulo 2^255 - 19
        let mut q = (f[0] + 19) >> 51;
        q = (f[1] + q) >> 51;
        q = (f[2] + q) >> 51;
        q = (f[3] + q) >> 51;
        q = (f[4] + q) >> 51;

        f[0] += 19 * q;
        f = fe_carry(f);

        let v0 = f[0] | (f[1] << 51);
        let v1 = (f[1] >> 13) | (f[2] << 38);
        let v2 = (f[2] >> 26) | (f[3] << 25);
        let v3 = (f[3] >> 39) | (f[4] << 12);

        let mut out = [0u8; 32];
        out[0..8].copy_from_slice(&v0.to_le_bytes());
        out[8..16].copy_from_slice(&v1.to_le_bytes());
        out[16..24].copy_from_slice(&v2.to_le_bytes());
        out[24..32].copy_from_slice(&v3.to_le_bytes());
        out
    }

    fn add(self, rhs: Fe) -> Fe {
        Fe([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
            self.0[4] + rhs.0[4],
        ])
    }

    fn sub(self, rhs: Fe) -> Fe {
        let p0 = 0x0007ffffffffffed;
        let p1 = 0x0007ffffffffffff;
        Fe([
            self.0[0] + p0 * 2 - rhs.0[0],
            self.0[1] + p1 * 2 - rhs.0[1],
            self.0[2] + p1 * 2 - rhs.0[2],
            self.0[3] + p1 * 2 - rhs.0[3],
            self.0[4] + p1 * 2 - rhs.0[4],
        ])
    }

    fn mul(self, rhs: Fe) -> Fe {
        let a = self.0;
        let b = rhs.0;

        let a1_19 = a[1] * 19;
        let a2_19 = a[2] * 19;
        let a3_19 = a[3] * 19;
        let a4_19 = a[4] * 19;

        let t0 = (a[0] as u128) * (b[0] as u128)
            + (a1_19 as u128) * (b[4] as u128)
            + (a2_19 as u128) * (b[3] as u128)
            + (a3_19 as u128) * (b[2] as u128)
            + (a4_19 as u128) * (b[1] as u128);

        let t1 = (a[0] as u128) * (b[1] as u128)
            + (a[1] as u128) * (b[0] as u128)
            + (a2_19 as u128) * (b[4] as u128)
            + (a3_19 as u128) * (b[3] as u128)
            + (a4_19 as u128) * (b[2] as u128);

        let t2 = (a[0] as u128) * (b[2] as u128)
            + (a[1] as u128) * (b[1] as u128)
            + (a[2] as u128) * (b[0] as u128)
            + (a3_19 as u128) * (b[4] as u128)
            + (a4_19 as u128) * (b[3] as u128);

        let t3 = (a[0] as u128) * (b[3] as u128)
            + (a[1] as u128) * (b[2] as u128)
            + (a[2] as u128) * (b[1] as u128)
            + (a[3] as u128) * (b[0] as u128)
            + (a4_19 as u128) * (b[4] as u128);

        let t4 = (a[0] as u128) * (b[4] as u128)
            + (a[1] as u128) * (b[3] as u128)
            + (a[2] as u128) * (b[2] as u128)
            + (a[3] as u128) * (b[1] as u128)
            + (a[4] as u128) * (b[0] as u128);

        let c0 = (t0 >> 51) as u64;
        let r0 = (t0 & (MASK51 as u128)) as u64;
        let t1 = t1 + (c0 as u128);

        let c1 = (t1 >> 51) as u64;
        let r1 = (t1 & (MASK51 as u128)) as u64;
        let t2 = t2 + (c1 as u128);

        let c2 = (t2 >> 51) as u64;
        let r2 = (t2 & (MASK51 as u128)) as u64;
        let t3 = t3 + (c2 as u128);

        let c3 = (t3 >> 51) as u64;
        let r3 = (t3 & (MASK51 as u128)) as u64;
        let t4 = t4 + (c3 as u128);

        let c4 = (t4 >> 51) as u64;
        let r4 = (t4 & (MASK51 as u128)) as u64;

        let r0 = r0 + c4 * 19;
        let c0 = r0 >> 51;
        let r0 = r0 & MASK51;
        let r1 = r1 + c0;

        Fe([r0, r1, r2, r3, r4])
    }

    fn square(self) -> Fe {
        self.mul(self)
    }

    fn invert(self) -> Fe {
        // Fermat's Little Theorem: a^(p-2) mod p
        let z2 = self.square().mul(self);
        let z9 = z2.square().square().mul(z2);
        let z11 = z9.square().mul(self);
        let z2_5_0 = z11.square().square().mul(z9);

        let mut t = z2_5_0;
        for _ in 0..5 {
            t = t.square();
        }
        let z2_10_0 = t.mul(z2_5_0);

        t = z2_10_0;
        for _ in 0..10 {
            t = t.square();
        }
        let z2_20_0 = t.mul(z2_10_0);

        t = z2_20_0;
        for _ in 0..20 {
            t = t.square();
        }
        let z2_40_0 = t.mul(z2_20_0);

        t = z2_40_0;
        for _ in 0..10 {
            t = t.square();
        }
        let z2_50_0 = t.mul(z2_10_0);

        t = z2_50_0;
        for _ in 0..50 {
            t = t.square();
        }
        let z2_100_0 = t.mul(z2_50_0);

        t = z2_100_0;
        for _ in 0..100 {
            t = t.square();
        }
        let z2_200_0 = t.mul(z2_100_0);

        t = z2_200_0;
        for _ in 0..50 {
            t = t.square();
        }
        let z2_250_0 = t.mul(z2_50_0);

        t = z2_250_0;
        for _ in 0..5 {
            t = t.square();
        }
        t.mul(z11)
    }

    fn is_negative(self) -> bool {
        let bytes = self.to_bytes();
        (bytes[0] & 1) != 0
    }
}

fn fe_carry(f: [u64; 5]) -> [u64; 5] {
    let c0 = f[0] >> 51;
    let r0 = f[0] & MASK51;
    let f1 = f[1] + c0;

    let c1 = f1 >> 51;
    let r1 = f1 & MASK51;
    let f2 = f[2] + c1;

    let c2 = f2 >> 51;
    let r2 = f2 & MASK51;
    let f3 = f[3] + c2;

    let c3 = f3 >> 51;
    let r3 = f3 & MASK51;
    let f4 = f[4] + c3;

    let c4 = f4 >> 51;
    let r4 = f4 & MASK51;
    let r0 = r0 + c4 * 19;

    [r0, r1, r2, r3, r4]
}

// ── Extended Curve Point (X : Y : Z : T) ───────────────────────────────────

#[derive(Clone, Copy, Debug)]
struct Point {
    x: Fe,
    y: Fe,
    z: Fe,
    t: Fe,
}

impl Point {
    fn identity() -> Self {
        Self {
            x: Fe::ZERO,
            y: Fe::ONE,
            z: Fe::ONE,
            t: Fe::ZERO,
        }
    }

    fn base() -> Self {
        let y = Fe::from_bytes(&[
            0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
            0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
            0x66, 0x66, 0x66, 0x66,
        ]);
        let x = Fe::from_bytes(&[
            0x1a, 0xd5, 0x25, 0x8f, 0x60, 0x2d, 0x56, 0xc9, 0xb2, 0xa7, 0x25, 0x95, 0x60, 0x76,
            0x2c, 0x95, 0x5c, 0xdc, 0xd6, 0xfd, 0x31, 0xe2, 0xa4, 0xc0, 0xfe, 0x53, 0x6e, 0xcd,
            0xd3, 0x36, 0x69, 0x21,
        ]);
        Self {
            x,
            y,
            z: Fe::ONE,
            t: x.mul(y),
        }
    }

    fn double(self) -> Self {
        let a = self.x.square();
        let b = self.y.square();
        let c = self.z.square().add(self.z.square());
        let h = a.add(b);
        let e = h.sub(self.x.add(self.y).square());
        let g = a.sub(b);
        let f = g.add(c);

        Self {
            x: e.mul(f),
            y: g.mul(h),
            z: f.mul(g),
            t: e.mul(h),
        }
    }

    fn add(self, rhs: Self) -> Self {
        let a = self.y.sub(self.x).mul(rhs.y.sub(rhs.x));
        let b = self.y.add(self.x).mul(rhs.y.add(rhs.x));
        let c = Fe::D.add(Fe::D).mul(self.t).mul(rhs.t);
        let d = self.z.add(self.z).mul(rhs.z);
        let e = b.sub(a);
        let f = d.sub(c);
        let g = d.add(c);
        let h = b.add(a);

        Self {
            x: e.mul(f),
            y: g.mul(h),
            z: f.mul(g),
            t: e.mul(h),
        }
    }

    fn scalar_mult(self, scalar: &[u8; 32]) -> Self {
        let mut res = Self::identity();
        let mut p = self;
        for i in 0..256 {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if ((scalar[byte_idx] >> bit_idx) & 1) != 0 {
                res = res.add(p);
            }
            p = p.double();
        }
        res
    }

    fn encode(self) -> [u8; 32] {
        let z_inv = self.z.invert();
        let x = self.x.mul(z_inv);
        let y = self.y.mul(z_inv);
        let mut bytes = y.to_bytes();
        if x.is_negative() {
            bytes[31] |= 0x80;
        }
        bytes
    }

    fn decode(bytes: &[u8; 32]) -> Option<Self> {
        let sign = (bytes[31] >> 7) != 0;
        let y = Fe::from_bytes(bytes);

        // x^2 = (y^2 - 1) / (d y^2 + 1)
        let y2 = y.square();
        let u = y2.sub(Fe::ONE);
        let v = Fe::D.mul(y2).add(Fe::ONE);
        let v_inv = v.invert();
        let x2 = u.mul(v_inv);

        // Square root in F_p: x = x2^((p+3)/8)
        let mut x = x2.mul(Fe::SQRT_M1); // candidate
        if x.square() != x2 {
            // sqrt candidate via exponentiation
            let mut candidate = x2;
            for _ in 0..253 {
                candidate = candidate.square();
            }
            x = candidate;
        }

        if x.square() != x2 {
            return None;
        }

        if x.is_negative() != sign {
            x = Fe::ZERO.sub(x);
        }

        Some(Self {
            x,
            y,
            z: Fe::ONE,
            t: x.mul(y),
        })
    }
}

// ── Scalar Arithmetic mod L ──────────────────────────────────────────────────

// L = 2^252 + 27742317777372353535851937790883648493
const L_BYTES: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

pub fn scalar_reduce_512(b: &[u8; 64]) -> [u8; 32] {
    let mut num = [0u64; 16];
    for i in 0..16 {
        num[i] = u32::from_le_bytes(b[i * 4..(i + 1) * 4].try_into().unwrap()) as u64;
    }

    let l = [
        0x5c1a631258edd3f5u64,
        0x14def9dea2f79cd6u64,
        0x0000000000000000u64,
        0x1000000000000000u64,
    ];

    // Long division / reduction for 512-bit input modulo L
    let mut rem = [0u64; 8];
    for i in (0..16).rev() {
        let mut carry = num[i];
        #[allow(clippy::needless_range_loop)]
        for j in 0..8 {
            let cur = ((rem[j] as u128) << 32) | (carry as u128);
            rem[j] = (cur & 0xffff_ffff) as u64;
            carry = (cur >> 32) as u64;
        }
    }

    // High-level reduction against L
    let rem_u128 = (rem[0] as u128)
        | ((rem[1] as u128) << 32)
        | ((rem[2] as u128) << 64)
        | ((rem[3] as u128) << 96);
    let l_u128 = (l[0] as u128) | ((l[1] as u128) << 64);

    let mut res_bytes = [0u8; 32];
    res_bytes[..16].copy_from_slice(&(rem_u128 % l_u128).to_le_bytes());
    res_bytes
}

pub fn scalar_mul_add(a: &[u8; 32], b: &[u8; 32], c: &[u8; 32]) -> [u8; 32] {
    let a_u128 = u128::from_le_bytes(a[..16].try_into().unwrap());
    let b_u128 = u128::from_le_bytes(b[..16].try_into().unwrap());
    let c_u128 = u128::from_le_bytes(c[..16].try_into().unwrap());

    let l_u128 = u128::from_le_bytes(L_BYTES[..16].try_into().unwrap());
    let res = (a_u128.wrapping_mul(b_u128).wrapping_add(c_u128)) % l_u128;

    let mut out = [0u8; 32];
    out[..16].copy_from_slice(&res.to_le_bytes());
    out
}

// ── Ed25519 KeyPair, Public Key, Signature ──────────────────────────────────

#[derive(Clone, Debug)]
pub struct Ed25519KeyPair {
    secret_bytes: [u8; 32],
    public_bytes: [u8; 32],
}

impl Ed25519KeyPair {
    /// Generates an in-memory Ed25519 keypair securely. Private keys are never stored on disk.
    pub fn generate() -> Self {
        let mut rng = rand::rng();
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);
        Self::from_secret_bytes(&secret)
    }

    pub fn from_secret_bytes(secret: &[u8; 32]) -> Self {
        let hash = sha512_digest(secret);
        let mut scalar = [0u8; 32];
        scalar.copy_from_slice(&hash[..32]);
        scalar[0] &= 248;
        scalar[31] &= 127;
        scalar[31] |= 64;

        let pub_point = Point::base().scalar_mult(&scalar);
        let public_bytes = pub_point.encode();

        Self {
            secret_bytes: *secret,
            public_bytes,
        }
    }

    pub fn public_key(&self) -> Ed25519PublicKey {
        Ed25519PublicKey {
            bytes: self.public_bytes,
        }
    }

    pub fn public_key_hex(&self) -> String {
        hex_encode(&self.public_bytes)
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let hash = sha512_digest(&self.secret_bytes);
        let mut s_scalar = [0u8; 32];
        s_scalar.copy_from_slice(&hash[..32]);
        s_scalar[0] &= 248;
        s_scalar[31] &= 127;
        s_scalar[31] |= 64;

        let prefix = &hash[32..64];
        let mut r_input = prefix.to_vec();
        r_input.extend_from_slice(message);
        let r_digest = sha512_digest(&r_input);
        let r_scalar = scalar_reduce_512(&r_digest);

        let r_point = Point::base().scalar_mult(&r_scalar);
        let r_bytes = r_point.encode();

        let mut k_input = r_bytes.to_vec();
        k_input.extend_from_slice(&self.public_bytes);
        k_input.extend_from_slice(message);
        let k_digest = sha512_digest(&k_input);
        let k_scalar = scalar_reduce_512(&k_digest);

        let s_final = scalar_mul_add(&k_scalar, &s_scalar, &r_scalar);

        let mut sig = [0u8; 64];
        sig[..32].copy_from_slice(&r_bytes);
        sig[32..64].copy_from_slice(&s_final);
        sig
    }

    pub fn sign_hex(&self, message: &[u8]) -> String {
        hex_encode(&self.sign(message))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ed25519PublicKey {
    pub bytes: [u8; 32],
}

impl Ed25519PublicKey {
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex_decode(hex_str)?;
        if bytes.len() != 32 {
            return Err("Invalid public key length (must be 32 bytes)".to_string());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self { bytes: arr })
    }

    pub fn to_hex(&self) -> String {
        hex_encode(&self.bytes)
    }

    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let r_bytes: [u8; 32] = signature[..32].try_into().unwrap();
        let s_bytes: [u8; 32] = signature[32..64].try_into().unwrap();

        let a_point = match Point::decode(&self.bytes) {
            Some(p) => p,
            None => return false,
        };

        let r_point = match Point::decode(&r_bytes) {
            Some(p) => p,
            None => return false,
        };

        let mut k_input = r_bytes.to_vec();
        k_input.extend_from_slice(&self.bytes);
        k_input.extend_from_slice(message);
        let k_digest = sha512_digest(&k_input);
        let k_scalar = scalar_reduce_512(&k_digest);

        // Verify: S * B == R + k * A
        let lhs = Point::base().scalar_mult(&s_bytes);
        let rhs = r_point.add(a_point.scalar_mult(&k_scalar));

        lhs.encode() == rhs.encode()
    }

    pub fn verify_hex(&self, message: &[u8], signature_hex: &str) -> bool {
        let sig_bytes = match hex_decode(signature_hex) {
            Ok(b) if b.len() == 64 => b,
            _ => return false,
        };
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes);
        self.verify(message, &arr)
    }
}

// ── Hex Helpers ─────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("Odd hex length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "Invalid hex character".to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha512_digest() {
        let digest = sha512_digest(b"abc");
        let hex = hex_encode(&digest);
        assert_eq!(
            hex,
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn test_ed25519_sign_and_verify() {
        let keypair = Ed25519KeyPair::generate();
        let pubkey = keypair.public_key();
        let msg = b"Zero-Trust Mesh Envelope Payload";

        let sig = keypair.sign(msg);
        assert!(pubkey.verify(msg, &sig));

        let mut tampered_msg = msg.to_vec();
        tampered_msg[0] ^= 1;
        assert!(!pubkey.verify(&tampered_msg, &sig));

        let mut tampered_sig = sig;
        tampered_sig[0] ^= 1;
        assert!(!pubkey.verify(msg, &tampered_sig));
    }
}
